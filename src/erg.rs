// src/erg.rs
//
// ERG resistance control module for Olympus.
// Handles erg mode target power adjustments from workout files.

use crate::data::{FitSession, UserProfile};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// ERG workout target structure
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ErgTarget {
    pub target_power: u16,
    pub duration_seconds: u16,
    pub rest_power: u16,
    pub rest_duration: u16,
}

/// Load an ERG workout from a .erg file
pub fn load_erg_workout(path: &Path) -> Result<Vec<ErgTarget>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read ERG file: {}", e))?;
    
    // ERG format is simple key-value pairs
    let mut targets = Vec::new();
    let mut current_target: Option<ErgTarget> = None;
    
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some(t) = current_target {
                targets.push(t);
                current_target = None;
            }
            continue;
        }
        
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        
        let key = parts[0].trim();
        let value: u16 = parts[1].trim().parse()
            .map_err(|_| format!("Invalid value in ERG file: {}", line))?;
        
        match key {
            "TARGET_POWER" => {
                current_target = Some(ErgTarget {
                    target_power: value,
                    duration_seconds: 0,
                    rest_power: 0,
                    rest_duration: 0,
                });
            }
            "DURATION" => {
                if let Some(ref mut t) = current_target {
                    t.duration_seconds = value;
                }
            }
            "REST_POWER" => {
                if let Some(ref mut t) = current_target {
                    t.rest_power = value;
                }
            }
            "REST_DURATION" => {
                if let Some(ref mut t) = current_target {
                    t.rest_duration = value;
                }
            }
            _ => {}
        }
    }
    
    if let Some(t) = current_target {
        targets.push(t);
    }
    
    Ok(targets)
}

/// Send ERG resistance target to trainer
pub fn set_erg_target(trainer_power: u16) -> Result<(), String> {
    // TODO: Implement btleplug Fitness Machine Control Point write
    // This sends a control point command to set the ERG target power
    Err("ERG control not yet fully implemented".into())
}

/// Parse a .zwo (Zwift workout) file
pub fn parse_zwo_workout(path: &Path) -> Result<Vec<ErgTarget>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read ZWO file: {}", e))?;
    
    // TODO: Full ZWO parsing - currently returns empty
    // ZWO files contain structured workout data with intervals, targets, etc.
    Err("ZWO parsing not yet implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_load_erg_workout() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.erg");
        std::fs::write(&path, "TARGET_POWER: 200\nDURATION: 120\n").unwrap();
        
        let targets = load_erg_workout(&path).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].target_power, 200);
        assert_eq!(targets[0].duration_seconds, 120);
    }
    
    #[test]
    fn test_zwo_parsing_stub() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.zwo");
        std::fs::write(&path, "").unwrap();
        
        let result = parse_zwo_workout(&path);
        assert!(result.is_err());
    }
}