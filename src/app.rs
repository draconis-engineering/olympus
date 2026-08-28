// src/app.rs
//
// App.rs is the main application struct and entry point for the TUI.

use super::boot::restore;
use super::data::UserProfile;
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
    // Time (seconds) spent in each of the 7 Coggan power zones.
    pub zone_seconds: [u64; 7],
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
            // Zone time
            zone_seconds: [0; 7],
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

pub struct UserData {
    pub profile: UserProfile,
}
impl UserData {
    pub fn new(profile: UserProfile) -> Self {
        Self { profile }
    }
    pub fn username(&self) -> &str {
        &self.profile.username
    }
    pub fn ftp(&self) -> u16 {
        self.profile.ftp
    }
    pub fn maxhr(&self) -> u16 {
        self.profile.max_hr
    }
    pub fn weight(&self) -> f32 {
        self.profile.weight
    }
    pub fn height(&self) -> f32 {
        self.profile.height
    }
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
pub const VEL_HISTORY_CAPACITY: usize = 300;

/// Which list is active in the Database screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseTab {
    Workouts,
    Sessions,
}

/// Cursor state for the Database (workout / session) screen.
pub struct DatabaseState {
    pub tab: DatabaseTab,
    /// Index of the highlighted row within the active list.
    pub selected: usize,
    /// Loaded workout files, in display order.
    pub workouts: Vec<crate::data::WorkoutEntry>,
    /// Stored session history, newest first.
    pub sessions: Vec<crate::data::StoredSession>,
    /// Set once the data has been loaded, so we don't rescan every frame.
    pub loaded: bool,
}
impl Default for DatabaseState {
    fn default() -> Self {
        Self {
            tab: DatabaseTab::Workouts,
            selected: 0,
            workouts: Vec::new(),
            sessions: Vec::new(),
            loaded: false,
        }
    }
}
impl DatabaseState {
    fn len(&self) -> usize {
        match self.tab {
            DatabaseTab::Workouts => self.workouts.len(),
            DatabaseTab::Sessions => self.sessions.len(),
        }
    }
    /// Move the cursor by `delta` (-1 / +1), clamped to the list bounds. Safe
    /// even when a list is empty.
    pub fn move_cursor(&mut self, delta: isize) {
        let len = self.len();
        if len == 0 {
            self.selected = 0;
            return;
        }
        let next = self.selected as isize + delta;
        self.selected = next.clamp(0, len as isize - 1) as usize;
    }
    pub fn selected_workout(&self) -> Option<&crate::data::WorkoutEntry> {
        self.workouts.get(self.selected)
    }
    pub fn selected_session(&self) -> Option<&crate::data::StoredSession> {
        self.sessions.get(self.selected)
    }
    fn ensure_selected_in_range(&mut self) {
        let len = self.len();
        if len > 0 && self.selected >= len {
            self.selected = len - 1;
        }
    }
}

/// Which rider profile field is being edited in the Settings screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Name,
    Weight,
    Height,
    Ftp,
    MaxHr,
}
impl SettingsField {
    const ALL: [SettingsField; 5] = [
        SettingsField::Name,
        SettingsField::Weight,
        SettingsField::Height,
        SettingsField::Ftp,
        SettingsField::MaxHr,
    ];
    pub fn next(self) -> Self {
        let cur = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
        Self::ALL[(cur + 1) % Self::ALL.len()]
    }
    pub fn prev(self) -> Self {
        let cur = Self::ALL.iter().position(|v| *v == self).unwrap_or(0);
        Self::ALL[(cur + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Cursor/editor state for the Settings screen.
pub struct SettingsState {
    pub field: SettingsField,
    /// Whether the editor for the active field is open.
    pub editing: bool,
    /// Text buffer while editing.
    pub draft: String,
    /// Whether the profile has unsaved changes to persist.
    pub dirty: bool,
}
impl Default for SettingsState {
    fn default() -> Self {
        Self {
            field: SettingsField::Name,
            editing: false,
            draft: String::new(),
            dirty: false,
        }
    }
}
impl SettingsState {
    /// Draw the current value of a profile field into the editor buffer when
    /// editing starts.
    pub fn begin_edit(&mut self, profile: &UserProfile) {
        self.draft = match self.field {
            SettingsField::Name => profile.username.clone(),
            SettingsField::Weight => format!("{:.1}", profile.weight),
            SettingsField::Height => format!("{:.1}", profile.height),
            SettingsField::Ftp => format!("{}", profile.ftp),
            SettingsField::MaxHr => format!("{}", profile.max_hr),
        };
        self.editing = true;
    }
    /// Parse the buffer back into the profile, clamping to sane ranges.
    pub fn commit_edit(&mut self, profile: &mut UserProfile) {
        match self.field {
            SettingsField::Name => {
                if !self.draft.trim().is_empty() {
                    profile.username = self.draft.trim().to_string();
                }
            }
            SettingsField::Weight => {
                if let Ok(v) = self.draft.trim().parse::<f32>() {
                    profile.weight = v.clamp(30.0, 300.0);
                }
            }
            SettingsField::Height => {
                if let Ok(v) = self.draft.trim().parse::<f32>() {
                    profile.height = v.clamp(120.0, 230.0);
                }
            }
            SettingsField::Ftp => {
                if let Ok(v) = self.draft.trim().parse::<u16>() {
                    profile.ftp = v.clamp(40, 600);
                }
            }
            SettingsField::MaxHr => {
                if let Ok(v) = self.draft.trim().parse::<u16>() {
                    profile.max_hr = v.clamp(80, 250);
                }
            }
        }
        self.editing = false;
        self.draft.clear();
    }
    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.draft.clear();
    }
}

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
    pub vel_history: Vec<f32>,

    /// The currently loaded workout schedule (ERG/ZWO), if any.
    pub workout: Option<Workout>,

    /// Database screen state.
    pub database: DatabaseState,
    /// Settings screen state.
    pub settings: SettingsState,
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
            vel_history: Vec::with_capacity(VEL_HISTORY_CAPACITY),
            workout: None,
            database: DatabaseState::default(),
            settings: SettingsState::default(),
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
        &self.userdata.profile.username
    }

    /// Mutable access to the rider profile (used by the Settings editor).
    pub fn profile_mut(&mut self) -> &mut UserProfile {
        &mut self.userdata.profile
    }

    /// Persist the current rider profile back to JSON.
    pub fn save_profile(&self) -> Result<(), String> {
        crate::data::save_profile(&self.userdata.profile)
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

    /// Pushes the current road speed onto the rolling history buffer.
    pub fn push_velocity_history(&mut self) {
        if self.vel_history.len() >= VEL_HISTORY_CAPACITY {
            self.vel_history.remove(0);
        }
        self.vel_history.push(self.livedata.crnt_vel);
    }

    pub fn vel_history(&self) -> &[f32] {
        &self.vel_history
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
        self.livedata.avg_pwr =
            math::rolling_mean(&self.power_history, self.power_history.len()).round() as u16;
        self.livedata.avg_rpm = math::rolling_mean(
            &self
                .rpm_history
                .iter()
                .map(|&x| u64::from(x))
                .collect::<Vec<_>>(),
            usize::MAX,
        )
        .round() as u16;
        self.livedata.avg_hr = math::rolling_mean(
            &self
                .hr_history
                .iter()
                .map(|&x| u64::from(x))
                .collect::<Vec<_>>(),
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
        self.livedata.max_rpm = self.rpm_history.iter().copied().max().unwrap_or(0);
        self.livedata.max_hr = self.hr_history.iter().copied().max().unwrap_or(0);

        // Performance metrics.
        let np = math::normalized_power(&self.power_history, sample_rate_hz);
        self.livedata.normalized_pwr = np as f32;
        let ifac = math::intensity_factor(np, ftp as f64);
        self.livedata.ifac = ifac as f32;
        self.livedata.tss =
            math::tss(np, ifac, ftp as f64, self.livedata.elapsed_secs as f64) as f32;

        let kj = math::energy_kj(
            self.livedata.avg_pwr as f64,
            self.livedata.elapsed_secs as f64,
        );
        self.livedata.kj = kj as f32;
        self.livedata.calories = math::calories_kcal(kj) as f32;

        // Average velocity from accumulated distance and time.
        if self.livedata.elapsed_secs > 0 {
            let hours = self.livedata.elapsed_secs as f32 / 3600.0;
            self.livedata.avg_vel = self.livedata.elapsed_distance / hours;
        } else {
            self.livedata.avg_vel = 0.0;
        }
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
        // Attribute one second of riding to the current Coggan zone.
        let zone = math::coggan_pwr_model(self.livedata.crnt_pwr, self.userdata.ftp());
        if (1..=7).contains(&zone) {
            let idx = (zone - 1) as usize;
            self.livedata.zone_seconds[idx] += 1;
        }
        target
    }

    /// Integrates current speed into the accumulated distance (km).
    pub fn accumulate_distance(&mut self, dt_s: f64) {
        self.livedata.elapsed_distance +=
            math::distance_km(self.livedata.crnt_vel as f64, dt_s) as f32;
    }

    /// Populate the Database screen lists (workouts + session history).
    pub fn load_database(&mut self) {
        if self.database.loaded {
            return;
        }
        self.database.workouts = crate::data::list_workout_files();
        if let Ok(conn) = crate::data::init_db(std::path::Path::new("data/olympus.db")) {
            if let Ok(sessions) = crate::data::list_sessions(&conn, 50) {
                self.database.sessions = sessions;
            }
        }
        self.database.ensure_selected_in_range();
        self.database.loaded = true;
    }

    /// Load the currently highlighted workout into the ride and return whether
    /// it succeeded (i.e. the ride can start).
    pub fn start_selected_workout(&mut self) -> bool {
        let Some(entry) = self.database.selected_workout() else {
            return false;
        };
        let path = std::path::Path::new(&entry.path);
        let ftp = self.userdata.ftp();
        let workout = if entry.path.to_lowercase().ends_with(".zwo") {
            crate::erg::parse_zwo_workout(path, ftp).ok()
        } else {
            crate::erg::load_erg_workout(path).ok()
        };
        match workout {
            Some(w) => {
                self.set_workout(Some(w));
                true
            }
            None => false,
        }
    }

    /// Arrow-key menu navigation plus Enter handling for the Main screen.
    fn handle_nav_key(&mut self, key_code: KeyCode) -> Action {
        match key_code {
            KeyCode::Up => {
                self.selections.prev(self.screen);
                Action::Continue
            }
            KeyCode::Down => {
                self.selections.next(self.screen);
                Action::Continue
            }
            KeyCode::Enter => match self.screen {
                Screen::Main => match *self.selections().main() {
                    MainSelection::NewRide => Action::Continue,
                    MainSelection::Control => {
                        self.screen = Screen::Control;
                        Action::Continue
                    }
                    MainSelection::Workouts => {
                        self.screen = Screen::Database;
                        self.load_database();
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

    /// Key handling for the Database (workouts + sessions) screen.
    fn handle_database_key(&mut self, key_code: KeyCode) -> Action {
        if !self.database.loaded {
            self.load_database();
        }
        match key_code {
            KeyCode::Up => {
                self.database.move_cursor(-1);
                Action::Continue
            }
            KeyCode::Down => {
                self.database.move_cursor(1);
                Action::Continue
            }
            KeyCode::Left | KeyCode::Tab => {
                self.database.tab = DatabaseTab::Sessions;
                self.database.ensure_selected_in_range();
                Action::Continue
            }
            KeyCode::Right | KeyCode::BackTab => {
                self.database.tab = DatabaseTab::Workouts;
                self.database.ensure_selected_in_range();
                Action::Continue
            }
            KeyCode::Enter => match self.database.tab {
                DatabaseTab::Workouts => {
                    // Load the highlighted workout and jump to the Control panel.
                    if self.start_selected_workout() {
                        self.screen = Screen::Control;
                    }
                    Action::Continue
                }
                DatabaseTab::Sessions => Action::Continue,
            },
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.screen = Screen::Main;
                Action::Continue
            }
            _ => Action::Continue,
        }
    }

    /// Key handling for the Settings screen (rider-profile editor).
    fn handle_settings_key(&mut self, key_code: KeyCode) -> Action {
        use super::nav::SettingsSelection;

        // The profile editor only lives on the "User" panel.
        if *self.selections.settings() != SettingsSelection::User {
            return match key_code {
                KeyCode::Up => {
                    self.selections.prev(self.screen);
                    Action::Continue
                }
                KeyCode::Down => {
                    self.selections.next(self.screen);
                    Action::Continue
                }
                _ => Action::Continue,
            };
        }

        // On the User panel: editing an active field.
        if self.settings.editing {
            match key_code {
                KeyCode::Char(c) => {
                    // Keep the draft focused: restrict numeric fields to digits
                    // and a single decimal point; the name accepts any char.
                    let is_numeric = self.settings.field != SettingsField::Name;
                    let mut push = true;
                    if is_numeric {
                        let only_digits_dot = c.is_ascii_digit() || c == '.';
                        if !only_digits_dot {
                            push = false;
                        } else if c == '.' && self.settings.draft.contains('.') {
                            push = false;
                        }
                    }
                    if push {
                        self.settings.draft.push(c);
                        self.settings.dirty = true;
                    }
                    Action::Continue
                }
                KeyCode::Backspace => {
                    self.settings.draft.pop();
                    self.settings.dirty = true;
                    Action::Continue
                }
                KeyCode::Enter => {
                    let settings = &mut self.settings;
                    let profile = &mut self.userdata.profile;
                    settings.commit_edit(profile);
                    self.settings.dirty = true;
                    self.save_profile();
                    Action::Continue
                }
                KeyCode::Esc => {
                    self.settings.cancel_edit();
                    Action::Continue
                }
                _ => Action::Continue,
            }
        } else {
            // Not editing: navigate between profile fields / start editing.
            match key_code {
                KeyCode::Up => {
                    self.settings.field = self.settings.field.prev();
                    Action::Continue
                }
                KeyCode::Down => {
                    self.settings.field = self.settings.field.next();
                    Action::Continue
                }
                KeyCode::Enter => {
                    let settings = &mut self.settings;
                    let profile = &mut self.userdata.profile;
                    settings.begin_edit(profile);
                    Action::Continue
                }
                KeyCode::Tab | KeyCode::Right => {
                    // Move to the next settings panel (escape hatch from the
                    // field editor so the user is never trapped on User).
                    self.selections.next(self.screen);
                    Action::Continue
                }
                KeyCode::BackTab | KeyCode::Left => {
                    self.selections.prev(self.screen);
                    Action::Continue
                }
                _ => Action::Continue,
            }
        }
    }

    pub fn handle_key_press(&mut self, key_code: KeyCode) -> Action {
        // Global screen shortcuts.
        match key_code {
            KeyCode::Char('m') | KeyCode::Char('M') => {
                self.screen = Screen::Main;
                return Action::Continue;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.screen = Screen::Control;
                return Action::Continue;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.screen = Screen::Database;
                self.load_database();
                return Action::Continue;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.screen = Screen::Settings;
                return Action::Continue;
            }
            _ => {}
        }

        match self.screen {
            Screen::Settings => self.handle_settings_key(key_code),
            Screen::Database => self.handle_database_key(key_code),
            _ => self.handle_nav_key(key_code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_cursor_stays_in_bounds_empty() {
        let mut st = DatabaseState::default();
        st.move_cursor(1);
        st.move_cursor(-1);
        assert_eq!(st.selected, 0);
    }

    #[test]
    fn database_cursor_clamps_at_edges() {
        let mut st = DatabaseState::default();
        st.workouts = vec![
            crate::data::WorkoutEntry {
                name: "a".into(),
                path: "a.zwo".into(),
            },
            crate::data::WorkoutEntry {
                name: "b".into(),
                path: "b.zwo".into(),
            },
            crate::data::WorkoutEntry {
                name: "c".into(),
                path: "c.zwo".into(),
            },
        ];
        st.tab = DatabaseTab::Workouts;
        st.move_cursor(1);
        assert_eq!(st.selected, 1);
        st.move_cursor(5);
        assert_eq!(st.selected, 2);
        st.move_cursor(-10);
        assert_eq!(st.selected, 0);
    }

    #[test]
    fn database_tab_switch_preserves_selection() {
        let mut st = DatabaseState::default();
        st.workouts.push(crate::data::WorkoutEntry {
            name: "x".into(),
            path: "x.zwo".into(),
        });
        st.sessions.push(crate::data::StoredSession {
            id: 1,
            filename: "f".into(),
            total_distance: 1.0,
            total_calories: 10.0,
            avg_speed: 20.0,
            max_speed: 30.0,
            max_heart_rate: 150,
            avg_heart_rate: 140,
            max_power: 300,
            avg_power: 200,
            recorded_at: "now".into(),
        });
        // Move to a valid index, switch tab, ensure it stays clamped.
        st.move_cursor(1);
        st.tab = DatabaseTab::Sessions;
        st.ensure_selected_in_range();
        assert_eq!(st.selected, 0);
    }

    #[test]
    fn settings_fields_wrap() {
        assert_eq!(SettingsField::Name.prev(), SettingsField::MaxHr);
        assert_eq!(SettingsField::MaxHr.next(), SettingsField::Name);
    }

    #[test]
    fn settings_commit_parses_and_clamps() {
        let mut st = SettingsState::default();
        let mut profile = UserProfile::default();

        st.field = SettingsField::Ftp;
        st.draft = "750".into();
        st.commit_edit(&mut profile);
        assert_eq!(profile.ftp, 600); // clamped to max

        st.field = SettingsField::Weight;
        st.draft = "not-a-number".into();
        st.commit_edit(&mut profile);
        assert_eq!(profile.weight, 75.0); // unchanged on parse failure
    }

    /// Renders every screen through the draw multiplexer on an offscreen
    /// terminal. Guards against runtime panics (e.g. empty chart data or
    /// layout clipping) on small `TestBackend` canvases.
    fn smoke_render(screen: Screen, width: u16, height: u16) {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new(
            LiveData::new(),
            UserData::new(UserProfile::default()),
            WorkoutData {
                duration: 0,
                total_distance: 0.0,
            },
        );
        app.screen = screen;

        // Give the control panel some graph history so the Chart path runs.
        app.push_power_history();
        app.push_power_history();
        app.push_hr_history();
        app.push_rpm_history();
        app.push_velocity_history();

        terminal
            .draw(|frame| crate::render::draw(frame, &app))
            .expect("screen should render without error");
    }

    #[test]
    fn control_screen_renders_small_terminal() {
        smoke_render(Screen::Control, 40, 15);
    }

    #[test]
    fn control_screen_renders_large_terminal() {
        smoke_render(Screen::Control, 120, 40);
    }

    #[test]
    fn main_screen_renders() {
        smoke_render(Screen::Main, 80, 24);
    }

    #[test]
    fn database_screen_renders() {
        smoke_render(Screen::Database, 80, 24);
    }

    #[test]
    fn settings_screen_renders() {
        smoke_render(Screen::Settings, 80, 24);
    }

    #[test]
    fn stats_screen_renders() {
        smoke_render(Screen::Stats, 80, 24);
    }
}
