// src/math.rs
//
// Math.rs is responsible for mathematical operations used in Olympus.

use ratatui::prelude::Color;

// == == = = == == //
// -- HR MODELS -- //
// == == = = == == //

// Olympiatoppen's Model | Get color from HR zone
pub fn olt_hr_model(hr: u16, maxhr: u16) -> u16 {
    let percent = (hr as f32 / maxhr as f32) * 100.0;
    let rounded_percent = percent.round() as u16;

    match rounded_percent {
        0..=54 => 0,
        55..=72 => 1,
        73..=82 => 2,
        83..=87 => 3,
        88..=93 => 4,
        94..=100 => 5,
        _ => 6,
    }
}

// == == = == = == == //
// -- POWER MODELS -- //
// == == = == = == == //

// Coggans Model | Convert power + lactate threshold power to zone
pub fn coggan_pwr_model(pwr: u16, ltpwr: u16) -> u16 {
    // Color, Zone, Zone description
    let ltpwr_percentage = (pwr as f32 / ltpwr as f32) * 100.0;
    match ltpwr_percentage.round() {
        0.0..=54.0 => 1,
        55.0..=75.0 => 2,
        76.0..=90.0 => 3,
        91.0..=105.0 => 4,
        106.0..=120.0 => 5,
        121.0..=150.0 => 6,
        151.0..=1000.0 => 7,
        _ => 0,
    }
}

// == == == = = == == == //
// -- RENDERING UTILS -- //
// == == == = = == == == //

pub fn zone2color(zone: u16) -> Color {
    match zone {
        1 => Color::LightBlue,
        2 => Color::Blue,
        3 => Color::Green,
        4 => Color::Yellow,
        5 => Color::Rgb(255, 128, 0),
        6 => Color::Red,
        7 => Color::Rgb(255, 192, 203),
        _ => Color::White,
    }
}

// == == == == == == == == //
// -- HISTORY HELPERS  --- //
// == == == == == == == == //

/// Pushes `pwr` onto a rolling history buffer of fixed capacity.
/// Oldest entry is dropped when the buffer is full.
pub fn push_history(buf: &mut Vec<u64>, pwr: u64, cap: usize) {
    if buf.len() >= cap {
        buf.remove(0);
    }
    buf.push(pwr);
}

/// Computes the rolling arithmetic mean over the last `window` samples.
pub fn rolling_mean(buf: &[u64], window: usize) -> f64 {
    let n = buf.len();
    if n == 0 {
        return 0.0;
    }
    let start = n.saturating_sub(window);
    let slice = &buf[start..];
    slice.iter().map(|&v| v as f64).sum::<f64>() / slice.len() as f64
}
