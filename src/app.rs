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

#[derive(Default, PartialEq, Clone, Copy)]
pub enum MainSelection {
    #[default]
    Main,
}
#[derive(Default, PartialEq, Clone, Copy)]
pub enum ControlSelection {
    #[default]
    Main,
}
#[derive(Default, PartialEq, Clone, Copy)]
pub enum DatabaseSelection {
    #[default]
    Main,
}
#[derive(Default, PartialEq, Clone, Copy)]
pub enum SettingsSelection {
    #[default]
    General,
    Appearance,
    Bluetooth,
    System,
}

#[derive(PartialEq)]
pub struct Selections {
    pub main_select: MainSelection,
    pub control_select: ControlSelection,
    pub database_select: DatabaseSelection,
    pub settings_select: SettingsSelection,
}
impl Selections {
    pub fn new() -> Self {
        Self {
            main_select: MainSelection::default(),
            control_select: ControlSelection::default(),
            database_select: DatabaseSelection::default(),
            settings_select: SettingsSelection::default(),
        }
    }
    pub fn main(&self) -> &MainSelection {
        &self.main_select
    }
    pub fn control(&self) -> &ControlSelection {
        &self.control_select
    }
    pub fn database(&self) -> &DatabaseSelection {
        &self.database_select
    }
    pub fn settings(&self) -> &SettingsSelection {
        &self.settings_select
    }
}

#[derive(PartialEq)]
pub enum Action {
    Continue,
    Quit,
}

pub struct App {
    screen: Screen,
    data: Data,
    selections: Selections,
}
impl App {
    pub fn new(data: Data) -> Self {
        Self {
            data,
            screen: Screen::default(),
            selections: Selections::new(),
        }
    }
    pub fn screen(&self) -> Screen {
        self.screen
    }
    pub fn data(&self) -> &Data {
        &self.data
    }
    pub fn selections(&self) -> &Selections {
        &self.selections
    }

    pub fn handle_key_press(&mut self, key_code: KeyCode, screen: Screen) -> Action {
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
            KeyCode::Char('^') => {
                // Correct symbol?
                match screen {
                    Screen::Main => Action::Continue, // No functionality for arrows yet
                    Screen::Control => Action::Continue, // No functionality for arrows yet
                    Screen::Database => Action::Continue, // No functionality for arrows yet
                    Screen::Settings => Action::Continue, // No functionality for arrows yet
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            _ => Action::Continue,
        }
    }
}
