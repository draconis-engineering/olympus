use ratatui::prelude::Color;

// Olympiatoppen's Model | Get color from HR zone
pub fn olt_hr_model(hr: u16, maxhr: u16) -> (Color, u16, f32) {
    let percent = (hr as f32 / maxhr as f32) * 100.0;
    let rounded_percent = percent.round() as u16;

    match rounded_percent {
        0..=54 => (Color::White, 0, percent.round()),
        55..=72 => (Color::Gray, 1, percent.round()),
        73..=82 => (Color::LightBlue, 2, percent.round()),
        83..=87 => (Color::Green, 3, percent.round()),
        88..=93 => (Color::Yellow, 4, percent.round()),
        94..=100 => (Color::Red, 5, percent.round()),
        _ => (Color::LightRed, 6, percent.round()),
    }
}

// Coggans Model | Convert power + lactate threshold power to color for rendering based on power zones
pub fn coggan_pwr_model(pwr: u16, ltpwr: u16) -> (Color, u16, f32) {
    // Color, Zone, Zone description
    let ltpwr_percentage = (pwr as f32 / ltpwr as f32) * 100.0;
    match ltpwr_percentage.round() {
        0.0..=54.0 => (Color::LightBlue, 1, ltpwr_percentage),
        55.0..=75.0 => (Color::Blue, 2, ltpwr_percentage),
        76.0..=90.0 => (Color::Green, 3, ltpwr_percentage),
        91.0..=105.0 => (Color::Yellow, 4, ltpwr_percentage),
        106.0..=120.0 => (Color::Rgb(255, 128, 0), 5, ltpwr_percentage), // Orange
        121.0..=150.0 => (Color::Red, 6, ltpwr_percentage),
        151.0..=1000.0 => (Color::Rgb(255, 192, 203), 7, ltpwr_percentage), // Pink
        _ => (Color::White, 0, ltpwr_percentage),
    }
}
