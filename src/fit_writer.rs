// src/fit_writer.rs
//
// A small, self-contained writer for Garmin FIT activity files.
//
// We generate a valid .fit with a FileId, a stream of Record messages (one per
// sample) and a Session summary. The resulting file can be uploaded to Strava,
// Garmin Connect, etc. This avoids pulling in a large FIT SDK dependency for
// the handful of messages Olympus needs.
//
// FIT protocol v1 details used here:
//   * Timestamps are seconds since the FIT epoch (1989-12-31 00:00:00 UTC).
//   * Distances are stored as meters * 100 (uint32).
//   * Speeds are stored as m/s * 1000 (uint16).
//   * CRC16-CCITT (poly 0x1021, init 0x0000) is used for the file CRC.

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Seconds between the Unix epoch and the FIT epoch (1989-12-31 UTC).
const FIT_EPOCH_OFFSET: i64 = 631_065_600;

// FIT base types (the high bit marks a "real" numeric type vs an enum).
const BASE_ENUM: u8 = 0x00;
const BASE_UINT8: u8 = 0x02;
const BASE_UINT16: u8 = 0x84;
const BASE_UINT32: u8 = 0x86;
const BASE_UINT32Z: u8 = 0x8C;

/// A single typed field inside a FIT message.
struct FitField {
    num: u8,
    base_type: u8,
    value: Vec<u8>,
}

impl FitField {
    fn enum_(num: u8, v: u8) -> Self {
        Self {
            num,
            base_type: BASE_ENUM,
            value: vec![v],
        }
    }
    fn uint8(num: u8, v: u8) -> Self {
        Self {
            num,
            base_type: BASE_UINT8,
            value: v.to_le_bytes().to_vec(),
        }
    }
    fn uint16(num: u8, v: u16) -> Self {
        Self {
            num,
            base_type: BASE_UINT16,
            value: v.to_le_bytes().to_vec(),
        }
    }
    fn uint32(num: u8, v: u32) -> Self {
        Self {
            num,
            base_type: BASE_UINT32,
            value: v.to_le_bytes().to_vec(),
        }
    }
    fn uint32z(num: u8, v: u32) -> Self {
        Self {
            num,
            base_type: BASE_UINT32Z,
            value: v.to_le_bytes().to_vec(),
        }
    }
}

/// A logical FIT message (definition + one or more data rows).
struct FitMessage {
    global: u16,
    local: u8,
    fields: Vec<FitField>,
}

impl FitMessage {
    /// Serialize the definition message.
    fn encode_definition(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(0x40 | self.local); // definition header
        out.push(0x00); // reserved
        out.push(0x01); // architecture: little-endian
        out.extend_from_slice(&self.global.to_le_bytes());
        out.push(self.fields.len() as u8);
        for f in &self.fields {
            out.push(f.num);
            out.push(f.value.len() as u8); // size in bytes
            out.push(f.base_type);
        }
        out
    }

    /// Serialize one data row.
    fn encode_data(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(self.local); // data header = local message number
        for f in &self.fields {
            out.extend_from_slice(&f.value);
        }
        out
    }
}

/// A record-level sample captured during the ride.
#[derive(Debug, Clone, Copy)]
pub struct RecordSample {
    /// Seconds since the Unix epoch.
    pub timestamp: i64,
    /// Power in watts.
    pub power: u16,
    /// Cadence in rpm.
    pub cadence: u8,
    /// Heart rate in bpm.
    pub heart_rate: u8,
    /// Speed in m/s.
    pub speed_mps: f32,
    /// Accumulated distance in meters.
    pub distance_m: f32,
}

/// Accumulates samples and can finalize a FIT activity file.
pub struct FitWriter {
    samples: Vec<RecordSample>,
    /// Start timestamp (Unix epoch seconds).
    pub start_time: i64,
}

impl Default for FitWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl FitWriter {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            start_time: chrono::Utc::now().timestamp(),
        }
    }

    /// Add one record sample.
    pub fn push(&mut self, s: RecordSample) {
        self.samples.push(s);
    }

    pub fn _sample_count(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Compute the session averages used by the summary message.
    fn session_summary(&self) -> (u16, u8, u8, u16, u16, u32, u16, u32) {
        // Returns (avg_speed_mps_x1000, avg_hr, max_hr, avg_power, max_power,
        //          total_distance_mx100, total_calories, total_time_ms)
        let n = self.samples.len().max(1);
        let total_dist: f32 = self.samples.last().map(|s| s.distance_m).unwrap_or(0.0);
        let sum_speed: f32 = self.samples.iter().map(|s| s.speed_mps).sum();
        let sum_pwr: u64 = self.samples.iter().map(|s| s.power as u64).sum();
        let sum_hr: u64 = self.samples.iter().map(|s| s.heart_rate as u64).sum();
        let max_hr = self.samples.iter().map(|s| s.heart_rate).max().unwrap_or(0);
        let max_pwr = self.samples.iter().map(|s| s.power).max().unwrap_or(0);
        // total time in ms from start
        if let (Some(first), Some(last)) = (self.samples.first(), self.samples.last()) {
            let total_ms = ((last.timestamp - first.timestamp) * 1000).max(0) as u32;
            (
                (sum_speed / n as f32 * 1000.0) as u16,
                (sum_hr / n as u64) as u8,
                max_hr,
                (sum_pwr / n as u64) as u16,
                max_pwr,
                (total_dist * 100.0) as u32,
                0,
                total_ms,
            )
        } else {
            (0, 0, 0, 0, 0, 0, 0, 0)
        }
    }

    /// Build the full FIT byte stream.
    fn encode(&self) -> Vec<u8> {
        let fit_start = self.start_time - FIT_EPOCH_OFFSET;

        // --- FileId message (local 0) ---
        let file_id = FitMessage {
            global: 0, // FILE_ID
            local: 0,
            fields: vec![
                FitField::enum_(0, 4),                 // type = activity
                FitField::uint16(1, 255),              // Garmin manufacturer
                FitField::uint16(2, 3277), // product (garmin: fēnix 5 => 2691; placeholder)
                FitField::uint32z(3, 0),   // serial
                FitField::uint32(4, fit_start as u32), // time_created
            ],
        };

        // --- Record message (local 1) ---
        // Definition is emitted once; each sample becomes a data row.

        // --- Session message (local 2) ---
        let (avg_speed, avg_hr, max_hr, avg_pwr, max_pwr, dist100, _cal, total_ms) =
            self.session_summary();

        let session = FitMessage {
            global: 18, // SESSION
            local: 2,
            fields: vec![
                FitField::uint32(253, fit_start as u32), // timestamp (end)
                FitField::uint32(0, fit_start as u32),   // start_time
                FitField::uint32(2, total_ms),           // total_elapsed_time (ms)
                FitField::uint32(3, total_ms),           // total_timer_time (ms)
                FitField::uint32(4, dist100),            // total_distance (m*100)
                FitField::uint16(6, 0),                  // total_calories
                FitField::uint16(7, avg_speed),          // avg_speed (m/s*1000)
                FitField::enum_(55, 2),                  // sport = cycling
                FitField::uint8(9, avg_hr),              // avg_heart_rate
                FitField::uint8(10, max_hr),             // max_heart_rate
                FitField::uint16(13, avg_pwr),           // avg_power
                FitField::uint16(14, max_pwr),           // max_power
            ],
        };

        // Serialize into a data buffer.
        let mut data: Vec<u8> = Vec::new();
        data.extend(file_id.encode_definition());
        data.extend(file_id.encode_data());

        // Record definition (emitted once; each sample is a data row).
        let record_def = FitMessage {
            global: 20,
            local: 1,
            fields: vec![
                FitField::uint32(253, 0),
                FitField::uint32(5, 0),
                FitField::uint16(6, 0),
                FitField::uint8(4, 0),
                FitField::uint8(3, 0),
                FitField::uint16(7, 0),
            ],
        };
        data.extend(record_def.encode_definition());

        for s in &self.samples {
            let ts = (s.timestamp - FIT_EPOCH_OFFSET) as u32;
            let new_record = FitMessage {
                global: 20,
                local: 1,
                fields: vec![
                    FitField::uint32(253, ts),
                    FitField::uint32(5, (s.distance_m * 100.0) as u32),
                    FitField::uint16(6, (s.speed_mps * 1000.0) as u16),
                    FitField::uint8(4, s.cadence),
                    FitField::uint8(3, s.heart_rate),
                    FitField::uint16(7, s.power),
                ],
            };
            data.extend(new_record.encode_data());
        }
        data.extend(session.encode_definition());
        data.extend(session.encode_data());

        // Append the data CRC.
        let mut crc = 0u16;
        for &b in &data {
            crc = crc16(crc, b);
        }
        data.extend_from_slice(&crc.to_le_bytes());

        // Build the 14-byte file header.
        let data_size = (data.len() - 2) as u32; // exclude trailing CRC
        let mut header = Vec::with_capacity(14);
        header.push(14); // header size
        header.push(0x10); // protocol version 1.0
        header.extend_from_slice(&(1055u16).to_le_bytes()); // profile version 10.55
        header.extend_from_slice(&data_size.to_le_bytes());
        header.extend_from_slice(b".FIT"); // data type (case-sensitive: ".FIT")

        // Header CRC over first 12 bytes.
        let mut header_crc = 0u16;
        for &b in &header {
            header_crc = crc16(header_crc, b);
        }
        header.extend_from_slice(&header_crc.to_le_bytes());

        let mut out = header;
        out.extend_from_slice(&data);
        out
    }

    /// Write the finished FIT activity to `path`.
    pub fn finish(&self, path: &Path) -> Result<u64, String> {
        if self.samples.is_empty() {
            return Err("no samples to write".into());
        }
        let bytes = self.encode();
        let n = bytes.len() as u64;
        let mut file = File::create(path).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        Ok(n)
    }
}

/// FIT file CRC. Matches the algorithm used by the FIT SDK and fitparser
/// (`fit_crc.c`) — a CRC-16 computed nibble-at-a-time off a lookup table,
/// starting from 0. This is CRC-16/ARC (poly 0x8005, reflected).
const CRC_TABLE: [u16; 16] = [
    0x0000, 0xCC01, 0xD801, 0x1400, 0xF001, 0x3C00, 0x2800, 0xE401, 0xA001, 0x6C00, 0x7800, 0xB401,
    0x5000, 0x9C01, 0x8801, 0x4400,
];

/// Update a running FIT CRC with one byte.
fn crc16(mut crc: u16, byte: u8) -> u16 {
    // lower four bits of byte
    let mut tmp = CRC_TABLE[(crc & 0xF) as usize];
    crc = (crc >> 4) & 0x0FFF;
    crc = crc ^ tmp ^ CRC_TABLE[(byte & 0xF) as usize];

    // upper four bits of byte
    tmp = CRC_TABLE[(crc & 0xF) as usize];
    crc = (crc >> 4) & 0x0FFF;
    crc = crc ^ tmp ^ CRC_TABLE[((byte >> 4) & 0xF) as usize];

    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crc_of(data: &[u8]) -> u16 {
        let mut crc = 0u16;
        for &b in data {
            crc = crc16(crc, b);
        }
        crc
    }

    #[test]
    fn crc16_ccitt_known_vector() {
        // "123456789" -> 0xBB3D for CRC-16/ARC (the algorithm FIT uses).
        let crc = crc_of(b"123456789");
        assert_eq!(crc, 0xBB3D);
    }

    #[test]
    fn writes_a_fit_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut w = FitWriter::new();
        let base = w.start_time;
        for i in 0..10 {
            w.push(RecordSample {
                timestamp: base + i,
                power: 200,
                cadence: 90,
                heart_rate: 150,
                speed_mps: 8.33,
                distance_m: i as f32 * 8.33,
            });
        }
        let path = dir.path().join("out.fit");
        let written = w.finish(&path).unwrap();
        assert!(written > 0);

        // The file must round-trip through fitparser (validates CRC).
        let mut f = File::open(&path).unwrap();
        let records = fitparser::from_reader(&mut f).unwrap();
        assert!(!records.is_empty());
    }

    #[test]
    fn refuses_empty_ride() {
        let dir = tempfile::tempdir().unwrap();
        let w = FitWriter::new();
        let path = dir.path().join("out.fit");
        assert!(w.finish(&path).is_err());
    }
}
