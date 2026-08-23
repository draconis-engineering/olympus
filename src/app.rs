// src/app.rs

use super::boot::restore;

use crossterm::event::KeyCode;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Stdout;
use std::ops::{Deref, DerefMut};
use std::u16;

// Live data from device
pub struct LiveData {
    // Power
    pub crnt_pwr: u16,
    pub avg_20min_pwr: u16,
    pub avg_10min_pwr: u16,
    pub avg_5min_pwr: u16,
    pub avg_pwr: u16,
    pub top_pwr: u16,
    pub target_pwr: u16,
    // Cadence
    pub crnt_rpm: u16,
    pub avg_rpm: u16,
    pub top_rpm: u16,
    // Heart rate
    pub crnt_hr: u16,
    pub avg_hr: u16,
    pub top_hr: u16,
    // Velocity
    pub crnt_vel: f32,
    pub top_vel: f32,
    pub avg_vel: f32,
    // Gradient/Elevation
    pub gradient: f32,
    pub altitude: f32,
    pub elev_gain: f32,
    pub elev_loss: f32,
    // Calories
    pub calories: f32,
}

impl LiveData {
    pub fn new() -> Self {
        Self {
            // Power
            crnt_pwr: 0,
            avg_20min_pwr: 0,
            avg_10min_pwr: 0,
            avg_5min_pwr: 0,
            avg_pwr: 0,
            top_pwr: 0,
            target_pwr: 0,
            // Cadence
            crnt_rpm: 0,
            avg_rpm: 0,
            top_rpm: 0,
            // Heart rate
            crnt_hr: 0,
            avg_hr: 0,
            top_hr: 0,
            // Velocity
            crnt_vel: 0.0,
            avg_vel: 0.0,
            top_vel: 0.0,
            // Gradient/Elevation
            gradient: 0.0,
            altitude: 0.0,
            elev_gain: 0.0,
            elev_loss: 0.0,
            // Calories
            calories: 0.0,
        }
    }

    pub fn update(
        &mut self,
        pwr: u16,
        rpm: u16,
        hr: u16,
        velo: f32,
        gradient: f32,
        altitude: f32,
        elev_gain: f32,
        elev_loss: f32,
    ) {
        self.crnt_pwr = pwr;
        self.crnt_rpm = rpm;
        self.crnt_hr = hr;
        self.crnt_vel = velo;
        self.gradient = gradient;
        self.altitude = altitude;
        self.elev_gain = elev_gain;
        self.elev_loss = elev_loss;

        // Update averages and top values here
    }
}

pub struct WorkoutData {
    pub duration: u16,
    pub elapsed_time: u16,
    pub total_distance: f32,
    pub elapsed_distance: f32,
}

// Data for calculating things like power/hr zones, etc
pub struct UserData {
    pub user: String,
    pub ltpwr: u16,
    pub maxhr: u16,
    // Preferences under here
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
    Main,
    Control,
    Database,
    Settings,
    Stats,
}

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

#[derive(PartialEq)]
pub enum Action {
    Continue,
    Quit,
}

pub struct App {
    screen: Screen,
    pub livedata: LiveData,
    userdata: UserData,
    workout_data: WorkoutData,
    selections: Selections,
}
impl App {
    pub fn new(livedata: LiveData, userdata: UserData, workout_data: WorkoutData) -> Self {
        Self {
            livedata,
            userdata,
            workout_data,
            screen: Screen::default(),
            selections: Selections::new(),
        }
    }
    pub fn screen(&self) -> Screen {
        self.screen
    }
    pub fn livedata(&self) -> &LiveData {
        &self.livedata
    }
    pub fn userdata(&self) -> &UserData {
        &self.userdata
    }
    pub fn workout_data(&self) -> &WorkoutData {
        &self.workout_data
    }
    pub fn selections(&self) -> &Selections {
        &self.selections
    }

    pub fn version(&self) -> &str {
        "0.1.0"
    }
    pub fn devices(&self) -> &str {
        "Wahoo Kickr Core 2 - CONNECTED"
    }
    pub fn user(&self) -> &str {
        &self.userdata.user
    }
    pub fn connection(&self) -> &str {
        "CONNECTED - Wahoo Kickr Core 2"
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

            KeyCode::Enter => match self.screen() {
                Screen::Main => match self.selections().main() {
                    MainSelection::NewRide => Action::Continue,
                    MainSelection::Control => {
                        self.screen = Screen::Control;
                        Action::Continue
                    }
                    MainSelection::Workouts => {
                        self.screen = Screen::Database;
                        Action::Continue
                    }
                    MainSelection::Settings => {
                        self.screen = Screen::Settings;
                        Action::Continue
                    }
                    MainSelection::Stats => {
                        self.screen = Screen::Stats;
                        Action::Continue
                    }
                    MainSelection::Quit => Action::Quit,
                },
                _ => Action::Continue,
            },
            _ => Action::Continue,
        }
    }
}
