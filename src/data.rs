// src/data.rs
//
// Data.rs handles FIT file parsing, SQLite storage, and user profile management.

use fitparser::FitFile;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// FIT file session data parsed from .fit files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitSession {
    pub total_distance: f32,
    pub total_calories: f32,
    pub total_power: f32,
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

    let profile = rows.next().map(|r| r.unwrap());
    Ok(profile)
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
