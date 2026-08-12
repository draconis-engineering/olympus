// src/guard.rs

use super::boot::restore;
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
