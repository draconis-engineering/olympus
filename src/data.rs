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

/// User profile stored in SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub weight: f32,
    pub height: f32,
    pub ftp: u16,
    pub max_hr: u16,
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

/// Initialize SQLite database and create tables
pub fn init_db(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_profiles (
            id INTEGER PRIMARY KEY,
            weight REAL NOT NULL,
            height REAL NOT NULL,
            ftp INTEGER NOT NULL,
            max_hr INTEGER NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

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

/// Save user profile to SQLite
pub fn save_user_profile(conn: &Connection, profile: &UserProfile) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO user_profiles (weight, height, ftp, max_hr) VALUES (?1, ?2, ?3, ?4)",
        (
            profile.weight as f64,
            profile.height as f64,
            profile.ftp as i32,
            profile.max_hr as i32,
        ),
    )?;
    Ok(())
}

/// Load user profile from SQLite (most recent)
pub fn load_user_profile(conn: &Connection) -> rusqlite::Result<Option<UserProfile>> {
    let mut stmt = conn.prepare(
        "SELECT weight, height, ftp, max_hr FROM user_profiles ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map([], |row| {
        Ok(UserProfile {
            weight: row.get::<_, f64>(0)? as f32,
            height: row.get::<_, f64>(1)? as f32,
            ftp: row.get::<_, i32>(2)? as u16,
            max_hr: row.get::<_, i32>(3)? as u16,
        })
    })?;

    if let Some(result) = rows.next() {
        Ok(Some(result?))
    } else {
        Ok(None)
    }
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
