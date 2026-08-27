// src/main.rs
//
// Main entry point for the Olympus application.

mod app;
mod boot;
mod data;
mod math;
mod nav;
mod render;
mod ble;
mod erg;

use app::{Action, App, LiveData, UserData, UserStats, WorkoutData};
use boot::{init, restore};
use crossterm::event::{Event, KeyEventKind, read};
use render::draw;
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize the terminal (async)
    let mut terminal = init().await?;

    // Load fake data - Make a function of this
    let livedata = LiveData::new();

    let stats = UserStats {
        maxhr: 180,
        _maxpwr: 180,
        ftp: 180,
    };

    let userdata = UserData {
        username: "Simon Stordal Amundgård".to_string(),
        stats: stats,
    };

    let workout_data = WorkoutData {
        duration: 3600,
        total_distance: 160.0,
    };

    let mut app = App::new(livedata, userdata, workout_data);
    let fps = Duration::from_secs_f64(1.0 / 60.0);
    let mut framecount = 0;

    // BLE channel for receiving trainer data
    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();

    // Spawn BLE worker thread (sync thread, sends to async channel)
    let ble_handle = thread::spawn(move || {
        // TODO: Initialize btleplug here and start scanning/connecting
        // For now, simulate data
        loop {
            // Simulate receiving power/cadence/hr data
            let _ = tx.send((
                200,      // power
                90,       // rpm
                150,      // heart rate
                5.2,      // speed
            ));
            thread::sleep(Duration::from_millis(1000));
        }
    });

    // == == == == == == //
    // -- LOOP LOGIC --- //
    // == == == == == == //

    loop {
        framecount += 1;
        let frame_start = Instant::now();
        terminal.draw(|frame| draw(frame, &app))?;

        // Receive BLE data
        if let Ok(data) = rx.try_recv() {
            let (pwr, rpm, hr, vel) = data;
            app.livedata.update(pwr, rpm, hr, vel, 0.0, 0.0, 0.0, 0.0);
            app.push_power_history();
            app.push_hr_history();
        }

        if let Event::Key(key) = read()? {
            if key.kind == KeyEventKind::Press {
                let action = app.handle_key_press(key.code);
                if action == Action::Quit {
                    break;
                }
            }
        }

        app.push_power_history();
        app.push_hr_history();

        let elapsed = frame_start.elapsed();
        if elapsed < fps {
            thread::sleep(fps - elapsed);
        }
        if framecount % 60 == 0 {
            framecount = 0;
            app.livedata.elapsed_secs += 1;
        }
    }

    // Cleanup
    let _ = restore().await;
    let _ = ble_handle.join();

    Ok(())
}