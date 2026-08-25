// src/nav.rs

// Navigation System for Olympus TUI

use super::app::Screen;

#[derive(Default, PartialEq, Clone, Copy)]
pub enum MainSelection {
    #[default]
    NewRide,
    Control,
    Workouts,
    Settings,
    Stats,
    Quit,
}
impl MainSelection {
    pub fn next(&mut self) {
        *self = match *self {
            MainSelection::NewRide => MainSelection::Control,
            MainSelection::Control => MainSelection::Workouts,
            MainSelection::Workouts => MainSelection::Settings,
            MainSelection::Settings => MainSelection::Stats,
            MainSelection::Stats => MainSelection::Quit,
            MainSelection::Quit => MainSelection::NewRide,
        };
    }
    pub fn prev(&mut self) {
        *self = match *self {
            MainSelection::NewRide => MainSelection::Quit,
            MainSelection::Control => MainSelection::NewRide,
            MainSelection::Workouts => MainSelection::Control,
            MainSelection::Settings => MainSelection::Workouts,
            MainSelection::Stats => MainSelection::Settings,
            MainSelection::Quit => MainSelection::Stats,
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

#[derive(Default, PartialEq, Clone, Copy)]
pub enum StatsSelection {
    #[default]
    Overview,
    Rides,
}
impl StatsSelection {
    pub fn next(&mut self) {
        *self = match *self {
            StatsSelection::Overview => StatsSelection::Rides,
            StatsSelection::Rides => StatsSelection::Overview,
        };
    }
    pub fn prev(&mut self) {
        *self = match *self {
            StatsSelection::Overview => StatsSelection::Rides,
            StatsSelection::Rides => StatsSelection::Overview,
        };
    }
}

#[derive(PartialEq)]
pub struct Selections {
    pub main_select: MainSelection,
    pub control_select: ControlSelection,
    pub database_select: DatabaseSelection,
    pub settings_select: SettingsSelection,
    pub stats_select: StatsSelection,
}
impl Selections {
    pub fn new() -> Self {
        Self {
            main_select: MainSelection::default(),
            control_select: ControlSelection::default(),
            database_select: DatabaseSelection::default(),
            settings_select: SettingsSelection::default(),
            stats_select: StatsSelection::default(),
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
    pub fn stats(&self) -> &StatsSelection {
        &self.stats_select
    }
    pub fn next(&mut self, screen: Screen) {
        match screen {
            Screen::Main => self.main_select.next(),
            Screen::Control => self.control_select.next(),
            Screen::Database => self.database_select.next(),
            Screen::Settings => self.settings_select.next(),
            Screen::Stats => self.stats_select.next(),
        }
    }
    pub fn prev(&mut self, screen: Screen) {
        match screen {
            Screen::Main => self.main_select.prev(),
            Screen::Control => self.control_select.prev(),
            Screen::Database => self.database_select.prev(),
            Screen::Settings => self.settings_select.prev(),
            Screen::Stats => self.stats_select.prev(),
        }
    }
}

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub trait Navigable: Sized {
    fn neighbor(&self, dir: Direction) -> Option<Self>;
}

impl Navigable for MainSelection {
    fn neighbor(&self, dir: Direction) -> Option<Self> {
        match dir {
            Direction::Down => None,
            Direction::Up => None,
            Direction::Left | Direction::Right => None,
        }
    }
}
