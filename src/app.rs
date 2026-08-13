// src/app.rs

use super::boot::restore;
use super::rendering::Data;

use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Stdout;
use std::ops::{Deref, DerefMut};

pub struct TUIGuard {
    pub tui: Terminal<CrosstermBackend<Stdout>>,
}

impl Drop for TUIGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}

impl Deref for TUIGuard {
    type Target = Terminal<CrosstermBackend<Stdout>>;
    fn deref(&self) -> &Terminal<CrosstermBackend<Stdout>> {
        &self.tui
    }
}

impl DerefMut for TUIGuard {
    fn deref_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.tui
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum Screen {
    #[default]
    Main, // Mainpage
    Control,  // Mission Control
    Database, // .FIT Database
    Settings, // Settings -> future json settings loaded with boot.rs
}

pub struct App {
    screen: Screen,
    data: Data,
}

#[derive(PartialEq)]
pub enum Action {
    Continue,
    Quit,
}

impl App {
    pub fn new(data: Data) -> Self {
        Self {
            data,
            screen: Screen::default(),
        }
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn data(&self) -> &Data {
        &self.data
    }

    // Pass the entire KeyEvent or just KeyCode, along with &mut self
    pub fn handle_key_press(&mut self, key_code: KeyCode) -> Action {
        match key_code {
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.screen = Screen::Main;
                Action::Continue
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.screen = Screen::Control;
                Action::Continue
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.screen = Screen::Database;
                Action::Continue
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.screen = Screen::Settings;
                Action::Continue
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            _ => Action::Continue,
        }
    }
}
