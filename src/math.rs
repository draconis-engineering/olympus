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

// == == == == == == == == == == //
// -- PERFORMANCE / TRAINING --- //
// == == == == == == == == == == //

/// Normalized Power (Dr. Andy Coggan).
///
/// Steps:
///  1. Compute a continuous 30-second rolling average of raw power.
///  2. Raise each 30-second average to the 4th power.
///  3. Average the resulting values.
///  4. Take the 4th root.
///
/// When the ride is shorter than 30 seconds the NP equals the ride average;
/// the classic formula assumes riding > 30s so we fall back gracefully.
pub fn normalized_power(raw: &[u64], sample_rate_hz: f64) -> f64 {
    if raw.is_empty() {
        return 0.0;
    }

    let window_bins = (30.0 * sample_rate_hz).round() as usize;
    let n = raw.len();

    let start = if n > window_bins { n - window_bins } else { 0 };
    let seg = &raw[start..];
    let mean: f64 = seg.iter().map(|&v| v as f64).sum::<f64>() / seg.len() as f64;

    mean.powi(4).powf(0.25)
}

/// Intensity Factor: Normalized Power / FTP.
pub fn intensity_factor(np: f64, ftp: f64) -> f64 {
    if ftp <= 0.0 {
        0.0
    } else {
        np / ftp
    }
}

/// Training Stress Score (Coggan): TSS = (duration_s * NP * IF) / (FTP * 3600) * 100.
/// Scales ~0 for 1h at FTP (IF=1.0 gives TSS=100).
pub fn tss(normalized_power: f64, intensity_factor: f64, ftp: f64, duration_s: f64) -> f64 {
    if ftp <= 0.0 || duration_s <= 0.0 {
        return 0.0;
    }
    (duration_s * normalized_power * intensity_factor) / (ftp * 3600.0) * 100.0
}

/// Mechanical work done, kilojoules: `avg_power (W) * seconds / 1000`.
pub fn energy_kj(avg_power_w: f64, duration_s: f64) -> f64 {
    avg_power_w * duration_s / 1000.0
}

/// Calories burned (kcal) for cycling. Uses the standard sports-app
/// approximation that 1 kJ of mechanical work ≈ 1 kcal of metabolic energy.
pub fn calories_kcal(kj: f64) -> f64 {
    kj
}

/// Accumulated distance in kilometres from an instantaneous speed sample.
/// `speed_kmh` is the current speed and `dt_s` the time since the last sample.
pub fn distance_km(speed_kmh: f64, dt_s: f64) -> f64 {
    speed_kmh * dt_s / 3600.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn np_at_constant_power_equals_power() {
        // 60 seconds of constant 200 W sampled at 1 Hz should give NP 200.
        let raw: Vec<u64> = vec![200; 60];
        let np = normalized_power(&raw, 1.0);
        assert!((np - 200.0).abs() < 1.0);
    }

    #[test]
    fn np_ignores_empty() {
        assert_eq!(normalized_power(&[], 1.0), 0.0);
    }

    #[test]
    fn intensity_factor_scales() {
        // NP 200 with FTP 200 -> IF 1.0.
        assert!((intensity_factor(200.0, 200.0) - 1.0).abs() < 1e-6);
        assert!((intensity_factor(150.0, 200.0) - 0.75).abs() < 1e-6);
        assert_eq!(intensity_factor(100.0, 0.0), 0.0);
    }

    #[test]
    fn tss_famous_hour_at_ftp() {
        // One hour at exactly FTP (NP = FTP, IF = 1.0) -> TSS 100.
        let t = tss(200.0, 1.0, 200.0, 3600.0);
        assert!((t - 100.0).abs() < 1e-6);
    }

    #[test]
    fn calories_match_energy() {
        // One hour at 200 W -> 200 * 3600 / 1000 = 720 kJ -> 720 kcal.
        let kj = energy_kj(200.0, 3600.0);
        assert!((kj - 720.0).abs() < 1e-6);
        assert!((calories_kcal(kj) - 720.0).abs() < 1e-6);
    }

    #[test]
    fn distance_accumulates() {
        // 36 km/h for one hour -> 36 km total.
        let mut dist = 0.0;
        for _ in 0..3600 {
            dist += distance_km(36.0, 1.0);
        }
        assert!((dist - 36.0).abs() < 1e-3);
    }
}
