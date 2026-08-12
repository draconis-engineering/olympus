// src/lib.rs

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct PCHS {
    // power cadence hr speed
    power: Rect,
    cadence: Rect,
    hr: Rect,
    speed: Rect,
}

pub fn quadsplit(area: Rect) -> PCHS {
    let constraints = [Constraint::Percentage(50), Constraint::Percentage(50)];
    let splitonce = Layout::vertical(constraints).split(area);
    let pc = Layout::horizontal(constraints).split(splitonce[0]);
    let hs = Layout::horizontal(constraints).split(splitonce[1]);
    return PCHS {
        power: pc[0],
        cadence: pc[1],
        hr: hs[0],
        speed: hs[1],
    };
}

pub struct Data {
    pub power: u16,
    pub cadence: u16,
    pub hr: u16,
    pub speed: f32,
}

pub fn draw(frame: &mut Frame, data: &Data) {
    let grids: PCHS = quadsplit(frame.area());
    let powerwdgt = Block::new().title("Power").borders(Borders::ALL);
    let cadencewdgt = Block::new().title("Cadence").borders(Borders::ALL);
    let hrwdgt = Block::new().title("Heart Rate").borders(Borders::ALL);
    let speedwdgt = Block::new().title("Speed").borders(Borders::ALL);
    let powerpara = Paragraph::new(format!("{} Watts", data.power)).block(powerwdgt);
    let cadencepara = Paragraph::new(format!("{} RPM", data.cadence)).block(cadencewdgt);
    let hrpara = Paragraph::new(format!("{} BPM", data.hr)).block(hrwdgt);
    let speedpara = Paragraph::new(format!("{} KM/H", data.speed)).block(speedwdgt);
    frame.render_widget(powerpara, grids.power);
    frame.render_widget(cadencepara, grids.cadence);
    frame.render_widget(hrpara, grids.hr);
    frame.render_widget(speedpara, grids.speed);
}
