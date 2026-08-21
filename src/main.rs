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

    loop {
        terminal.draw(|frame| draw(frame, &app, app.selections()))?;
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
