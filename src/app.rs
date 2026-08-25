// src/app.rs

use super::boot::restore;
use super::nav::{MainSelection, Selections};

use crossterm::event::KeyCode;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::{Color, Style},
    text::Span,
};
use std::{
    io::Stdout,
    ops::{Deref, DerefMut},
    u16,
};

// Live data from device
pub struct LiveData {
    // Power
    pub crnt_pwr: u16,
    pub avg_20min_pwr: u16,
    pub avg_10min_pwr: u16,
    pub avg_5min_pwr: u16,
    pub avg_pwr: u16,
    pub max_pwr: u16,
    pub target_pwr: u16,
    pub normalized_pwr: f32,
    // Cadence
    pub crnt_rpm: u16,
    pub avg_rpm: u16,
    pub max_rpm: u16,
    pub target_rpm: u16,
    // Heart rate
    pub crnt_hr: u16,
    pub avg_hr: u16,
    pub max_hr: u16,
    pub target_hrz: u16,
    // Velocity
    pub crnt_vel: f32,
    pub max_vel: f32,
    pub avg_vel: f32,
    pub target_vel: f32,
    // Gradient/Elevation
    pub grad: f32,
    pub alti: f32,
    pub egain: f32,
    pub eloss: f32,
    // Energy
    pub calories: f32,
    pub kj: f32,
    // TSS / IF
    pub tss: f32,
    pub ifac: f32,
    // Elapsed
    pub elapsed_secs: u32,
    pub elapsed_distance: f32,
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
            max_pwr: 0,
            target_pwr: 0,
            normalized_pwr: 0.0,
            // Cadence
            crnt_rpm: 0,
            avg_rpm: 0,
            max_rpm: 0,
            target_rpm: 0,
            // Heart rate
            crnt_hr: 0,
            avg_hr: 0,
            max_hr: 0,
            target_hrz: 0,
            // Velocity
            crnt_vel: 0.0,
            avg_vel: 0.0,
            max_vel: 0.0,
            target_vel: 0.0,
            // Gradient/Elevation
            grad: 0.0,
            alti: 0.0,
            egain: 0.0,
            eloss: 0.0,
            // Calories
            kj: 0.0,
            calories: 0.0,
            // TSS / IF
            tss: 0.0,
            ifac: 0.0,
            // Elapsed
            elapsed_secs: 0,
            elapsed_distance: 0.0,
        }
    }

    pub fn update(
        &mut self,
        pwr: u16,
        rpm: u16,
        hr: u16,
        vel: f32,
        grad: f32,
        alti: f32,
        egain: f32,
        eloss: f32,
    ) {
        self.crnt_pwr = pwr;
        self.crnt_rpm = rpm;
        self.crnt_hr = hr;
        self.crnt_vel = vel;
        self.grad = grad;
        self.alti = alti;
        self.egain = egain;
        self.eloss = eloss;

        if pwr > self.max_pwr {
            self.max_pwr = pwr;
        }
        if hr > self.max_hr {
            self.max_hr = hr;
        }
        if vel > self.max_vel {
            self.max_vel = vel;
        }
    }
}

pub struct WorkoutData {
    pub duration: u16,
    pub total_distance: f32,
}

pub struct UserStats {
    pub maxhr: u16,

    pub ftp: u16,
    pub _maxpwr: u16,
}
pub struct UserData {
    pub username: String,
    pub stats: UserStats,
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

#[derive(PartialEq)]
pub enum Action {
    Continue,
    Quit,
}

pub struct Preferences {
    pub dark_mode: bool,
    pub high_contrast: bool,
}
impl Preferences {
    fn new() -> Self {
        Self {
            dark_mode: false,
            high_contrast: false,
        }
    }
}

pub struct App {
    screen: Screen,
    selections: Selections,

    pub livedata: LiveData,
    userdata: UserData,
    workout_data: WorkoutData,
    preferences: Preferences,
}
impl App {
    pub fn new(livedata: LiveData, userdata: UserData, workout_data: WorkoutData) -> Self {
        Self {
            livedata,
            userdata,
            workout_data,
            screen: Screen::default(),
            preferences: Preferences::new(),
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

    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    pub fn version(&self) -> &str {
        "0.1.5"
    }
    pub fn user(&self) -> &str {
        &self.userdata.username
    }
    pub fn connection(&self) -> Vec<Span<'_>> {
        vec![
            Span::from("Tacx Flux S2"),
            Span::from("CONNECTED").style(Style::default().fg(Color::Green)),
        ]
    }

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
            KeyCode::Up => {
                self.selections.prev(self.screen);
                Action::Continue
            }
            KeyCode::Down => {
                self.selections.next(self.screen);
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
