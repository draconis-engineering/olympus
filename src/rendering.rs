// src/rendering.rs

use super::app::{App, Screen};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

struct PCHS {
    // power cadence hr speed
    power: Rect,
    cadence: Rect,
    hr: Rect,
    speed: Rect,
}

pub struct Data {
    pub power: u16,
    pub cadence: u16,
    pub hr: u16,
    pub speed: f32,
}

// Simple quadsplit helper assuming it slices an area into 4 quarters
fn quadsplit(area: Rect) -> PCHS {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical[0]);

    let bottom = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(vertical[1]);

    PCHS {
        power: top[0],
        cadence: top[1],
        hr: bottom[0],
        speed: bottom[1],
    }
}

// --- Page-specific Draw Functions ---

fn main_draw(frame: &mut Frame, data: &Data) {
    // Split screen vertically into Header, Main Visual, and Footer status bar
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top HUD Header
            Constraint::Min(10),   // Main Virtual Road / Chart Area
            Constraint::Length(3), // Bottom Navigation Hint
        ])
        .split(frame.area());

    // Split the Top HUD horizontally into 4 quick-glance segments
    let hud_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(main_layout[0]);

    // Render Top HUD Stats
    frame.render_widget(
        Paragraph::new(format!(" ⚡ {} W", data.power)).block(Block::new().borders(Borders::ALL)),
        hud_layout[0],
    );
    frame.render_widget(
        Paragraph::new(format!(" 🔄 {} RPM", data.cadence))
            .block(Block::new().borders(Borders::ALL)),
        hud_layout[1],
    );
    frame.render_widget(
        Paragraph::new(format!(" ❤️ {} BPM", data.hr)).block(Block::new().borders(Borders::ALL)),
        hud_layout[2],
    );
    frame.render_widget(
        Paragraph::new(format!(" 💨 {:.1} KM/H", data.speed))
            .block(Block::new().borders(Borders::ALL)),
        hud_layout[3],
    );

    // Centered Dashboard Content (Zwift Visual Mockup)
    // You could replace this later with a Canvas, Sparkline, or Chart widget
    let center_block = Block::new()
        .title(" Virtual Route View ")
        .borders(Borders::ALL);

    let mock_visual = Paragraph::new("\n\n   ▲  [You] Climbing Watopia Epic KOM (1.2km remaining at 7%)\n   ▲  Rider B (+0:14)\n   ▼  Rider C (-0:05)")
        .block(center_block)
        .wrap(Wrap { trim: true });

    frame.render_widget(mock_visual, main_layout[1]);

    // Bottom Navigation / Context Footer
    let footer = Paragraph::new(
        " [M] Main  |  [C] Control Panel  |  [D] Database  |  [S] Settings  |  Press Esc to Quit",
    )
    .block(Block::new().borders(Borders::TOP));
    frame.render_widget(footer, main_layout[2]);
}

fn control_draw(frame: &mut Frame, data: &Data) {
    let grids: PCHS = quadsplit(frame.area());
    let powerwdgt = Block::new().title("Power").borders(Borders::ALL);
    let cadencewdgt = Block::new().title("Cadence").borders(Borders::ALL);
    let hrwdgt = Block::new().title("Heart Rate").borders(Borders::ALL);
    let speedwdgt = Block::new().title("Speed").borders(Borders::ALL);
    let powerpara = Paragraph::new(format!("{} Watts", data.power)).block(powerwdgt);
    let cadencepara = Paragraph::new(format!("{} RPM", data.cadence)).block(cadencewdgt);
    let hrpara = Paragraph::new(format!("{} BPM", data.hr)).block(hrwdgt);
    let speedpara = Paragraph::new(format!("{:.1} KM/H", data.speed)).block(speedwdgt);
    frame.render_widget(powerpara, grids.power);
    frame.render_widget(cadencepara, grids.cadence);
    frame.render_widget(hrpara, grids.hr);
    frame.render_widget(speedpara, grids.speed);
}

// Draw Multiplexer
pub fn draw(frame: &mut Frame, app: &App) {
    match app.screen() {
        Screen::Main => {
            main_draw(frame, app.data());
        }
        Screen::Control => {
            control_draw(frame, app.data());
        }
        Screen::Database => {
            // Simple placeholder layout rendering
            let stub = Paragraph::new("Database View - Coming Soon")
                .block(Block::new().borders(Borders::ALL));
            frame.render_widget(stub, frame.area());
        }
        Screen::Settings => {
            // Simple placeholder layout rendering
            let stub = Paragraph::new("Settings View - Coming Soon")
                .block(Block::new().borders(Borders::ALL));
            frame.render_widget(stub, frame.area());
        }
    }
}
