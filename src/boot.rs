// src/boot.rs

use super::app::TUIGuard;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::panic;

// Restore screen and "give back" screen to terminal
pub fn restore() -> io::Result<()> {
    // Leave alternate screen buffer
    execute!(io::stdout(), LeaveAlternateScreen)?;

    // Disable raw input processing
    disable_raw_mode()?;

    Ok(())
}

// Initialize terminal state safely
pub fn init() -> io::Result<TUIGuard> {
    // Step 1: Enable raw mode
    enable_raw_mode()?;

    // Step 2: Enter alternate screen
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        // If alternate screen fails, undo step 1 and return error
        let _ = disable_raw_mode();
        return Err(e);
    }

    // Step 3: Setup panic hook for unexpected runtime panics
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore(); // Reset terminal before printing panic message
        original_hook(panic_info);
    }));

    // Step 4: Initialize Ratatui terminal backend
    let backend = CrosstermBackend::new(stdout);
    match Terminal::new(backend) {
        Ok(terminal) => Ok(TUIGuard { tui: terminal }),
        Err(e) => {
            // If creation fails, fully clean up step 2 and step 1
            let _ = restore();
            Err(e)
        }
    }
}
