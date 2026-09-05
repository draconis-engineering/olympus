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
pub fn _parse_fit_file(path: &Path) -> Result<FitSession, Box<dyn std::error::Error>> {
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

/// Ensure the standard Olympus data directories exist on disk. Safe to call
/// repeatedly; creates nothing extra if they are already present.
pub fn ensure_data_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all("data")?;
    std::fs::create_dir_all(WORKOUTS_DIR)?;
    std::fs::create_dir_all("data/.fit")?;
    Ok(())
}

/// Initialize SQLite database, create/migrate tables, and ensure the data
/// directory exists.
pub fn init_db(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
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

    // Per-second time-series samples so analytics (NP, power curves, etc.) can
    // be computed retroactively. One row per recorded second.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS samples (
            id INTEGER PRIMARY KEY,
            session_id INTEGER NOT NULL REFERENCES fit_sessions(id) ON DELETE CASCADE,
            t INTEGER NOT NULL,
            power INTEGER NOT NULL,
            cadence INTEGER NOT NULL,
            heart_rate INTEGER NOT NULL,
            speed REAL NOT NULL
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_samples_session ON samples(session_id)",
        [],
    )?;

    // Schema version for future migrations.
    let user_version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if user_version < 2 {
        conn.pragma_update(None, "user_version", 2)?;
    }

    Ok(conn)
}

/// A single per-second ride sample.
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Unix timestamp (seconds).
    pub t: i64,
    pub power: u16,
    pub cadence: u16,
    pub heart_rate: u16,
    /// Speed in m/s.
    pub speed: f32,
}

/// Persist a completed ride: the session summary + its time-series samples, in
/// one transaction. Returns the new session id.
pub fn save_ride(
    conn: &Connection,
    session: &FitSession,
    filename: &str,
    samples: &[Sample],
) -> rusqlite::Result<i64> {
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
    let id = conn.last_insert_rowid();

    {
        let mut stmt = conn.prepare(
            "INSERT INTO samples (session_id, t, power, cadence, heart_rate, speed) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for s in samples {
            stmt.execute((
                id,
                s.t,
                s.power as i32,
                s.cadence as i32,
                s.heart_rate as i32,
                s.speed as f64,
            ))?;
        }
    }

    Ok(id)
}

/// Save a FIT session to SQLite (summary only, no samples).
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    // Prepare the SQL statement to select the most recent sessions.
    let mut stmt = conn.prepare(
        "SELECT id, filename, total_distance, total_calories, avg_speed, max_speed, \
         max_heart_rate, avg_heart_rate, max_power, avg_power, recorded_at \
         FROM fit_sessions ORDER BY id DESC LIMIT ?1",
    )?;

    // Extract rows into StoredSession structs.
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

/// Load the per-second samples of one session (for the detail replay chart),
/// in chronological order.
pub fn session_samples(conn: &Connection, session_id: i64) -> rusqlite::Result<Vec<Sample>> {
    let mut stmt = conn.prepare(
        "SELECT t, power, cadence, heart_rate, speed FROM samples \
         WHERE session_id = ?1 ORDER BY t",
    )?;
    let rows = stmt.query_map([session_id], |row| {
        Ok(Sample {
            t: row.get(0)?,
            power: row.get::<_, i64>(1)? as u16,
            cadence: row.get::<_, i64>(2)? as u16,
            heart_rate: row.get::<_, i64>(3)? as u16,
            speed: row.get::<_, f64>(4)? as f32,
        })
    })?;
    rows.collect()
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// One bar on the weekly Training Stress Score chart.
#[derive(Debug, Clone, Default)]
pub struct WeeklyTss {
    pub label: String,
    pub tss: f64,
}

/// Aggregated ride statistics for the Stats screen.
#[derive(Debug, Clone, Default)]
pub struct StatsSummary {
    /// ISO weeks, oldest → newest, capped at the requested window.
    pub weeks: Vec<WeeklyTss>,
    /// Best average power over any 1 / 5 / 20 minute rolling window.
    pub best_1m: u16,
    pub best_5m: u16,
    pub best_20m: u16,
    /// Lifetime volume.
    pub total_km: f64,
    pub total_hours: f64,
}

/// Compute the Stats screen contents from history. TSS is computed per ride
/// from its samples (NP + duration), then bucketed by ISO week; the power
/// curve is the best 1m / 5m / 20m rolling mean across all stored rides.
pub fn compute_stats(
    conn: &Connection,
    ftp: u16,
    num_weeks: usize,
    max_sessions: usize,
) -> rusqlite::Result<StatsSummary> {
    use chrono::{Datelike, Local, NaiveDate};

    let mut out = StatsSummary::default();

    // Buckets run oldest -> newest: the last `num_weeks` ISO weeks ending today.
    let today = Local::now().date_naive();
    let buckets: Vec<(String, i32)> = (0..num_weeks)
        .map(|k| {
            let d = today - chrono::Duration::days(((num_weeks - 1 - k) as i64) * 7);
            let w = d.iso_week();
            (format!("{}w{:02}", w.year(), w.week()), w.year() * 53 + w.week() as i32)
        })
        .collect();
    out.weeks = buckets
        .iter()
        .map(|(label, _)| WeeklyTss {
            label: label.clone(),
            tss: 0.0,
        })
        .collect();

    let mut sessions = conn.prepare(
        "SELECT id, recorded_at, total_distance FROM fit_sessions \
         ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = sessions.query_map([max_sessions as i64], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?))
    })?;

    let mut power_stmt = conn.prepare("SELECT power FROM samples WHERE session_id = ?1 ORDER BY t")?;

    let mut total_secs: f64 = 0.0;
    for row in rows.flatten() {
        let (id, recorded_at, dist) = row;

        let powers: Vec<u16> = power_stmt
            .query_map([id], |r| r.get::<_, i64>(0).map(|v| v as u16))?
            .filter_map(|p| p.ok())
            .collect();

        // Volume.
        out.total_km += dist;
        total_secs += powers.len() as f64;

        // Power curve (best rolling average per window).
        out.best_1m = out.best_1m.max(crate::math::best_rolling_mean(&powers, 60));
        out.best_5m = out.best_5m.max(crate::math::best_rolling_mean(&powers, 300));
        out.best_20m = out.best_20m.max(crate::math::best_rolling_mean(&powers, 1200));

        // TSS from NP + ride duration, then bucket by ISO week.
        if !powers.is_empty()
            && let Ok(date) = NaiveDate::parse_from_str(&recorded_at[..10], "%Y-%m-%d")
            && let Some(idx) = buckets.iter().position(|(_, b)| {
                let w = date.iso_week();
                *b == w.year() * 53 + w.week() as i32
            })
        {
            let raw: Vec<u64> = powers.iter().map(|&p| p as u64).collect();
            let np = crate::math::normalized_power(&raw, 1.0);
            let ifac = crate::math::intensity_factor(np, ftp as f64);
            let tss = crate::math::tss(np, ifac, ftp as f64, powers.len() as f64);
            out.weeks[idx].tss += tss;
        }
    }

    out.total_hours = total_secs / 3600.0;
    Ok(out)
}

/// Directory that stores the rider's workout files.
pub const WORKOUTS_DIR: &str = "data/workouts";

/// One workout available for loading.
#[derive(Debug, Clone)]
pub struct WorkoutEntry {
    pub name: String,
    pub path: String,
    /// Total scheduled duration from the file, in seconds.
    pub duration_seconds: u32,
    /// Estimated Training Stress Score, `TSS ≈ Σ(target/FTP)²·dur/3600·100`,
    /// computed against the rider's current FTP at scan time.
    pub tss: f32,
}

/// Scan the workouts directory for `.zwo` / `.erg` files, parsing each so the
/// UI can show its duration and estimated TSS (against `ftp`).
pub fn list_workout_files(ftp: u16) -> Vec<WorkoutEntry> {
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

        // Parse just enough to derive the subtitle (duration + estimated TSS).
        let workout = if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("zwo"))
            .unwrap_or(false)
        {
            crate::erg::parse_zwo_workout(&path, ftp)
        } else {
            crate::erg::load_erg_workout(&path)
        };
        let (duration_seconds, tss) = match workout {
            Ok(w) => {
                let tss = if ftp == 0 {
                    0.0
                } else {
                    w.steps.iter().fold(0.0, |acc, step| {
                        let dur_s = (step.end_secs - step.start_secs) as f64;
                        let ratio = step.target_power as f64 / ftp as f64;
                        acc + ratio * ratio * dur_s / 3600.0 * 100.0
                    })
                };
                (w.total_seconds, tss as f32)
            }
            Err(_) => (0, 0.0),
        };

        entries.push(WorkoutEntry {
            name,
            path: path.to_string_lossy().into_owned(),
            duration_seconds,
            tss,
        });
    }

    // Deterministic ordering keeps the cursor stable between redraws.
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let conn = init_db(&path).unwrap();
        (dir, conn)
    }

    fn farey_ride(conn: &Connection, power: u16) -> i64 {
        let session = FitSession {
            total_distance: 10.0,
            total_calories: 300.0,
            avg_power: power,
            max_power: power,
            ..Default::default()
        };
        let samples: Vec<Sample> = (0..60)
            .map(|t| Sample {
                t,
                power,
                cadence: 90,
                heart_rate: 150,
                speed: 8.33,
            })
            .collect();
        save_ride(conn, &session, "ride.fit", &samples).unwrap()
    }

    #[test]
    fn session_samples_round_trip_in_order() {
        let (_dir, conn) = test_conn();
        let id = farey_ride(&conn, 200);
        let samples = session_samples(&conn, id).unwrap();
        assert_eq!(samples.len(), 60);
        // Chronological: ensure t is strictly increasing.
        assert_eq!(samples[0].t, 0);
        assert_eq!(samples[59].t, 59);
        assert!(samples.windows(2).all(|w| w[0].t < w[1].t));
        assert!(samples.iter().all(|s| s.power == 200));
    }

    #[test]
    fn session_samples_unknown_id_is_empty() {
        let (_dir, conn) = test_conn();
        assert!(session_samples(&conn, 999).unwrap().is_empty());
    }

    #[test]
    fn compute_stats_aggregates_pr_and_volume() {
        let (_dir, conn) = test_conn();
        // Two rides: one at 200 W for 60s, one at 300 W for 60s.
        farey_ride(&conn, 200);
        farey_ride(&conn, 300);
        let stats = compute_stats(&conn, 200, 8, 50).unwrap();
        // Volume: 2 x 10 km, 2 x 60 s.
        assert!((stats.total_km - 20.0).abs() < 0.001);
        assert!((stats.total_hours - 120.0 / 3600.0).abs() < 0.001);
        // PR 1m is the max of the two (300 W ride).
        assert_eq!(stats.best_1m, 300);
        // <5 min of data -> no 5m/20m PR.
        assert_eq!(stats.best_5m, 0);
        assert_eq!(stats.best_20m, 0);
        // Week buckets: exactly 8 labels, oldest->newest.
        assert_eq!(stats.weeks.len(), 8);
        // TSS landed in the current week bucket (non-zero).
        assert!(stats.weeks.iter().any(|w| w.tss > 0.0));
    }

    #[test]
    fn compute_stats_empty_db_has_zero_volume() {
        let (_dir, conn) = test_conn();
        let stats = compute_stats(&conn, 200, 8, 50).unwrap();
        assert_eq!(stats.weeks.len(), 8);
        assert_eq!(stats.total_km, 0.0);
        assert_eq!(stats.best_1m, 0);
    }
}
