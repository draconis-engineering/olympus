// src/main.rs

mod boot;
mod guard;
mod rendering;

use boot::init;
use crossterm::event::{Event, KeyCode, KeyEventKind, read};
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

    loop {
        terminal.draw(|frame| draw(frame, &data))?;
        if let Event::Key(key) = read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                break;
            }
        }
    }
    Ok(())
}
