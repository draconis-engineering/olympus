// src/data.rs
//
// Data.rs handles FIT file parsing, SQLite storage, and user profile management.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

/// FIT file session data parsed from .fit files
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FitSession {
    pub total_distance: f32,
    pub total_calories: f32,
    pub total_power: f32, // Note: Not in your original SQLite schema, but kept in struct
    pub avg_speed: f32,
    pub max_speed: f32,
    pub max_heart_rate: u16,
    pub avg_heart_rate: u16,
    pub max_power: u16,
    pub avg_power: u16,
    pub timestamp: i64,
}

/// User profile, persisted to JSON in `data/user/profile.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub username: String,
    pub weight: f32,
    pub height: f32,
    pub ftp: u16,
    pub max_hr: u16,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            username: "Rider".to_string(),
            weight: 75.0,
            height: 180.0,
            ftp: 200,
            max_hr: 180,
        }
    }
}

/// Parses a .fit file and extracts the session summary data
pub fn parse_fit_file(path: &Path) -> Result<FitSession, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let records = fitparser::from_reader(&mut file)?;
    let mut session_data = FitSession::default();

    // Look for the single summary "session" message in the FIT file
    if let Some(session_record) = records
        .iter()
        .find(|r| r.kind() == fitparser::profile::MesgNum::Session)
    {
        for field in session_record.fields() {
            match field.name() {
                "total_distance" => {
                    if let fitparser::Value::Float32(v) = field.value() {
                        session_data.total_distance = *v;
                    }
                }
                "total_calories" => {
                    if let fitparser::Value::UInt16(v) = field.value() {
                        session_data.total_calories = *v as f32;
                    }
                }
                "avg_speed" => {
                    if let fitparser::Value::UInt16(v) = field.value() {
                        session_data.avg_speed = *v as f32 / 1000.0;
                    }
                } // FIT scales speed
                "max_speed" => {
                    if let fitparser::Value::UInt16(v) = field.value() {
                        session_data.max_speed = *v as f32 / 1000.0;
                    }
                }
                "max_heart_rate" => {
                    if let fitparser::Value::UInt8(v) = field.value() {
                        session_data.max_heart_rate = *v as u16;
                    }
                }
                "avg_heart_rate" => {
                    if let fitparser::Value::UInt8(v) = field.value() {
                        session_data.avg_heart_rate = *v as u16;
                    }
                }
                "max_power" => {
                    if let fitparser::Value::UInt16(v) = field.value() {
                        session_data.max_power = *v;
                    }
                }
                "avg_power" => {
                    if let fitparser::Value::UInt16(v) = field.value() {
                        session_data.avg_power = *v;
                    }
                }
                "timestamp" => {
                    if let fitparser::Value::Timestamp(v) = field.value() {
                        session_data.timestamp = v.timestamp();
                    }
                }
                _ => {}
            }
        }
        return Ok(session_data);
    }

    Err("No session summary record found in FIT file".into())
}

/// Path to the rider profile JSON file, relative to the project data dir.
pub const PROFILE_PATH: &str = "data/user/profile.json";

/// Load the rider profile from JSON, falling back to defaults when the file
/// doesn't exist yet (first run).
pub fn load_profile() -> UserProfile {
    let Ok(text) = std::fs::read_to_string(PROFILE_PATH) else {
        return UserProfile::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Persist the rider profile to JSON.
pub fn save_profile(profile: &UserProfile) -> Result<(), String> {
    if let Some(parent) = Path::new(PROFILE_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
    std::fs::write(PROFILE_PATH, json).map_err(|e| e.to_string())
}

/// Initialize SQLite database and create tables
pub fn init_db(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS fit_sessions (
            id INTEGER PRIMARY KEY,
            filename TEXT NOT NULL,
            total_distance REAL NOT NULL,
            total_calories REAL NOT NULL,
            avg_speed REAL NOT NULL,
            max_speed REAL NOT NULL,
            max_heart_rate INTEGER NOT NULL,
            avg_heart_rate INTEGER NOT NULL,
            max_power INTEGER NOT NULL,
            avg_power INTEGER NOT NULL,
            recorded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    Ok(conn)
}

/// Save a FIT session to SQLite
pub fn save_fit_session(
    conn: &Connection,
    session: &FitSession,
    filename: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO fit_sessions (filename, total_distance, total_calories, avg_speed, max_speed, max_heart_rate, avg_heart_rate, max_power, avg_power) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        (
            filename,
            session.total_distance as f64,
            session.total_calories as f64,
            session.avg_speed as f64,
            session.max_speed as f64,
            session.max_heart_rate as i32,
            session.avg_heart_rate as i32,
            session.max_power as i32,
            session.avg_power as i32,
        ),
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Database reading / browsing
// ---------------------------------------------------------------------------

/// One stored session row, as read back from the SQLite history.
#[derive(Debug, Clone, Default)]
pub struct StoredSession {
    pub id: i64,
    pub filename: String,
    pub total_distance: f32,
    pub total_calories: f32,
    pub avg_speed: f32,
    pub max_speed: f32,
    pub max_heart_rate: u16,
    pub avg_heart_rate: u16,
    pub max_power: u16,
    pub avg_power: u16,
    pub recorded_at: String,
}

/// Load the most recent stored sessions (newest first).
pub fn list_sessions(conn: &Connection, limit: usize) -> rusqlite::Result<Vec<StoredSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, filename, total_distance, total_calories, avg_speed, max_speed, \
         max_heart_rate, avg_heart_rate, max_power, avg_power, recorded_at \
         FROM fit_sessions ORDER BY id DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map([limit as i64], |row| {
        Ok(StoredSession {
            id: row.get(0)?,
            filename: row.get(1)?,
            total_distance: row.get(2)?,
            total_calories: row.get(3)?,
            avg_speed: row.get(4)?,
            max_speed: row.get(5)?,
            max_heart_rate: row.get::<_, i64>(6)? as u16,
            avg_heart_rate: row.get::<_, i64>(7)? as u16,
            max_power: row.get::<_, i64>(8)? as u16,
            avg_power: row.get::<_, i64>(9)? as u16,
            recorded_at: row.get(10)?,
        })
    })?;

    rows.collect()
}

/// Directory that stores the rider's workout files.
pub const WORKOUTS_DIR: &str = "data/workouts";

/// One workout available for loading.
#[derive(Debug, Clone)]
pub struct WorkoutEntry {
    pub name: String,
    pub path: String,
}

/// Scan the workouts directory for `.zwo` / `.erg` files.
pub fn list_workout_files() -> Vec<WorkoutEntry> {
    let Ok(read_dir) = std::fs::read_dir(WORKOUTS_DIR) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        let is_workout = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("zwo") || e.eq_ignore_ascii_case("erg"))
            .unwrap_or(false);
        if !is_workout {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("workout")
            .to_string();
        entries.push(WorkoutEntry {
            name,
            path: path.to_string_lossy().into_owned(),
        });
    }

    // Deterministic ordering keeps the cursor stable between redraws.
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries
}
