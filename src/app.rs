// src/app.rs

use super::boot::restore;

use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Stdout;
use std::ops::{Deref, DerefMut};

pub struct Data {
    pub pwr: u16,
    pub ltpwr: u16,
    pub cadence: u16,
    pub hr: u16,
    pub speed: f32,
}

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

impl MainSelection {
    pub fn next(&mut self) {
        *self = match *self {
            MainSelection::Main => MainSelection::Main,
        };
    }
    pub fn prev(&mut self) {
        *self = match *self {
            MainSelection::Main => MainSelection::Main,
        };
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum ControlSelection {
    #[default]
    Main,
}

impl ControlSelection {
    pub fn next(&mut self) {
        *self = match *self {
            ControlSelection::Main => ControlSelection::Main,
        };
    }
    pub fn prev(&mut self) {
        *self = match *self {
            ControlSelection::Main => ControlSelection::Main,
        };
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum DatabaseSelection {
    #[default]
    Main,
}

impl DatabaseSelection {
    pub fn next(&mut self) {
        *self = match *self {
            DatabaseSelection::Main => DatabaseSelection::Main,
        };
    }
    pub fn prev(&mut self) {
        *self = match *self {
            DatabaseSelection::Main => DatabaseSelection::Main,
        };
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum SettingsSelection {
    #[default]
    General,
    Appearance,
    Bluetooth,
    System,
    User,
}

impl SettingsSelection {
    pub fn next(&mut self) {
        *self = match *self {
            SettingsSelection::General => SettingsSelection::Appearance,
            SettingsSelection::Appearance => SettingsSelection::Bluetooth,
            SettingsSelection::Bluetooth => SettingsSelection::System,
            SettingsSelection::System => SettingsSelection::User,
            SettingsSelection::User => SettingsSelection::General,
        };
    }
    pub fn prev(&mut self) {
        *self = match *self {
            SettingsSelection::General => SettingsSelection::User,
            SettingsSelection::Appearance => SettingsSelection::General,
            SettingsSelection::Bluetooth => SettingsSelection::Appearance,
            SettingsSelection::System => SettingsSelection::Bluetooth,
            SettingsSelection::User => SettingsSelection::System,
        };
    }
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
    //pub fn main(&self) -> &MainSelection {
    //    &self.main_select
    //}
    //pub fn control(&self) -> &ControlSelection {
    //    &self.control_select
    //}
    //pub fn database(&self) -> &DatabaseSelection {
    //    &self.database_select
    //}
    pub fn settings(&self) -> &SettingsSelection {
        &self.settings_select
    }
    pub fn next(&mut self, screen: Screen) {
        match screen {
            Screen::Main => self.main_select.next(),
            Screen::Control => self.control_select.next(),
            Screen::Database => self.database_select.next(),
            Screen::Settings => self.settings_select.next(),
        }
    }
    pub fn prev(&mut self, screen: Screen) {
        match screen {
            Screen::Main => self.main_select.prev(),
            Screen::Control => self.control_select.prev(),
            Screen::Database => self.database_select.prev(),
            Screen::Settings => self.settings_select.prev(),
        }
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
            KeyCode::Up => {
                self.selections.prev(screen);
                Action::Continue
            }
            KeyCode::Down => {
                self.selections.next(screen);
                Action::Continue
            }

            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
            _ => Action::Continue,
        }
    }
}
