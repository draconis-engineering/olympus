// src/app.rs
//
// App.rs is the main application struct and entry point for the TUI.

use super::boot::restore;
use super::erg::Workout;
use super::math;
use super::nav::{MainSelection, Selections};

use crossterm::event::KeyCode;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::{Color, Style},
    text::Span,
};
use std::{
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
    pub tui: Terminal<CrosstermBackend<std::io::Stdout>>,
}
impl Drop for TUIGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}
impl Deref for TUIGuard {
    type Target = Terminal<CrosstermBackend<std::io::Stdout>>;
    fn deref(&self) -> &Terminal<CrosstermBackend<std::io::Stdout>> {
        &self.tui
    }
}
impl DerefMut for TUIGuard {
    fn deref_mut(&mut self) -> &mut Terminal<CrosstermBackend<std::io::Stdout>> {
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

/// Capacity (in samples) of the rolling power, heart rate, and cadence history buffers.
pub const POWER_HISTORY_CAPACITY: usize = 300;
pub const HR_HISTORY_CAPACITY: usize = 300;
pub const RPM_HISTORY_CAPACITY: usize = 300;

pub struct App {
    screen: Screen,
    selections: Selections,

    pub livedata: LiveData,
    userdata: UserData,
    workout_data: WorkoutData,
    preferences: Preferences,

    // Rolling histories used to render the live graphs in the
    // control panel. Capacity is fixed so the buffer never grows unbounded.
    pub power_history: Vec<u64>,
    pub hr_history: Vec<u16>,
    pub rpm_history: Vec<u16>,

    /// The currently loaded workout schedule (ERG/ZWO), if any.
    pub workout: Option<Workout>,
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
            power_history: Vec::with_capacity(POWER_HISTORY_CAPACITY),
            hr_history: Vec::with_capacity(HR_HISTORY_CAPACITY),
            rpm_history: Vec::with_capacity(RPM_HISTORY_CAPACITY),
            workout: None,
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

    /// Loads a workout schedule into the app. `None` clears it (free ride).
    pub fn set_workout(&mut self, workout: Option<Workout>) {
        self.workout = workout;
    }

    pub fn workout(&self) -> Option<&Workout> {
        self.workout.as_ref()
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

    /// Capacity of the rolling power history buffer (samples).
    pub const fn power_history_capacity(&self) -> usize {
        POWER_HISTORY_CAPACITY
    }

    /// Pushes the current power reading onto the rolling history buffer.
    pub fn push_power_history(&mut self) {
        if self.power_history.len() >= POWER_HISTORY_CAPACITY {
            self.power_history.remove(0);
        }
        self.power_history.push(self.livedata.crnt_pwr as u64);
    }

    pub fn power_history(&self) -> &[u64] {
        &self.power_history
    }

    /// Pushes the current heart rate onto the rolling history buffer.
    pub fn push_hr_history(&mut self) {
        if self.hr_history.len() >= HR_HISTORY_CAPACITY {
            self.hr_history.remove(0);
        }
        self.hr_history.push(self.livedata.crnt_hr);
    }

    pub fn hr_history(&self) -> impl Iterator<Item = u64> + '_ {
        self.hr_history.iter().map(|&x| u64::from(x))
    }

    /// Pushes the current cadence onto the rolling history buffer.
    pub fn push_rpm_history(&mut self) {
        if self.rpm_history.len() >= RPM_HISTORY_CAPACITY {
            self.rpm_history.remove(0);
        }
        self.rpm_history.push(self.livedata.crnt_rpm);
    }

    pub fn rpm_history(&self) -> impl Iterator<Item = u64> + '_ {
        self.rpm_history.iter().map(|&x| u64::from(x))
    }

    // Rolling window sizes (seconds) for the power averages shown on screen.
    const PWR_20MIN_S: usize = 1200;
    const PWR_10MIN_S: usize = 600;
    const PWR_5MIN_S: usize = 300;

    /// Recomputes the derived ride metrics (power/cadence/HR averages, rolling
    /// windows, NP, IF, TSS, kJ, calories) from the recorded history buffers.
    /// `sample_rate_hz` is how often history is pushed (usually ~1/s).
    pub fn recompute_metrics(&mut self, ftp: f32, sample_rate_hz: f64) {
        let sample_len = std::cmp::max(1, sample_rate_hz.round() as usize);

        let pwr_20_bins = Self::PWR_20MIN_S / sample_len;
        let pwr_10_bins = Self::PWR_10MIN_S / sample_len;
        let pwr_5_bins = Self::PWR_5MIN_S / sample_len;

        // Whole-ride averages.
        self.livedata.avg_pwr = math::rolling_mean(&self.power_history, self.power_history.len())
            .round() as u16;
        self.livedata.avg_rpm = math::rolling_mean(
            &self.rpm_history.iter().map(|&x| u64::from(x)).collect::<Vec<_>>(),
            usize::MAX,
        )
        .round() as u16;
        self.livedata.avg_hr = math::rolling_mean(
            &self.hr_history.iter().map(|&x| u64::from(x)).collect::<Vec<_>>(),
            usize::MAX,
        )
        .round() as u16;

        // Rolling power windows (best effort — uses whatever has accumulated).
        self.livedata.avg_20min_pwr =
            math::rolling_mean(&self.power_history, pwr_20_bins).round() as u16;
        self.livedata.avg_10min_pwr =
            math::rolling_mean(&self.power_history, pwr_10_bins).round() as u16;
        self.livedata.avg_5min_pwr =
            math::rolling_mean(&self.power_history, pwr_5_bins).round() as u16;

        // Maxes from history.
        self.livedata.max_pwr = self
            .power_history
            .iter()
            .copied()
            .max()
            .unwrap_or(0)
            .min(u16::MAX as u64) as u16;
        self.livedata.max_rpm = self
            .rpm_history
            .iter()
            .copied()
            .max()
            .unwrap_or(0);
        self.livedata.max_hr = self.hr_history.iter().copied().max().unwrap_or(0);

        // Performance metrics.
        let np = math::normalized_power(&self.power_history, sample_rate_hz);
        self.livedata.normalized_pwr = np as f32;
        let ifac = math::intensity_factor(np, ftp as f64);
        self.livedata.ifac = ifac as f32;
        self.livedata.tss =
            math::tss(np, ifac, ftp as f64, self.livedata.elapsed_secs as f64) as f32;

        let kj = math::energy_kj(self.livedata.avg_pwr as f64, self.livedata.elapsed_secs as f64);
        self.livedata.kj = kj as f32;
        self.livedata.calories = math::calories_kcal(kj) as f32;
    }

    /// Advances the elapsed ride clock by one second and, if a workout is
    /// active, resolves the new target power for the current interval.
    pub fn tick_second(&mut self) -> u16 {
        self.livedata.elapsed_secs += 1;
        let target = match &self.workout {
            Some(w) => w
                .step_at(self.livedata.elapsed_secs)
                .map(|s| s.target_power)
                .unwrap_or(0),
            None => self.livedata.target_pwr,
        };
        self.livedata.target_pwr = target;
        target
    }

    /// Integrates current speed into the accumulated distance (km).
    pub fn accumulate_distance(&mut self, dt_s: f64) {
        self.livedata.elapsed_distance +=
            math::distance_km(self.livedata.crnt_vel as f64, dt_s) as f32;
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
