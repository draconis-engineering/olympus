// src/main.rs

mod app;
mod boot;
mod data;
mod rendering;

use app::{Action, App, LiveData, UserData};
use boot::init;
use crossterm::event::{Event, KeyEventKind, read};
use rendering::draw;
use std::io;

fn main() -> io::Result<()> {
    // Initialize the terminal
    let mut terminal = init()?;

    // Load fake data - Make a function of this and update inside the loop? async
    let livedata = LiveData {
        pwr: 1000,
        rpm: 95,
        hr: 208,
        vel: 50.5,
    };

    let userdata = UserData {
        user: "Simon Stordal Amundgård".to_string(),
        ltpwr: 250,
        maxhr: 208,
    };

    let mut app = App::new(livedata, userdata);

    // To implement before loop:
    // 1. Data fetching (JSON/SQL/other system) and populate App with data
    // 2. Start bluetooth client
    // 3. Try to connect to known bluetooth devices

    // To implement in this loop:
    // 1. Async/threaded data fetching from bluetooth client
    // 2. Bulletproof navigation between screens
    // 3. Event handling like quitting                        (DONE)
    // 4. Event handling like key presses                     (DONE)
    // 5. Resistance on roller
    // 6. Update livedata
    // 7. FPS limiter and counter (30-60fps, should be comfortable since Olympus only currently uses like 1mb of memory)
    // 8.

    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        if let Event::Key(key) = read()? {
            if key.kind == KeyEventKind::Press {
                let action = app.handle_key_press(key.code, app.screen());
                if action == Action::Quit {
                    break;
                }
            }
        }
    }

    Ok(())
}
