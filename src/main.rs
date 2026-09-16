/*
* Main entry point for the Olympus application.
* Copyright (C) 2026 Simon Stordal Amundgård
*
* This program is free software: you can redistribute it and/or modify
* it under the terms of the GNU General Public License as published by
* the Free Software Foundation, either version 3 of the License, or
* any later version.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
* GNU General Public License for more details.
*
* You should have received a copy of the GNU General Public License
* along with this program.  If not, see http://www.gnu.org/licenses.
*/

mod app;
mod ble;
mod boot;
mod data;
mod erg;
mod fit_writer;
mod math;
mod nav;
mod render;
mod strava;

use app::{Action, App, BleUiState, LiveData, UserData};
use ble::BleState;
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
    // `--help` / `--version` are handled before we get here.
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

/// Handle `--help` / `--version`. Returns `true` when the program should exit
/// immediately after printing (the TUI is not started).
fn handle_cli_args() -> bool {
    let mut args = std::env::args().skip(1);
    let Some(arg) = args.next() else {
        return false;
    };
    match arg.as_str() {
        "--help" | "-h" => {
            println!(
                "Olympus {} - a terminal cycling trainer\n\n\
                 USAGE:\n    olympus [OPTIONS] [WORKOUT]\n\n\
                 ARGS:\n    WORKOUT    Optional path to a .erg or .zwo workout file\n\n\
                 OPTIONS:\n    -h, --help       Print this help message\n    -V, --version    Print version information",
                app::App::version_static()
            );
            true
        }
        "--version" | "-V" => {
            println!("olympus {}", app::App::version_static());
            true
        }
        _ => false,
    }
}

/// Persist a finished ride to a FIT file and the SQLite history, or discard it.
/// Called from the main loop once the end-of-ride dialog picks an option.
/// Returns the saved FIT path on success so the caller can upload it.
fn finish_ride(
    app: &mut App,
    fit: &mut FitWriter,
    samples: &mut Vec<data::Sample>,
    save: bool,
) -> Option<std::path::PathBuf> {
    let mut saved_path: Option<std::path::PathBuf> = None;
    if save && !fit.is_empty() {
        app.has_ride_history = true;
        std::fs::create_dir_all("data/.fit").ok();
        let stamp = Utc::now().format("%Y%m%d_%H%M%S");
        let fit_path = std::path::Path::new("data/.fit").join(format!("ride_{stamp}.fit"));
        match fit.finish(&fit_path) {
            Ok(_) => {
                log::info!("Wrote {}", fit_path.display());
                saved_path = Some(fit_path.clone());
            }
            Err(e) => log::error!("Failed to write FIT: {e}"),
        }

        if let Ok(conn) = data::init_db(std::path::Path::new("data/olympus.db")) {
            let avg_vel = if app.livedata.elapsed_secs > 0 {
                app.livedata.elapsed_distance / (app.livedata.elapsed_secs as f32 / 3600.0)
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
            let path_str = saved_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            match data::save_ride(&conn, &session, &path_str, samples) {
                Ok(id) => log::info!("Saved ride #{id} with {} samples", samples.len()),
                Err(e) => log::error!("Failed to persist ride: {e}"),
            }
        }
    } else if !save {
        log::info!("Ride discarded");
    }

    // Reset the recording buffers for the next ride.
    *fit = FitWriter::new();
    samples.clear();
    saved_path
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize the logger first so log::info!/error! are visible instead of
    // silent (e.g. FIT write failures, BLE scan problems).
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    // `--help` / `--version` print and exit without entering the TUI.
    if handle_cli_args() {
        std::process::exit(0);
    }

    // Ensure the standard data directories exist so a fresh install has
    // somewhere for FIT files, the SQLite DB and bundled workouts.
    let _ = data::ensure_data_dirs();

    // Initialize the terminal (async)
    let mut terminal = init().await?;

    // Live data from the ride
    let livedata = LiveData::new();

    // Load the rider profile (JSON) instead of hardcoded values.
    // TODO: Load settings here
    let profile = data::load_profile();
    let userdata = UserData::new(profile.clone());

    let mut app = App::new(livedata, userdata);
    app.set_workout(resolve_workout(profile.ftp));
    app.refresh_strava();

    // Arriving with an existing session history turns off the new-rider
    // onboarding hint on Main (it lives for a truly fresh install).
    if let Ok(conn) = data::init_db(std::path::Path::new("data/olympus.db")) {
        app.has_ride_history = data::list_sessions(&conn, 1)
            .map(|s| !s.is_empty())
            .unwrap_or(false);
    }

    // Retry any queued Strava uploads from a previous offline session.
    {
        let queued = strava::load_queue().len();
        if queued > 0 {
            log::info!("strava: retrying {queued} queued upload(s) on boot");
        }
        tokio::spawn(async move {
            let n = strava::retry_queued_uploads().await;
            if n > 0 {
                log::info!("strava: uploaded {n} queued ride(s) on boot");
            }
        });
    }

    let fps = Duration::from_secs_f64(1.0 / 60.0);

    // ---- BLE driver -------------------------------------------------------
    let ble::BleDriver {
        mut rx,
        cmd_tx,
        mut state_rx,
        ..
    } = ble::start_driver();

    // If a workout is active, slap its first target on the trainer immediately.
    if let Some(target) = app
        .workout()
        .and_then(|w| w.step_at(0))
        .map(|s| s.target_power)
    {
        let _ = cmd_tx.send(ble::BleCommand::SetTargetPower(target)).await;
    }

    // Push an initial telemetry drop so the metrics/history have a baseline.
    app.push_power_history();
    app.push_hr_history();
    app.push_rpm_history();
    app.push_velocity_history();
    app.recompute_metrics(profile.ftp as f32, 1.0);

    // ---- Key events (blocking read moved off the async loop) ---------------
    let (key_tx, key_rx) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(Event::Key(k)) => {
                    if key_tx.send(k).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // ---- FIT recording -----------------------------------------------------
    let mut fit = FitWriter::new();
    // Per-second time-series samples for the analytics table.
    let mut samples: Vec<data::Sample> = Vec::new();

    let mut last_tick = Instant::now();

    loop {
        let frame_start = Instant::now();
        terminal.draw(|frame| draw(frame, &app))?;

        // Drain trainer connection-state changes into the app for display.
        while let Ok(state) = state_rx.try_recv() {
            match state {
                BleState::Idle => {
                    app.ble = BleUiState::Idle;
                    app.strap_name.clear();
                }
                BleState::Scanning => {
                    app.ble = BleUiState::Scanning;
                    app.strap_name.clear();
                }
                BleState::Connecting => {
                    app.ble = BleUiState::Connecting;
                    app.strap_name.clear();
                }
                BleState::Connected { trainer, strap } => {
                    app.ble = BleUiState::Connected;
                    app.trainer_name = trainer.unwrap_or_default();
                    app.strap_name = strap.unwrap_or_default();
                }
                BleState::Error(e) => {
                    app.ble = BleUiState::Error(e);
                    app.strap_name.clear();
                }
            }
        }

        // Process incoming telemetry as it arrives (avoids blocking the loop).
        // Telemetry is emitted per-characteristic, so any single sample may
        // carry only some fields (e.g. strap HR, or trainer power + cadence).
        // We merge rather than overwrite, keeping the last-known value of every
        // other field — HR never flickers to 0 between power notifications.
        while let Ok(t) = rx.try_recv() {
            let has_data = t.power.is_some()
                || t.cadence.is_some()
                || t.heart_rate.is_some()
                || t.speed.is_some();
            if !has_data {
                continue;
            }
            app.livedata.update(
                t.power.unwrap_or(app.livedata.crnt_pwr),
                t.cadence.unwrap_or(app.livedata.crnt_rpm),
                t.heart_rate.unwrap_or(app.livedata.crnt_hr),
                t.speed.unwrap_or(app.livedata.crnt_vel),
                0.0,
                0.0,
                0.0,
                0.0,
            );
        }

        // Handle user input.
        if let Ok(key) = key_rx.try_recv() {
            if key.kind == KeyEventKind::Press {
                match app.handle_key_press(key.code) {
                    Action::Quit => break,
                    // Settings → Bluetooth → Enter: tell the driver to drop the
                    // current trainer and scan for a new one.
                    Action::Scan => {
                        let _ = cmd_tx.send(ble::BleCommand::Scan).await;
                    }
                    Action::Continue => {}
                }
            }
        }

        // When the end-of-ride dialog picks Save or Discard, persist/clear the
        // recording here (main owns the FIT writer and the sample buffer).
        if let Some(save) = app.pending_save.take() {
            let saved_path = finish_ride(&mut app, &mut fit, &mut samples, save);
            // The session history changed; force the Database tab to rescan.
            app.database.loaded = false;
            // Stats aggregates changed too; recompute on next visit.
            app.stats.loaded = false;

            // Strava auto-upload: on Save, push the FIT to Strava or queue
            // for retry. Fire-and-forget so the UI stays snappy.
            if let Some(fit_path) = saved_path {
                let activity_name = app
                    .last_workout_name
                    .clone()
                    .unwrap_or_else(|| "Olympus Ride".to_string());
                if strava::is_connected() {
                    app.strava_status = Some("Uploading to Strava...".to_string());
                } else {
                    // No token yet — queue for later; Settings → Strava explains.
                    let _ = strava::enqueue_fit(&fit_path);
                    app.strava_status =
                        Some("FIT saved — Strava not connected (queued).".to_string());
                }
                let fit_path_clone = fit_path.clone();
                tokio::spawn(async move {
                    match strava::try_upload_or_queue(&fit_path_clone, Some(&activity_name)).await
                    {
                        Ok(true) => log::info!("strava: upload ok"),
                        Ok(false) => log::info!("strava: queued for retry"),
                        Err(e) => log::error!("strava: upload error: {e}"),
                    }
                });
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

            // Only accrue distance / record samples while a ride is live.
            if app.is_recording() {
                app.accumulate_distance(1.0);
                app.push_power_history();
                app.push_hr_history();
                app.push_rpm_history();
                app.push_velocity_history();
                app.recompute_metrics(profile.ftp as f32, 1.0);

                fit.push(RecordSample {
                    timestamp: Utc::now().timestamp(),
                    power: app.livedata.crnt_pwr,
                    cadence: app.livedata.crnt_rpm.min(255) as u8,
                    heart_rate: app.livedata.crnt_hr.min(255) as u8,
                    speed_mps: app.livedata.crnt_vel / 3.6,
                    distance_m: app.livedata.elapsed_distance * 1000.0,
                });
                samples.push(data::Sample {
                    t: Utc::now().timestamp(),
                    power: app.livedata.crnt_pwr,
                    cadence: app.livedata.crnt_rpm.min(255) as u16,
                    heart_rate: app.livedata.crnt_hr.min(255) as u16,
                    speed: app.livedata.crnt_vel / 3.6,
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

    // Finalize any leftover FIT file (e.g. quit without an explicit save).
    if !fit.is_empty() {
        std::fs::create_dir_all("data/.fit").ok();
        let stamp = Utc::now().format("%Y%m%d_%H%M%S");
        let fit_path = std::path::Path::new("data/.fit").join(format!("ride_{stamp}.fit"));
        match fit.finish(&fit_path) {
            Ok(_) => log::info!("Wrote {}", fit_path.display()),
            Err(e) => log::error!("Failed to write FIT: {e}"),
        }
    }

    let _ = restore().await;
    Ok(())
}
