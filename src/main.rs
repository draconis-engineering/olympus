// src/main.rs
//
// Main entry point for the Olympus application.

mod app;
mod ble;
mod boot;
mod data;
mod erg;
mod fit_writer;
mod math;
mod nav;
mod render;

use app::{Action, App, LiveData, UserData, UserStats, WorkoutData};
use boot::{init, restore};
use chrono::Utc;
use crossterm::event::{Event, KeyEventKind};
use fit_writer::{FitWriter, RecordSample};
use render::draw;
use std::io;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Build the loaded workout (if any) from a CLI-supplied path, or `None`.
fn resolve_workout(ftp: u16) -> Option<erg::Workout> {
    let arg = std::env::args().nth(1)?;
    let path = std::path::Path::new(&arg);
    if arg.ends_with(".erg") {
        match erg::load_erg_workout(path) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("Failed to load workout: {e}");
                None
            }
        }
    } else if arg.ends_with(".zwo") {
        match erg::parse_zwo_workout(path, ftp) {
            Ok(w) => Some(w),
            Err(e) => {
                eprintln!("Failed to load workout: {e}");
                None
            }
        }
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize the terminal (async)
    let mut terminal = init().await?;

    // Live data from the ride
    let livedata = LiveData::new();

    // Load the rider profile (JSON) instead of hardcoded values.
    let profile = data::load_profile();
    let stats = UserStats {
        maxhr: profile.max_hr,
        ftp: profile.ftp,
        _maxpwr: 0,
    };
    let userdata = UserData {
        username: profile.username.clone(),
        stats,
    };

    let workout_data = WorkoutData {
        duration: 0,
        total_distance: 0.0,
    };

    let mut app = App::new(livedata, userdata, workout_data);
    app.set_workout(resolve_workout(profile.ftp));

    let fps = Duration::from_secs_f64(1.0 / 60.0);

    // ---- BLE driver -------------------------------------------------------
    let ble::BleDriver {
        mut rx,
        cmd_tx,
        mut state_rx,
        ..
    } = ble::start_driver();

    // If a workout is active, slap its first target on the trainer immediately.
    if let Some(target) = app.workout().and_then(|w| w.step_at(0)).map(|s| s.target_power) {
        let _ = cmd_tx.send(ble::BleCommand::SetTargetPower(target)).await;
    }

    // Push an initial telemetry drop so the metrics/history have a baseline.
    app.push_power_history();
    app.push_hr_history();
    app.push_rpm_history();
    app.recompute_metrics(profile.ftp as f32, 1.0);

    // ---- Key events (blocking read moved off the async loop) ---------------
    let (key_tx, key_rx) = mpsc::channel();
    std::thread::spawn(move || loop {
        match crossterm::event::read() {
            Ok(Event::Key(k)) => {
                if key_tx.send(k).is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    });

    // ---- FIT recording -----------------------------------------------------
    let mut fit = FitWriter::new();
    let mut recording = false;

    let mut last_tick = Instant::now();

    loop {
        let frame_start = Instant::now();
        terminal.draw(|frame| draw(frame, &app))?;

        // Drain any state changes from the driver.
        while let Ok(state) = state_rx.try_recv() {
            let _ = state;
        }

        // Process incoming telemetry as it arrives (avoids blocking the loop).
        while let Ok(t) = rx.try_recv() {
            app.livedata.update(
                t.power.unwrap_or(0),
                t.cadence.unwrap_or(0),
                t.heart_rate.unwrap_or(0),
                t.speed.unwrap_or(0.0),
                0.0,
                0.0,
                0.0,
                0.0,
            );
            if !recording && (t.power.is_some() || t.speed.is_some()) {
                recording = true;
            }
        }

        // Handle user input.
        if let Ok(key) = key_rx.try_recv() {
            if key.kind == KeyEventKind::Press {
                if app.handle_key_press(key.code) == Action::Quit {
                    break;
                }
            }
        }

        // One-second metronome: advance the ride clock and update metrics.
        if last_tick.elapsed() >= Duration::from_secs(1) {
            last_tick = Instant::now();

            let target = app.tick_second();

            // Drive ERG when a workout is loaded; free ride keeps its own target.
            if app.workout().is_some() {
                let _ = cmd_tx.send(ble::BleCommand::SetTargetPower(target)).await;
            }

            app.accumulate_distance(1.0);
            app.push_power_history();
            app.push_hr_history();
            app.push_rpm_history();
            app.recompute_metrics(profile.ftp as f32, 1.0);

            // Record a FIT sample once per second.
            if recording || app.workout().is_some() {
                fit.push(RecordSample {
                    timestamp: Utc::now().timestamp(),
                    power: app.livedata.crnt_pwr,
                    cadence: app.livedata.crnt_rpm.min(255) as u8,
                    heart_rate: app.livedata.crnt_hr.min(255) as u8,
                    speed_mps: app.livedata.crnt_vel / 3.6,
                    distance_m: app.livedata.elapsed_distance * 1000.0,
                });
            }
        }

        // Pace the frame to ~60 fps.
        let elapsed = frame_start.elapsed();
        if elapsed < fps {
            std::thread::sleep(fps - elapsed);
        }
    }

    // ---- Cleanup: stop the driver, finalize the ride, persist. -------------
    let _ = cmd_tx.send(ble::BleCommand::Disconnect).await;
    let _ = rx.close();

    // Finalize FIT file.
    if !fit.is_empty() {
        std::fs::create_dir_all("data/.fit").ok();
        let stamp = Utc::now().format("%Y%m%d_%H%M%S");
        let fit_path = std::path::Path::new("data/.fit")
            .join(format!("ride_{stamp}.fit"));
        match fit.finish(&fit_path) {
            Ok(_) => log::info!("Wrote {}", fit_path.display()),
            Err(e) => log::error!("Failed to write FIT: {e}"),
        }

        // Persist a session summary to SQLite.
        if let Ok(conn) = data::init_db(std::path::Path::new("data/olympus.db")) {
            let avg_vel = if app.livedata.elapsed_secs > 0 {
                app.livedata.elapsed_distance
                    / (app.livedata.elapsed_secs as f32 / 3600.0)
            } else {
                0.0
            };
            let session = data::FitSession {
                total_distance: app.livedata.elapsed_distance,
                total_calories: app.livedata.calories,
                total_power: app.livedata.avg_pwr as f32,
                avg_speed: avg_vel,
                max_speed: app.livedata.max_vel,
                max_heart_rate: app.livedata.max_hr,
                avg_heart_rate: app.livedata.avg_hr,
                max_power: app.livedata.max_pwr,
                avg_power: app.livedata.avg_pwr,
                timestamp: Utc::now().timestamp(),
            };
            let _ = data::save_fit_session(&conn, &session, &fit_path.to_string_lossy());
        }
    }

    let _ = restore().await;
    Ok(())
}
