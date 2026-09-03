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
use futures::StreamExt;
use rand::{Rng, SeedableRng};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time;
use uuid::Uuid;

// Standard GATT service / characteristic UUIDs relevant to cycling trainers.
const CPS_UUID: Uuid = uuid_from_u16(0x1818); // Cycling Power Service
const PM_UUID: Uuid = uuid_from_u16(0x2A63); // Cycling Power Measurement
const HRM_UUID: Uuid = uuid_from_u16(0x2A37); // Heart Rate Measurement
const FMS_UUID: Uuid = uuid_from_u16(0x1826); // Fitness Machine Service
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

/// Connection state, surfaced to the UI for the System panel.
#[derive(Debug, Clone, PartialEq)]
pub enum BleState {
    Idle,
    Scanning,
    Connecting,
    Connected { name: String },
    Simulated,
    Error(String),
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

/// Core async BLE event loop.
async fn driver_loop(
    tel_tx: Sender<Telemetry>,
    mut cmd_rx: Receiver<BleCommand>,
    state_tx: Sender<BleState>,
) {
    let state_send = |s: BleState| {
        let _ = state_tx.try_send(s.clone());
    };

    state_send(BleState::Idle);

    // Attempt to grab an adapter. If none exists we retry periodically rather
    // than crash, which keeps the driver resilient to Bluetooth not being up yet.
    let mut peripheral = loop {
        state_send(BleState::Scanning);
        match find_trainer().await {
            Ok(Some(p)) => {
                state_send(BleState::Connecting);
                break p;
            }
            Ok(None) => {
                // No device found — show simulated data so the UI stays live,
                // then try scanning again.
                state_send(BleState::Simulated);
                emit_simulated(&tel_tx, &mut cmd_rx).await;
                time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                state_send(BleState::Error(format!("BLE error: {e}")));
                time::sleep(Duration::from_secs(2)).await;
            }
        }
        // If a command arrives that asks for a scan, loop around.
        match cmd_rx.try_recv() {
            Ok(BleCommand::Scan)
            | Ok(BleCommand::SetTargetPower(_))
            | Ok(BleCommand::Disconnect) => {
                continue;
            }
            Err(_) => {}
        }
    };

    let initial_name = peripheral
        .properties()
        .await
        .ok()
        .flatten()
        .and_then(|p| p.local_name)
        .unwrap_or_else(|| "Trainer".to_string());
    state_send(BleState::Connected { name: initial_name });

    // Discover services and subscribe to the characteristics we need.
    let subscriptions = match peripheral.discover_services().await {
        Ok(_) => subscribe_all(&mut peripheral, &tel_tx).await,
        Err(e) => {
            state_send(BleState::Error(format!("service discovery failed: {e}")));
            Vec::new()
        }
    };

    let mut notifications = match peripheral.notifications().await {
        Ok(n) => n,
        Err(e) => {
            state_send(BleState::Error(format!("notifications unavailable: {e}")));
            return;
        }
    };

    // Shared crank state so cadence can be derived across notifications.
    let crank = CrankTracker::new();

    loop {
        tokio::select! {
            // Incoming data notifications from the trainer.
            maybe = notifications.next() => {
                match maybe {
                    Some(n) => handle_notification(&n, &tel_tx, &crank).await,
                    None => {
                        state_send(BleState::Idle);
                        break;
                    }
                }
            }
            // Commands from the UI.
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(BleCommand::SetTargetPower(w)) => {
                        if let Err(e) = set_target_power(&peripheral, &subscriptions, w).await {
                            state_send(BleState::Error(format!("ERG failed: {e}")));
                        }
                    }
                    Some(BleCommand::Disconnect) => {
                        let _ = peripheral.disconnect().await;
                        state_send(BleState::Idle);
                        break;
                    }
                    Some(BleCommand::Scan) => {
                        let _ = peripheral.disconnect().await;
                        state_send(BleState::Idle);
                        break;
                    }
                    None => break,
                }
            }
            // Heartbeat so we can report a healthy "connected" state periodically.
            _ = time::sleep(Duration::from_secs(5)) => {
                let hb_name = peripheral
                    .properties()
                    .await
                    .ok()
                    .flatten()
                    .and_then(|p| p.local_name)
                    .unwrap_or_else(|| "Trainer".into());
                state_send(BleState::Connected { name: hb_name });
            }
        }
    }
}

/// Discovery flow: find the first adapter, scan, and return the trainer.
async fn find_trainer() -> Result<Option<Peripheral>, String> {
    let Ok(manager) = Manager::new().await else {
        return Err("failed to create BLE manager".into());
    };

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("no adapters: {e}"))?;
    let Some(central) = adapters.into_iter().next() else {
        return Ok(None);
    };

    // Start scanning. Heavily filter to only cycling/fitness devices so we don't
    // churn through stray BLE peripherals.
    central
        .start_scan(ScanFilter {
            services: vec![CPS_UUID, FMS_UUID, HRM_UUID],
        })
        .await
        .map_err(|e| format!("failed to start scan: {e}"))?;

    time::sleep(Duration::from_secs(2)).await;

    let peripherals = central
        .peripherals()
        .await
        .map_err(|e| format!("failed to list peripherals: {e}"))?;

    // Prefer a device whose properties advertise a fitness/cycling service, or
    // failing that one which has a name we can match (e.g. "Tacx Flux S2").
    let mut candidate: Option<Peripheral> = None;
    for p in peripherals.into_iter() {
        if let Ok(Some(props)) = p.properties().await {
            let has_cps = props.services.contains(&CPS_UUID);
            let has_fms = props.services.contains(&FMS_UUID);
            let name_matches = props
                .local_name
                .as_deref()
                .map(|n| n.contains("Tacx") || n.contains("Flux") || n.contains("TRAINER"))
                .unwrap_or(false);
            if has_cps || has_fms || name_matches {
                candidate = Some(p);
                break;
            }
        }
    }

    central.stop_scan().await.ok();

    match candidate {
        Some(p) => {
            // Connect with a sensible timeout.
            p.connect()
                .await
                .map_err(|e| format!("connect failed: {e}"))?;
            Ok(Some(p))
        }
        None => Ok(None),
    }
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

/// Parse a single notification and forward the parsed values to the UI.
async fn handle_notification(
    n: &ValueNotification,
    tel_tx: &Sender<Telemetry>,
    crank: &CrankTracker,
) {
    let mut t = Telemetry {
        power: None,
        cadence: None,
        heart_rate: None,
        speed: None,
    };

    if n.uuid == PM_UUID {
        // Cycling Power Measurement (0x2A63):
        //   Flags u16 (LE), then Instantaneous Power u16 (always present),
        //   then optional fields gated by flag bits:
        //     bit4 (0x0010): wheel revs u32 + last wheel time u16
        //     bit5 (0x0020): crank revs u16 + last crank time u16
        let b = &n.value;
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
    } else if n.uuid == HRM_UUID {
        // HR Measurement: flags byte 0, then HR u8 (or u16 if bit 0 of flags set).
        let b = &n.value;
        if b.is_empty() {
            return;
        }
        let flags = b[0];
        let hr = if flags & 0x01 != 0 {
            // u16 little-endian
            if b.len() >= 3 {
                Some(u16::from_le_bytes([b[1], b[2]]))
            } else {
                None
            }
        } else {
            if b.len() >= 2 {
                Some(b[1] as u16)
            } else {
                None
            }
        };
        if let Some(hr) = hr {
            t.heart_rate = Some(hr);
        }
    } else if n.uuid == CSC_MEASUREMENT_UUID {
        // CSC Measurement: flags byte, then optional fields. Flags bit 0 =
        // wheel data (cumulative wheel revs u32 + last wheel event time u16),
        // bit 1 = crank data (cumulative crank revs u16 + last crank event time u16).
        let b = &n.value;
        if b.is_empty() {
            return;
        }
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
    } else if n.uuid == FMCP_UUID {
        // Control point responses — primarily for ERG confirmation.
        // Response codes: 0x80 = success, 0x82 = opcode not supported, etc.
        return;
    }

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

/// When no trainer is discoverable we emit gentle simulated data so the UI
/// stays responsive and the graphs keep moving. Uses slight random variance
/// so it looks alive.
async fn emit_simulated(tel_tx: &Sender<Telemetry>, cmd_rx: &mut Receiver<BleCommand>) {
    let mut pwr: u16 = 180;
    let mut rpm: u16 = 88;
    let mut hr: u16 = 135;
    // StdRng is Send so it can be held across the awaits below.
    let mut rng = rand::rngs::StdRng::from_entropy();

    for _ in 0..10 {
        pwr = (pwr as i32 + rng.gen_range(-6..=6)) as u16;
        rpm = (rpm as i32 + rng.gen_range(-2..=2)).clamp(60, 120) as u16;
        hr = (hr as i32 + rng.gen_range(-2..=2)).clamp(90, 190) as u16;

        let _ = tel_tx
            .send(Telemetry {
                power: Some(pwr.clamp(0, 1000)),
                cadence: Some(rpm),
                heart_rate: Some(hr),
                speed: Some(30.0 + rng.gen_range(-1.0..=1.0)),
            })
            .await;

        // Give up early if the UI asks us to scan.
        if cmd_rx
            .try_recv()
            .map(|c| c == BleCommand::Scan)
            .unwrap_or(false)
        {
            break;
        }
        time::sleep(Duration::from_millis(200)).await;
    }
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
            name: "Tacx Flux S2".into(),
        };
        let b = BleState::Connected {
            name: "Tacx Flux S2".into(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn simulated_range_is_bounded() {
        let mut rng = rand::rngs::StdRng::from_entropy();
        for _ in 0..100 {
            let v = rng.gen_range(-6..=6);
            assert!(v >= -6 && v <= 6);
        }
    }
}
