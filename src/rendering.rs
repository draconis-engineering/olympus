// src/rendering.rs

use super::app::{App, Screen};

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
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

fn footer_spans(current: Screen) -> Vec<Span<'static>> {
    // Default style
    let default_style = Style::default();
    let quit_style = Style::default().fg(Color::Red);

    // Initialize un-highlighted
    let mut mainspan = Span::styled("[ M ] Main page | ", default_style);
    let mut conspan = Span::styled("[ C ] Control panel | ", default_style);
    let mut dbspan = Span::styled("[ D ] Database | ", default_style);
    let mut setspan = Span::styled("[ S ] Settings | ", default_style);
    let quitspan = Span::styled("[ Q ] Quit", quit_style);

    // Highlighted style
    let active_style = Style::default()
        .add_modifier(Modifier::BOLD)
        .fg(Color::Green);

    // Highlight current screen
    match current {
        Screen::Main => mainspan = mainspan.style(active_style),
        Screen::Control => conspan = conspan.style(active_style),
        Screen::Database => dbspan = dbspan.style(active_style),
        Screen::Settings => setspan = setspan.style(active_style),
    }

    vec![mainspan, conspan, dbspan, setspan, quitspan]
}

// --- Page-specific Draw Functions ---

fn main_draw(frame: &mut Frame, area: Rect, data: &Data) {
    // Split screen vertically into Header, Main Visual, and Footer status bar
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top HUD Header
            Constraint::Min(10),   // Main Virtual Road / Chart Area
        ])
        .split(area);

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
}

fn control_draw(frame: &mut Frame, area: Rect, data: &Data) {
    let grids: PCHS = quadsplit(area);
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

fn database_draw(frame: &mut Frame, area: Rect) {
    let [sidebar_area, list_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);
}

fn filters_spans() -> Vec<Span<'static>> {
    vec![Span::styled("Filters", Style::default())]
}

fn searchbar_spans() -> Vec<Span<'static>> {
    vec![Span::styled("Searchbar", Style::default())]
}

fn buttons_spans() -> Vec<Span<'static>> {
    vec![Span::styled("Buttons", Style::default())]
}

fn settings_draw(frame: &mut Frame, area: Rect) {
    let [sidebar_area, list_area] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(area);

    let [filters_area, searchbar_area, buttons_area, etc_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(8),
        Constraint::Min(0),
    ])
    .areas(sidebar_area);

    let filters =
        Paragraph::new(Line::from(filters_spans())).block(Block::new().borders(Borders::ALL));
    let searchbar =
        Paragraph::new(Line::from(searchbar_spans())).block(Block::new().borders(Borders::ALL));
    let buttons =
        Paragraph::new(Line::from(buttons_spans())).block(Block::new().borders(Borders::ALL));
    let etc = Block::new().borders(Borders::ALL);
    frame.render_widget(filters, filters_area);
    frame.render_widget(searchbar, searchbar_area);
    frame.render_widget(buttons, buttons_area);
    frame.render_widget(etc, etc_area);
}

// Multiplexer
pub fn draw(frame: &mut Frame, app: &App) {
    // Properly split the screen area into two horizontal sections
    let [content_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    // Render footer
    let footer = Paragraph::new(Line::from(footer_spans(app.screen())));
    frame.render_widget(footer, footer_area);

    // Render content
    match app.screen() {
        Screen::Main => {
            main_draw(frame, content_area, app.data());
        }
        Screen::Control => {
            control_draw(frame, content_area, app.data());
        }
        Screen::Database => {
            database_draw(frame, content_area);
        }
        Screen::Settings => {
            settings_draw(frame, content_area);
        }
    };
}
