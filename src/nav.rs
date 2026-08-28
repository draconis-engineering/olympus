// src/nav.rs

// Nav.rs implements the navigation system for the Olympus TUI.

use super::app::Screen;

// ---------------------------------------------------------------------------
// Selection navigation
//
// Each selection has a stable, explicitly ordered list of its variants. Moving
// up/down simply indexes into that list with wrapping arithmetic, so there is
// no hand-maintained match chain to get out of sync, and navigation is always
// bounded — it can never panic or fall off either end.
// ---------------------------------------------------------------------------

/// Radius of a closed loop: `count > 0` guarantees wrapping indexes stay in
/// range. All our menus have at least one entry, so this never underflows.
fn wrap(index: usize, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    index % count
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
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
    /// All variants, in display order (first = top of the menu).
    const ALL: [MainSelection; 6] = [
        MainSelection::NewRide,
        MainSelection::Control,
        MainSelection::Workouts,
        MainSelection::Settings,
        MainSelection::Stats,
        MainSelection::Quit,
    ];

    pub fn next(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + 1, Self::ALL.len())];
    }
    pub fn prev(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + Self::ALL.len() - 1, Self::ALL.len())];
    }
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum ControlSelection {
    #[default]
    Main,
}

impl ControlSelection {
    const ALL: [ControlSelection; 1] = [ControlSelection::Main];

    pub fn next(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + 1, Self::ALL.len())];
    }
    pub fn prev(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + Self::ALL.len() - 1, Self::ALL.len())];
    }
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum DatabaseSelection {
    #[default]
    Main,
}

impl DatabaseSelection {
    const ALL: [DatabaseSelection; 1] = [DatabaseSelection::Main];

    pub fn next(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + 1, Self::ALL.len())];
    }
    pub fn prev(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + Self::ALL.len() - 1, Self::ALL.len())];
    }
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum SettingsSelection {
    #[default]
    General,
    Appearance,
    Bluetooth,
    System,
    User,
}

impl SettingsSelection {
    const ALL: [SettingsSelection; 5] = [
        SettingsSelection::General,
        SettingsSelection::Appearance,
        SettingsSelection::Bluetooth,
        SettingsSelection::System,
        SettingsSelection::User,
    ];

    pub fn next(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + 1, Self::ALL.len())];
    }
    pub fn prev(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + Self::ALL.len() - 1, Self::ALL.len())];
    }
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum StatsSelection {
    #[default]
    Overview,
    Rides,
}

impl StatsSelection {
    const ALL: [StatsSelection; 2] = [StatsSelection::Overview, StatsSelection::Rides];

    pub fn next(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + 1, Self::ALL.len())];
    }
    pub fn prev(&mut self) {
        let cur = Self::ALL.iter().position(|v| v == self).unwrap_or(0);
        *self = Self::ALL[wrap(cur + Self::ALL.len() - 1, Self::ALL.len())];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_selection_wraps_without_panic() {
        let mut s = MainSelection::default();
        // Cycle forward several times beyond the length.
        for _ in 0..(MainSelection::ALL.len() * 3 + 1) {
            s.next();
        }
        assert_eq!(s, MainSelection::ALL[1 % MainSelection::ALL.len()]);

        // Cycle backward.
        for _ in 0..(MainSelection::ALL.len() * 3 + 1) {
            s.prev();
        }
        assert_eq!(s, MainSelection::default());
    }

    #[test]
    fn settings_selection_bounded() {
        let mut s = SettingsSelection::default();
        // Wrap around the entire loop.
        for _ in 0..(SettingsSelection::ALL.len() * 2) {
            s.next();
        }
        assert_eq!(s, SettingsSelection::default());
    }

    #[test]
    fn single_item_menu_is_stable() {
        let mut c = ControlSelection::default();
        c.next();
        c.prev();
        assert_eq!(c, ControlSelection::Main);
    }
}
