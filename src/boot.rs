// src/boot.rs
//
// Boot.rs is responsible for initializing the terminal and TUI state.
// Async-aware version with tokio integration.

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;

/// Initialize terminal state safely (async version)
/// Must be called from a tokio runtime.
pub async fn init() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    // Step 1: Enable raw mode
    enable_raw_mode()?;

    // Step 2: Enter alternate screen
    let mut stdout = std::io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        // If alternate screen fails, undo step 1 and return error
        let _ = disable_raw_mode();
        return Err(e);
    }

    // Step 3: Setup panic hook for unexpected runtime panics
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode(); // Reset terminal before printing panic message
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Step 4: Initialize Ratatui terminal backend
    let backend = CrosstermBackend::new(stdout);
    match Terminal::new(backend) {
        Ok(terminal) => Ok(terminal),
        Err(e) => {
            // If creation fails, fully clean up step 2 and step 1
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
            Err(e)
        }
    }
}

/// Restore screen and "give back" screen to terminal
/// Async-compatible version
pub async fn restore() -> io::Result<()> {
    // Leave alternate screen buffer
    execute!(std::io::stdout(), LeaveAlternateScreen)?;

    // Disable raw input processing
    disable_raw_mode()?;

    Ok(())
}
