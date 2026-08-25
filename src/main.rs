// src/main.rs
//
// Main entry point for the Olympus application.

mod app;
mod boot;
mod data;
mod math;
mod nav;
mod rendering;

use app::{Action, App, LiveData, UserData, UserStats, WorkoutData};
use boot::init;
use crossterm::event::{Event, KeyEventKind, read};
use rendering::draw;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

fn main() -> io::Result<()> {
    // == == == == == == //
    // -- INIT LOGIC --- //
    // == == == == == == //

    // To implement before loop:
    // 1. Data fetching (JSON/SQL/other system) and populate App with data
    // 2. Start bluetooth client
    // 3. Try to connect to known bluetooth devices
    // 4. Boot Screen
    // 5.

    // Initialize the terminal
    let mut terminal = init()?;

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

    // To implement in this loop:
    // 1. Async/threaded data fetching from bluetooth client
    // 2. Bulletproof navigation between screens
    // 3. Resistance on roller
    // 4. Update livedata
    // 5. Rolling calculations
    // 6. FPS Counter
    // 7. Threaded data writing/reading
    // 8. Post-exercise data processing and analysis
    // 9. Post-exercise data visualization
    // 10. Post-exercise data export

    // == == == == == == //
    // -- LOOP LOGIC --- //
    // == == == == == == //

    loop {
        framecount += 1;
        let frame_start = Instant::now();
        terminal.draw(|frame| draw(frame, &app))?;
        if let Event::Key(key) = read()? {
            if key.kind == KeyEventKind::Press {
                let action = app.handle_key_press(key.code);
                if action == Action::Quit {
                    break;
                }
            }
        }
        app.livedata.update(0, 0, 0, 0.0, 0.0, 0.0, 0.0, 0.0);

        let elapsed = frame_start.elapsed();
        if elapsed < fps {
            thread::sleep(fps - elapsed);
        }
        if framecount % 60 == 0 {
            framecount = 0;
            app.livedata.elapsed_secs += 1;
        }
    }

    Ok(())
}
