// src/ble.rs
//
// BLE connectivity module for Olympus.
// Handles trainer/sensor connection via btleplug.
//
// Cross-platform: btleplug abstracts BlueZ (Linux), CoreBluetooth (macOS/iOS),
// WinRT (Windows 10+) and DroidPlug (Android). We parse the standard GATT
// profiles for Cycling Power, Heart Rate and the Fitness Machine Service so a
// Tacx Flux S2 (FE-C over BLE) is driven end-to-end.

use btleplug::api::{
    Central, CharPropFlags, Characteristic, Manager as _, Peripheral as _, ScanFilter,
    ValueNotification, WriteType, bleuuid::uuid_from_u16,
};
use btleplug::platform::{Manager, Peripheral};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time;
use uuid::Uuid;

// Standard GATT service / characteristic UUIDs relevant to cycling trainers.
const CPS_UUID: Uuid = uuid_from_u16(0x1818); // Cycling Power Service
const PM_UUID: Uuid = uuid_from_u16(0x2A63); // Cycling Power Measurement
const FMS_UUID: Uuid = uuid_from_u16(0x1826); // Fitness Machine Service
const HRS_UUID: Uuid = uuid_from_u16(0x180D); // Heart Rate Service
const HRM_UUID: Uuid = uuid_from_u16(0x2A37); // Heart Rate Measurement (characteristic)
const FMCP_UUID: Uuid = uuid_from_u16(0x2AD9); // Fitness Machine Control Point
const _CSC_UUID: Uuid = uuid_from_u16(0x1816); // Cycling Speed & Cadence
const CSC_MEASUREMENT_UUID: Uuid = uuid_from_u16(0x2A5B); // CSC Measurement

/// Tracks cumulative crank revolutions + last event time across notifications
/// so cadence (rpm) can be derived from the delta between two samples, using
/// the GATT convention of 1/1024 s per crank-event time unit.
///
/// Both the Cycling Power Measurement and CSC Measurement profiles report a
/// cumulative crank revolution count and a "last crank event time". Cadence:
///     rpm = (Δrevs * 1024 * 60) / Δevent_time
struct CrankTracker {
    revs: AtomicU32,
    event_time: AtomicU16,
    initialized: AtomicBool,
}

impl CrankTracker {
    fn new() -> Self {
        Self {
            revs: AtomicU32::new(0),
            event_time: AtomicU16::new(0),
            initialized: AtomicBool::new(false),
        }
    }

    /// Feed a new (revs, event_time) sample; returns the derived cadence in
    /// rpm, or `None` until a second sample is available (or on a bogus delta).
    fn cadence(&self, revs: u32, event_time: u16) -> Option<u16> {
        let prev_revs = self.revs.load(Ordering::Relaxed) as u32;
        let prev_time = self.event_time.load(Ordering::Relaxed);
        let init = self.initialized.load(Ordering::Relaxed);

        self.revs.store(revs, Ordering::Relaxed);
        self.event_time.store(event_time, Ordering::Relaxed);
        self.initialized.store(true, Ordering::Relaxed);

        if !init {
            return None;
        }
        // Handle 16-bit event-time wraparound.
        let dt = event_time.wrapping_sub(prev_time) as u16 as u32;
        // Revolutions may wrap the counter; handle using i32 subtraction.
        let drevs = revs.wrapping_sub(prev_revs) as i32;
        if drevs < 0 || dt == 0 {
            return None;
        }
        let rpm = (drevs as u32 * 1024 * 60) / dt;
        Some(rpm.min(250) as u16)
    }
}

/// A single telemetry sample emitted from the BLE driver.
#[derive(Debug, Clone, PartialEq)]
pub struct Telemetry {
    pub power: Option<u16>,
    pub cadence: Option<u16>,
    pub heart_rate: Option<u16>,
    pub speed: Option<f32>,
}

/// Commands the UI can send back to the BLE driver.
#[derive(Debug, Clone, PartialEq)]
pub enum BleCommand {
    /// Set trainer target power / ERG level in watts.
    SetTargetPower(u16),
    /// Ask the driver to (re)scan and connect to the configured trainer.
    Scan,
    /// Disconnect cleanly.
    Disconnect,
}

/// Connection state, surfaced to the UI for the System panel. A single state
/// drives both the FTMS trainer and the optional HR strap: `Connected` carries
/// the display name of each device that is currently paired (`None` = not present).
#[derive(Debug, Clone, PartialEq)]
pub enum BleState {
    Idle,
    Scanning,
    Connecting,
    Connected {
        trainer: Option<String>,
        strap: Option<String>,
    },
    Error(String),
}

/// A boxed notification stream, so the driver loop can treat a missing trainer
/// or strap slot as a stream that simply never yields.
type NotificationStream = Pin<Box<dyn Stream<Item = ValueNotification> + Send>>;

/// What a discovered peripheral can be used for. A trainer also often includes
/// a Heart Rate service (so it can relay a chest strap), so FMS/CPS wins over
/// HRM when both are advertised; a device advertising only HRM is a strap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SensorRole {
    Trainer,
    Strap,
}

/// Classify a peripheral by its advertised GATT services (+ name heuristics for
/// devices that advertise no services). Returns `None` for devices that aren't
/// a trainer or an HR strap we care about.
fn classify(services: &[Uuid], name: Option<&str>) -> Option<SensorRole> {
    let has_cps = services.contains(&CPS_UUID);
    let has_fms = services.contains(&FMS_UUID);
    let has_hr = services.contains(&HRS_UUID);

    if has_cps || has_fms {
        return Some(SensorRole::Trainer);
    }
    if has_hr {
        return Some(SensorRole::Strap);
    }

    match name {
        Some(n) if n.contains("Tacx") || n.contains("Flux") || n.contains("TRAINER") => {
            Some(SensorRole::Trainer)
        }
        Some(n) if n.contains("Polar") || n.contains("H10") || n.contains("Garmin") => {
            Some(SensorRole::Strap)
        }
        _ => None,
    }
}

/// Result of running the BLE driver: a task handle plus channels.
#[allow(dead_code)]
pub struct BleDriver {
    /// Handle to the spawned async task that owns the BLE stack.
    pub task: tokio::task::JoinHandle<()>,
    /// Incoming telemetry samples.
    pub rx: Receiver<Telemetry>,
    /// Send commands (target power, scan, disconnect) to the driver.
    pub cmd_tx: Sender<BleCommand>,
    /// Endpoint for the driver to report its connection state.
    pub state_rx: Receiver<BleState>,
    pub state_tx: Sender<BleState>,
}

/// Starts the BLE driver task. Returns a handle plus live channels.
pub fn start_driver() -> BleDriver {
    let (tel_tx, tel_rx) = mpsc::channel(256);
    let (cmd_tx, cmd_rx) = mpsc::channel(32);
    let (state_tx, state_rx) = mpsc::channel(16);

    let task = tokio::spawn(driver_loop(tel_tx, cmd_rx, state_tx.clone()));

    BleDriver {
        task,
        rx: tel_rx,
        cmd_tx,
        state_rx,
        state_tx,
    }
}

/// Core async BLE event loop. Owns both connection slots (the FTMS trainer and
/// the optional HR strap) and drives their notification streams with a single
/// command channel. When either slot drops, the whole connection is torn down
/// and the scan restarts so a lost strap or trainer reconnects cleanly.
async fn driver_loop(
    tel_tx: Sender<Telemetry>,
    mut cmd_rx: Receiver<BleCommand>,
    state_tx: Sender<BleState>,
) {
    let state_send = |s: BleState| {
        let _ = state_tx.try_send(s.clone());
    };

    state_send(BleState::Idle);

    // Reconnect loop: scan, connect the trainer + (optional) strap, then drive
    // both until either disconnects, then start over.
    loop {
        state_send(BleState::Scanning);
        let (trainer, strap) = match find_sensors().await {
            Ok(found) => found,
            Err(e) => {
                state_send(BleState::Error(format!("BLE error: {e}")));
                time::sleep(Duration::from_secs(2)).await;
                state_send(BleState::Scanning);
                continue;
            }
        };

        if trainer.is_none() && strap.is_none() {
            // Nothing to read — report idle and rescan shortly. Olympus never
            // emits made-up telemetry; without sensors the UI shows an idle
            // panel instead of fake numbers.
            state_send(BleState::Idle);
            time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        state_send(BleState::Connecting);

        // ---- Trainer slot -------------------------------------------------
        let mut trainer_p = trainer;
        let mut trainer_subscriptions: Vec<Characteristic> = Vec::new();
        let mut trainer_name: Option<String> = None;
        if let Some(p) = trainer_p.as_mut() {
            trainer_name = p
                .properties()
                .await
                .ok()
                .flatten()
                .and_then(|pr| pr.local_name);
            match p.discover_services().await {
                Ok(_) => trainer_subscriptions = subscribe_all(p, &tel_tx).await,
                Err(e) => state_send(BleState::Error(format!("service discovery failed: {e}"))),
            }
        }

        // ---- HR strap slot -------------------------------------------------
        let mut strap_p = strap;
        let mut strap_name: Option<String> = None;
        if let Some(p) = strap_p.as_mut() {
            strap_name = p
                .properties()
                .await
                .ok()
                .flatten()
                .and_then(|pr| pr.local_name);
            match p.discover_services().await {
                Ok(_) => {
                    let _ = subscribe_all(p, &tel_tx).await;
                }
                Err(e) => log::warn!("strap service discovery failed: {e}"),
            }
        }

        state_send(BleState::Connected {
            trainer: trainer_name.clone(),
            strap: strap_name.clone(),
        });

        // Notification streams (a `pending` filler for a slot with no device).
        let mut trainer_stream: NotificationStream = match trainer_p.as_mut() {
            Some(p) => match p.notifications().await {
                Ok(s) => s.boxed(),
                Err(e) => {
                    state_send(BleState::Error(format!(
                        "trainer notifications unavailable: {e}"
                    )));
                    return;
                }
            },
            None => futures::stream::pending().boxed(),
        };
        let mut strap_stream: NotificationStream = match strap_p.as_mut() {
            Some(p) => match p.notifications().await {
                Ok(s) => s.boxed(),
                Err(e) => {
                    state_send(BleState::Error(format!(
                        "strap notifications unavailable: {e}"
                    )));
                    return;
                }
            },
            None => futures::stream::pending().boxed(),
        };

        // Shared crank state so cadence can be derived across notifications.
        let crank = CrankTracker::new();

        // The last target power actually commanded to the trainer, so we only
        // ramp when the ERG target *changes* (the UI re-sends the target every
        // second, and re-ramping a static target would never converge).
        let mut last_written: Option<u16> = None;

        // While a strap is feeding HR readings it is authoritative; if it drops
        // we let the trainer's own HR (rare) take over again.
        let mut strap_hr_active: bool = strap_p.is_some();

        loop {
            tokio::select! {
                // Incoming data notifications from the trainer.
                maybe = trainer_stream.next() => {
                    match maybe {
                        Some(n) => {
                            handle_notification(&n, &tel_tx, &crank, strap_hr_active).await
                        }
                        None => {
                            state_send(BleState::Idle);
                            break;
                        }
                    }
                }
                // Incoming HR notifications from the strap.
                maybe = strap_stream.next() => {
                    match maybe {
                        Some(n) => {
                            if n.uuid == HRM_UUID && let Some(hr) = parse_hr(&n.value) {
                                strap_hr_active = true;
                                let _ = tel_tx
                                    .send(Telemetry {
                                        power: None,
                                        cadence: None,
                                        heart_rate: Some(hr),
                                        speed: None,
                                    })
                                    .await;
                            }
                        }
                        None => {
                            // Dropped strap: forget the stale reading and let
                            // trainer-side HR through again (fallback).
                            strap_hr_active = false;
                            strap_stream = futures::stream::pending().boxed();
                            state_send(BleState::Connected {
                                trainer: trainer_name.clone(),
                                strap: None,
                            });
                        }
                    }
                }
                // Commands from the UI.
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(BleCommand::SetTargetPower(w)) => {
                            // Without a trainer there is nothing to command.
                            let Some(p) = trainer_p.as_ref() else {
                                continue;
                            };
                            // Dedupe: if the target is unchanged, the previous
                            // ramp (if any) already delivered it.
                            if last_written == Some(w) {
                                continue;
                            }
                            match ramp_target(p, &trainer_subscriptions, last_written, w).await {
                                Ok(()) => last_written = Some(w),
                                Err(e) => state_send(BleState::Error(format!("ERG failed: {e}"))),
                            }
                        }
                        Some(BleCommand::Disconnect) | Some(BleCommand::Scan) => {
                            if let Some(p) = trainer_p.as_ref() {
                                let _ = p.disconnect().await;
                            }
                            if let Some(p) = strap_p.as_ref() {
                                let _ = p.disconnect().await;
                            }
                            state_send(BleState::Idle);
                            break;
                        }
                        None => break,
                    }
                }
                // Heartbeat so we can report a healthy "connected" state
                // periodically with both device names.
                _ = time::sleep(Duration::from_secs(5)) => {
                    state_send(BleState::Connected {
                        trainer: trainer_name.clone(),
                        strap: strap_name.clone(),
                    });
                }
            }
        }
    }
}

/// Discovery flow: find the first adapter, scan, and return the FTMS trainer
/// (if any) plus an HR strap (if any), connected and ready for subscription.
/// Returns `Ok((None, None))` when nothing useful was found so the caller can
/// retry the scan later.
async fn find_sensors() -> Result<(Option<Peripheral>, Option<Peripheral>), String> {
    let Ok(manager) = Manager::new().await else {
        return Err("failed to create BLE manager".into());
    };

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("no adapters: {e}"))?;
    let Some(central) = adapters.into_iter().next() else {
        return Ok((None, None));
    };

    // Start scanning. Heavily filter to only cycling/fitness devices so we don't
    // churn through stray BLE peripherals.
    central
        .start_scan(ScanFilter {
            services: vec![CPS_UUID, FMS_UUID, HRS_UUID],
        })
        .await
        .map_err(|e| format!("failed to start scan: {e}"))?;

    time::sleep(Duration::from_secs(2)).await;

    let peripherals = central
        .peripherals()
        .await
        .map_err(|e| format!("failed to list peripherals: {e}"))?;

    // Group by role: the first FTMS/CPS device is the trainer, the first device
    // advertising only Heart Rate is the strap.
    let mut trainer: Option<Peripheral> = None;
    let mut strap: Option<Peripheral> = None;
    for p in peripherals.into_iter() {
        if let Ok(Some(props)) = p.properties().await {
            match classify(&props.services, props.local_name.as_deref()) {
                Some(SensorRole::Trainer) if trainer.is_none() => trainer = Some(p.clone()),
                Some(SensorRole::Strap) if strap.is_none() => strap = Some(p.clone()),
                _ => {}
            }
            if trainer.is_some() && strap.is_some() {
                break;
            }
        }
    }

    central.stop_scan().await.ok();

    // A missing trainer is fine (this may be a strap-only setup); a missing
    // strap is fine too (trainer-only — the common case). Only a trainer that
    // fails to *connect* is an error; a dead strap just logs and is dropped.
    if let Some(t) = trainer.as_ref() {
        t.connect()
            .await
            .map_err(|e| format!("connect failed: {e}"))?;
    }
    if let Some(s) = strap.clone() && let Err(e) = s.connect().await {
        log::warn!("strap connect failed, dropping it: {e}");
        strap = None;
    }

    Ok((trainer, strap))
}

/// Discover characteristics after connecting and subscribe to all the ones we
/// care about. Returns the list of subscribed characteristics so we can write
/// to the Fitness Machine Control Point later.
async fn subscribe_all(
    peripheral: &mut Peripheral,
    tel_tx: &Sender<Telemetry>,
) -> Vec<Characteristic> {
    let mut subs = Vec::new();

    let chars = peripheral.characteristics();

    for c in chars {
        let is_subscribable = c.properties.contains(CharPropFlags::NOTIFY)
            || c.properties.contains(CharPropFlags::INDICATE);

        if !is_subscribable {
            continue;
        }

        // Only subscribe to the measurements we care about.
        let wanted = c.uuid == PM_UUID
            || c.uuid == HRM_UUID
            || c.uuid == CSC_MEASUREMENT_UUID
            || c.uuid == FMCP_UUID;

        if !wanted {
            continue;
        }

        if let Err(e) = peripheral.subscribe(&c).await {
            log::warn!("failed to subscribe to {c:?}: {e}");
            continue;
        }

        subs.push(c);
    }

    // Send an initial telemetry sample so the UI has *something* immediately.
    let _ = tel_tx
        .send(Telemetry {
            power: None,
            cadence: None,
            heart_rate: None,
            speed: None,
        })
        .await;

    subs
}

/// Parse an HR Measurement notification (0x2A37). `None` when the payload is
/// malformed. Byte 0 is a flags byte; bit 0 requests a u16 LE heart rate,
/// otherwise the value is a single u8.
fn parse_hr(value: &[u8]) -> Option<u16> {
    if value.is_empty() {
        return None;
    }
    let flags = value[0];
    if flags & 0x01 != 0 {
        if value.len() >= 3 {
            Some(u16::from_le_bytes([value[1], value[2]]))
        } else {
            None
        }
    } else if value.len() >= 2 {
        Some(value[1] as u16)
    } else {
        None
    }
}

/// Parse a single trainer notification into a partial `Telemetry` (fields not
/// present in the payload stay `None`). Pure and sync so the HR-merge rules
/// below are unit-testable without a live BLE stack.
///
/// `suppress_trainer_hr` is set while an HR strap is live: trainers sometimes
/// relay a chest strap, and we never let a trainer-side reading override the
/// strap, whose notifications arrive on the strap slot instead.
fn parse_notification(
    uuid: Uuid,
    value: &[u8],
    crank: &CrankTracker,
    suppress_trainer_hr: bool,
) -> Telemetry {
    let mut t = Telemetry {
        power: None,
        cadence: None,
        heart_rate: None,
        speed: None,
    };

    if uuid == PM_UUID {
        // Cycling Power Measurement (0x2A63):
        //   Flags u16 (LE), then Instantaneous Power u16 (always present),
        //   then optional fields gated by flag bits:
        //     bit4 (0x0010): wheel revs u32 + last wheel time u16
        //     bit5 (0x0020): crank revs u16 + last crank time u16
        let b = value;
        if b.len() >= 4 {
            let flags = u16::from_le_bytes([b[0], b[1]]);
            let power = u16::from_le_bytes([b[2], b[3]]);
            t.power = Some(power);

            let mut offset = 4;
            if flags & 0x0010 != 0 {
                offset += 6; // wheel revs (u32) + last wheel time (u16)
            }
            if flags & 0x0020 != 0 && b.len() >= offset + 4 {
                let revs =
                    u32::from_le_bytes([b[offset], b[offset + 1], b[offset + 2], b[offset + 3]]);
                let event_time = u16::from_le_bytes([b[offset + 4], b[offset + 5]]);
                t.cadence = crank.cadence(revs, event_time);
            }
        }
    } else if uuid == HRM_UUID {
        // HR Measurement (0x2A37): a strap owns HR while it's connected, so a
        // trainer-relayed reading is only used when no strap is feeding us.
        if !suppress_trainer_hr && let Some(hr) = parse_hr(value) {
            t.heart_rate = Some(hr);
        }
    } else if uuid == CSC_MEASUREMENT_UUID {
        // CSC Measurement: flags byte, then optional fields. Flags bit 0 =
        // wheel data (cumulative wheel revs u32 + last wheel event time u16),
        // bit 1 = crank data (cumulative crank revs u16 + last crank event time u16).
        let b = value;
        if !b.is_empty() {
            let flags = b[0];
            let mut idx = 1;
            if flags & 0x01 != 0 {
                idx += 6; // skip wheel fields (u32 + u16)
            }
            if flags & 0x02 != 0 && b.len() >= idx + 4 {
                let revs = u32::from_le_bytes([b[idx], b[idx + 1], 0, 0]);
                let event_time = u16::from_le_bytes([b[idx + 2], b[idx + 3]]);
                t.cadence = crank.cadence(revs, event_time);
            }
        }
    }

    t
}

/// Parse a single trainer notification and forward the parsed values to the UI.
async fn handle_notification(
    n: &ValueNotification,
    tel_tx: &Sender<Telemetry>,
    crank: &CrankTracker,
    suppress_trainer_hr: bool,
) {
    let t = parse_notification(n.uuid, &n.value, crank, suppress_trainer_hr);
    let _ = tel_tx.send(t).await;
}

/// Write an ERG target power to the trainer via the Fitness Machine Control
/// Point. The FTMS request for setting target power is:
///   [0x03] [power W u16 LE] [0x01]
/// with 0x03 = "Set Target Power" opcode and 0x01 = power zone encouragement.
async fn set_target_power(
    peripheral: &Peripheral,
    subscriptions: &[Characteristic],
    watts: u16,
) -> Result<(), String> {
    let ctl_point = subscriptions
        .iter()
        .find(|c| c.uuid == FMCP_UUID)
        .ok_or_else(|| String::from("Fitness Machine Control Point not subscribed"))?;

    let payload: Vec<u8> = vec![0x03, (watts & 0xFF) as u8, (watts >> 8) as u8, 0x01];
    peripheral
        .write(ctl_point, &payload, WriteType::WithResponse)
        .await
        .map_err(|e| format!("FTMS write failed: {e}"))
}

/// A linear ERG ramp from `from` to `to`, split into `n` evenly-spaced
/// intermediate commands with the final `to` included last. Returns just
/// `[to]` when the values are already equal.
fn ramp_steps(from: u16, to: u16, n: u32) -> Vec<u16> {
    if from == to || n == 0 {
        return vec![to];
    }
    let delta = (to as f32 - from as f32) / n as f32;
    (1..=n)
        .map(|i| (from as f32 + delta * i as f32).round().clamp(0.0, u16::MAX as f32) as u16)
        .collect()
}

/// Command the target gradually so a Flux-style trainer doesn't jolt toward a
/// big wattage jump. On the first-ever command (`from` is `None`) the target is
/// written directly; afterwards we step over ~2.5 s in 100 ms increments.
async fn ramp_target(
    peripheral: &Peripheral,
    subscriptions: &[Characteristic],
    from: Option<u16>,
    to: u16,
) -> Result<(), String> {
    let Some(start) = from else {
        return set_target_power(peripheral, subscriptions, to).await;
    };

    // ~2.5 s ramp at 100 ms per step.
    for w in ramp_steps(start, to, 25) {
        set_target_power(peripheral, subscriptions, w).await?;
        time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_defaults_are_none() {
        let t = Telemetry {
            power: None,
            cadence: None,
            heart_rate: None,
            speed: None,
        };
        assert_eq!(t.power, None);
        assert_eq!(t.speed, None);
    }

    #[test]
    fn ble_state_matches() {
        let a = BleState::Connected {
            trainer: Some("Tacx Flux S2".into()),
            strap: Some("Polar H10".into()),
        };
        let b = BleState::Connected {
            trainer: Some("Tacx Flux S2".into()),
            strap: Some("Polar H10".into()),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn classify_groups_peripherals_by_role() {
        // FTMS trainer, with or without a relayed HR service, is a trainer.
        assert_eq!(
            classify(&[FMS_UUID], None),
            Some(SensorRole::Trainer)
        );
        assert_eq!(
            classify(&[FMS_UUID, HRS_UUID], None),
            Some(SensorRole::Trainer)
        );
        assert_eq!(
            classify(&[CPS_UUID], None),
            Some(SensorRole::Trainer)
        );
        // A device advertising only the Heart Rate Service is a strap.
        assert_eq!(
            classify(&[HRS_UUID], None),
            Some(SensorRole::Strap)
        );
        // Name heuristics kick in when no services advertise.
        assert_eq!(
            classify(&[], Some("Tacx Flux S2")),
            Some(SensorRole::Trainer)
        );
        assert_eq!(
            classify(&[], Some("Polar H10 8B1D")),
            Some(SensorRole::Strap)
        );
        // Unrelated devices are ignored.
        assert_eq!(classify(&[], Some("Wireless Mouse")), None);
        assert_eq!(classify(&[], None), None);
    }

    #[test]
    fn parse_hr_handles_u8_and_u16() {
        // u8 heart rate, flags = 0x00.
        assert_eq!(parse_hr(&[0x00, 135]), Some(135));
        // u16 little-endian, flags bit 0 set.
        assert_eq!(parse_hr(&[0x01, 0x80, 0x00]), Some(128));
        assert_eq!(parse_hr(&[0x01, 0x4D, 0x01]), Some(333));
        // Malformed payloads are rejected.
        assert_eq!(parse_hr(&[]), None);
        assert_eq!(parse_hr(&[0x01, 0x4D]), None);
        assert_eq!(parse_hr(&[0x00]), None);
    }

    #[test]
    fn trainer_hr_passes_while_strap_down_but_not_while_live() {
        let crank = CrankTracker::new();
        let hrm = vec![0x00, 135]; // u8 HR, 135 bpm

        // No strap paired: the trainer-relayed HR is forwarded.
        let t = parse_notification(HRM_UUID, &hrm, &crank, false);
        assert_eq!(t.heart_rate, Some(135));

        // Strap live: the same trainer notification must NOT override it.
        let t = parse_notification(HRM_UUID, &hrm, &crank, true);
        assert_eq!(t.heart_rate, None, "strap owns HR while connected");
    }

    #[test]
    fn strap_hr_wins_over_trainer_hr_end_to_end() {
        // Simulate the driver's phone-in for both slots: a trainer power
        // packet and a strap HR packet. The merged LiveData keeps the last
        // value of every field, so power survives the HR-only sample and the
        // strap reading is never clobbered by a trainer HR.
        let crank = CrankTracker::new();
        let pwr = parse_notification(
            PM_UUID,
            &[0x00, 0x00, 0xC8, 0x00], // 200 W, no crank/wheel bits
            &crank,
            true, // strap alive → trainer HR suppressed
        );
        let strap = Telemetry {
            power: None,
            cadence: None,
            heart_rate: Some(142),
            speed: None,
        };

        let mut livedata = crate::app::LiveData::new();
        // main.rs merge: keep last-known value for fields the sample lacks.
        if pwr.power.is_some() {
            livedata.update(
                pwr.power.unwrap(),
                pwr.cadence.unwrap_or(livedata.crnt_rpm),
                pwr.heart_rate.unwrap_or(livedata.crnt_hr),
                pwr.speed.unwrap_or(livedata.crnt_vel),
                0.0,
                0.0,
                0.0,
                0.0,
            );
        }
        if strap.heart_rate.is_some() {
            livedata.update(
                strap.power.unwrap_or(livedata.crnt_pwr),
                strap.cadence.unwrap_or(livedata.crnt_rpm),
                strap.heart_rate.unwrap(),
                strap.speed.unwrap_or(livedata.crnt_vel),
                0.0,
                0.0,
                0.0,
                0.0,
            );
        }
        assert_eq!(livedata.crnt_pwr, 200);
        assert_eq!(livedata.crnt_hr, 142);
    }

    #[test]
    fn ramp_steps_is_monotonic_and_bounded() {
        // A big 300 W jump ramps upward and lands exactly on the target.
        let steps = ramp_steps(100, 400, 25);
        assert_eq!(steps.len(), 25);
        assert_eq!(*steps.last().unwrap(), 400);
        assert!(steps.windows(2).all(|w| w[0] <= w[1]),
            "ramp must be non-decreasing: {steps:?}");

        // Ramping down never undershoots below the floor.
        let down = ramp_steps(400, 50, 25);
        assert_eq!(*down.last().unwrap(), 50);
        assert!(down.windows(2).all(|w| w[0] >= w[1]));

        // No-op when the target is unchanged.
        assert_eq!(ramp_steps(200, 200, 25), vec![200]);
    }

    #[test]
    fn ramp_steps_tiny_nudge_reaches_goal() {
        // A 5 W nudge still converges to the new target over the step count.
        let steps = ramp_steps(200, 205, 25);
        assert_eq!(*steps.last().unwrap(), 205);
    }
}
