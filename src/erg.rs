// src/erg.rs
//
// ERG resistance control module for Olympus.
// Loads .erg / .zwo workout files, schedules interval targets, and (via the
// BLE driver) drives the trainer's ERG target power.
//
// .erg format (Tacx / TrainerRoad legacy, simple key-value pairs):
//   TARGET_POWER: 200     (watts)
//   DURATION: 300          (seconds)
//   REST_POWER: 120
//   REST_DURATION: 180
//
// .zwo format (Zwift workout, XML):
//   <workout_file>
//     <workout>
//       <Warmup Duration="600" PowerLow="0.5" PowerHigh="0.6"/>
//       <IntervalsT Repeat="3" OnDuration="300" OffDuration="180"
//                   OnPower="0.9" OffPower="0.5"/>
//       <Cooldown Duration="300" PowerLow="0.4" PowerHigh="0.5"/>
//     </workout>
//   </workout_file>
//   Powers may be absolute watts or a multiplier of FTP.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single interval step inside a workout.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ErgTarget {
    /// Target power in watts.
    pub target_power: u16,
    /// Length of this step in seconds.
    pub duration_seconds: u16,
    /// Recovery/rest power in watts (0 when this is a plain effort).
    pub rest_power: u16,
    /// Length of the recovery period in seconds (0 when this is a plain effort).
    pub rest_duration: u16,
}

/// Holds a parsed workout together with helpers to compute the live ERG target.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Workout {
    /// Optional friendly name (parsed from .zwo metadata).
    pub name: Option<String>,
    /// Ordered list of steps. Relative times are derived by accumulation.
    pub steps: Vec<WorkoutStep>,
    /// Total duration in seconds (sum of all steps).
    pub total_seconds: u32,
}

/// One scheduled step with absolute (start, end) window and a target.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WorkoutStep {
    /// Start time of the step, seconds from the workout's beginning.
    pub start_secs: u32,
    /// End time of the step, seconds from the workout's beginning.
    pub end_secs: u32,
    /// Target power in watts during the step.
    pub target_power: u16,
}

impl Workout {
    /// Build a `Workout` from a flat list of effort steps. Each step's
    /// recovery (rest_power/rest_duration) expands into its own sub-step, so
    /// the returned schedule alternates effort and recovery.
    pub fn from_targets(targets: &[ErgTarget]) -> Self {
        let mut steps = Vec::new();
        let mut t: u32 = 0;

        for target in targets {
            if target.duration_seconds > 0 {
                let start = t;
                let end = t + target.duration_seconds as u32;
                steps.push(WorkoutStep {
                    start_secs: start,
                    end_secs: end,
                    target_power: target.target_power,
                });
                t = end;
            }
            if target.rest_duration > 0 {
                let start = t;
                let end = t + target.rest_duration as u32;
                steps.push(WorkoutStep {
                    start_secs: start,
                    end_secs: end,
                    target_power: target.rest_power,
                });
                t = end;
            }
        }

        Workout {
            name: None,
            total_seconds: t,
            steps,
        }
    }

    /// Given elapsed seconds of the ride, return the step currently active.
    pub fn step_at(&self, elapsed_secs: u32) -> Option<&WorkoutStep> {
        self.steps
            .iter()
            .find(|s| elapsed_secs >= s.start_secs && elapsed_secs < s.end_secs)
    }

    /// True when `elapsed_secs` is beyond the end of the workout.
    pub fn is_finished(&self, elapsed_secs: u32) -> bool {
        elapsed_secs >= self.total_seconds
    }
}

/// Load an ERG workout from a .erg file.
pub fn load_erg_workout(path: &Path) -> Result<Workout, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read ERG file: {e}"))?;

    let mut targets = Vec::new();
    let mut current: Option<ErgTarget> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some(t) = current.take() {
                if t.duration_seconds > 0 {
                    targets.push(t);
                }
            }
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let Ok(value) = value.trim().parse::<u16>() else {
            continue;
        };

        match key {
            "TARGET_POWER" => {
                current = Some(ErgTarget {
                    target_power: value,
                    duration_seconds: 0,
                    rest_power: 0,
                    rest_duration: 0,
                });
            }
            "DURATION" => {
                if let Some(ref mut t) = current {
                    t.duration_seconds = value;
                }
            }
            "REST_POWER" => {
                if let Some(ref mut t) = current {
                    t.rest_power = value;
                }
            }
            "REST_DURATION" => {
                if let Some(ref mut t) = current {
                    t.rest_duration = value;
                }
            }
            _ => {}
        }
    }

    if let Some(t) = current {
        if t.duration_seconds > 0 {
            targets.push(t);
        }
    }

    if targets.is_empty() {
        return Err("No valid steps found in ERG file".into());
    }

    Ok(Workout::from_targets(&targets))
}

// ---------------------------------------------------------------------------
// ZWO (Zwift Workout) parsing via xml-rs
// ---------------------------------------------------------------------------

/// Parse a .zwo workout file into a flat list of erg targets.
///
/// `ftp` is the rider's functional threshold power used to resolve the
/// fractional powers (e.g. 0.9 -> 0.9 * ftp). Absolute integer powers are
/// taken as-is.
pub fn parse_zwo_workout(path: &Path, ftp: u16) -> Result<Workout, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open ZWO file: {e}"))?;
    let source = std::io::BufReader::new(file);
    let parser = xml::reader::EventReader::new(source);

    let mut targets: Vec<ErgTarget> = Vec::new();
    let mut name: Option<String> = None;
    let mut in_name = false;

    for event in parser {
        use xml::reader::XmlEvent;

        match event.map_err(|e| format!("Invalid ZWO XML: {e}"))? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                let local = name.local_name.as_str();

                // Capture the workout name inside <name><en_US>...</en_US></name>.
                if local == "name" {
                    in_name = true;
                    continue;
                }
                if in_name && local == "en_US" {
                    // name text handled below via Characters
                }

                let get = |key: &str| -> Option<f32> {
                    attributes
                        .iter()
                        .find(|a| a.name.local_name == key)
                        .and_then(|a| a.value.parse().ok())
                };

                match local {
                    "Warmup" | "Cooldown" => {
                        let duration = get("Duration").unwrap_or(0.0) as u32;
                        let low = get("PowerLow");
                        let high = get("PowerHigh");
                        let power = get("Power");
                        if let Some(t) = resolve_power(power.or(high).or(low), ftp) {
                            targets.push(ErgTarget {
                                target_power: t,
                                duration_seconds: duration as u16,
                                rest_power: 0,
                                rest_duration: 0,
                            });
                        }
                    }
                    "SteadyState" => {
                        let duration = get("Duration").unwrap_or(0.0) as u32;
                        let power = get("Power");
                        if let Some(t) = resolve_power(power, ftp) {
                            targets.push(ErgTarget {
                                target_power: t,
                                duration_seconds: duration as u16,
                                rest_power: 0,
                                rest_duration: 0,
                            });
                        }
                    }
                    "Intervals" | "IntervalsT" => {
                        let repeat = get("Repeat").unwrap_or(1.0) as u32;
                        let on_duration = get("OnDuration").unwrap_or(0.0) as u32;
                        let off_duration = get("OffDuration").unwrap_or(0.0) as u32;
                        let on = resolve_power(get("OnPower"), ftp).unwrap_or(0);
                        let off = resolve_power(get("OffPower"), ftp).unwrap_or(0);
                        for _ in 0..repeat {
                            targets.push(ErgTarget {
                                target_power: on,
                                duration_seconds: on_duration as u16,
                                rest_power: off,
                                rest_duration: off_duration as u16,
                            });
                        }
                    }
                    "Ramp" => {
                        let duration = get("Duration").unwrap_or(0.0) as u32;
                        let ftp_val = get("Ftp") as Option<f32>;
                        // Ramp power is typically given as a fraction (e.g.
                        // <Ramp Duration="120" Ftp="1.0"/>). Treat `Ftp` as a
                        // fraction of the rider's FTP.
                        if let Some(fraction) = ftp_val {
                            targets.push(ErgTarget {
                                target_power: resolve_power(Some(fraction), ftp).unwrap_or(0),
                                duration_seconds: duration as u16,
                                rest_power: 0,
                                rest_duration: 0,
                            });
                        }
                    }
                    // FreeRide / rest sections produce no ERG target.
                    _ => {}
                }
            }
            XmlEvent::Characters(text) => {
                if in_name {
                    name = Some(text.trim().to_string());
                }
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == "name" {
                    in_name = false;
                }
            }
            _ => {}
        }
    }

    if targets.is_empty() {
        return Err("ZWO file contained no supported steps".into());
    }

    let mut workout = Workout::from_targets(&targets);
    workout.name = name.filter(|n| !n.is_empty());

    Ok(workout)
}

/// Resolve a ZWO power. Values <= 10.0 are interpreted as a fraction of FTP,
/// anything larger as absolute watts. Returns None for a missing power.
fn resolve_power(maybe: Option<f32>, ftp: u16) -> Option<u16> {
    let v = maybe?;
    if v <= 10.0 {
        Some(((v * ftp as f32).round() as u16).max(0))
    } else {
        Some(v.round() as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_erg_workout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.erg");
        std::fs::write(&path, "TARGET_POWER: 200\nDURATION: 120\n").unwrap();

        let workout = load_erg_workout(&path).unwrap();
        assert_eq!(workout.steps.len(), 1);
        assert_eq!(workout.steps[0].target_power, 200);
        assert_eq!(workout.steps[0].start_secs, 0);
        assert_eq!(workout.steps[0].end_secs, 120);
    }

    #[test]
    fn test_erg_recovery_expands() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.erg");
        std::fs::write(
            &path,
            "TARGET_POWER: 200\nDURATION: 60\nREST_POWER: 100\nREST_DURATION: 30\n",
        )
        .unwrap();

        let workout = load_erg_workout(&path).unwrap();
        assert_eq!(workout.steps.len(), 2);
        assert_eq!(workout.steps[0].target_power, 200);
        assert_eq!(workout.steps[1].target_power, 100);
        assert_eq!(workout.total_seconds, 90);
    }

    #[test]
    fn test_zwo_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.zwo");
        std::fs::write(
            &path,
            r#"<?xml version="1.0"?>
<workout_file>
  <name><en_US>Test Workout</en_US></name>
  <workout>
    <Warmup Duration="600" PowerLow="0.5" PowerHigh="0.6"/>
    <IntervalsT Repeat="3" OnDuration="300" OffDuration="180" OnPower="0.9" OffPower="0.5"/>
    <Cooldown Duration="300" PowerLow="0.4" PowerHigh="0.5"/>
  </workout>
</workout_file>"#,
        )
        .unwrap();

        let workout = parse_zwo_workout(&path, 300).unwrap();
        // warmup (1) + 3 intervals e/o (6) + cooldown (1) = 8 steps
        assert_eq!(workout.steps.len(), 8);
        assert_eq!(
            workout.steps[0].target_power,
            (0.6f32 * 300.0).round() as u16
        );
        assert_eq!(
            workout.steps[1].target_power,
            (0.9f32 * 300.0).round() as u16
        );
        assert_eq!(workout.name.as_deref(), Some("Test Workout"));
    }

    #[test]
    fn test_zwo_absolute_power() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.zwo");
        std::fs::write(
            &path,
            r#"<?xml version="1.0"?>
<workout_file>
  <workout>
    <SteadyState Duration="600" Power="200"/>
  </workout>
</workout_file>"#,
        )
        .unwrap();

        let workout = parse_zwo_workout(&path, 300).unwrap();
        assert_eq!(workout.steps[0].target_power, 200);
    }
}
