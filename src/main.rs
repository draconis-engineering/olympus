// src/main.rs

mod app;
mod boot;
mod rendering;

use app::{Action, App, Data};
use boot::init;
use crossterm::event::{Event, KeyEventKind, read};
use rendering::draw;
use std::io;

fn main() -> io::Result<()> {
    // Initialize the terminal
    let mut terminal = init()?;

    // Load fake data - Make a function of this and update inside the loop? async
    let data = Data {
        pwr: 1000,
        ltpwr: 250,
        cadence: 95,
        hr: 208,
        speed: 50.5,
    };

    let mut app = App::new(data);

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
