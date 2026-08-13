// src/main.rs

mod app;
mod boot;
mod rendering;

use app::{Action, App};
use boot::init;
use crossterm::event::{Event, KeyEventKind, read};
use rendering::{Data, draw};
use std::io;

fn main() -> io::Result<()> {
    let mut terminal = init()?;

    let data = Data {
        power: 300,
        cadence: 95,
        hr: 180,
        speed: 42.5,
    };

    let mut app = App::new(data);

    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        if let Event::Key(key) = read()? {
            if key.kind == KeyEventKind::Press {
                let action = app.handle_key_press(key.code);
                if action == Action::Quit {
                    break;
                }
            }
        }
    }

    Ok(())
}
