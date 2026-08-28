# OLYMPUS - TUI Cycling Trainer App (Minimalist Zwift/Rouvy Replacement)

Olympus is a minimalist, high-performance **TUI** for indoor cycling. The application connects directly to smart trainers and fitness sensors via **BLE**, renders real-time telemetry metrics using high-res terminal graphics, parses `.fit`-files, with more functionality on the way.

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

## Architectural Overview

The application utilizes a decoupled, multi-threaded design powered by an asynchronous runtime to ensure user interface rendering never blocks real-time hardware data collection.

```markdown
┌────────────────────────────────────────────────────────┐
│                       OLYMPUS                          │
├────────────────┬────────────────────┬──────────────────┤
│    UI LOOP     │     ASYNC RUN      │     IPC LAYER    │
│   (Ratatui)    │  (Tokio Runtime)   │     (Maturin)    │
└───────┬────────┴─────────┬──────────┴─────────┬────────┘
        │                  │                    │
        ▼                  ▼                    ▼
┌────────────────┐ ┌────────────────┐ ┌──────────────────┐
│ STORAGE ENGINE │ │  HARDWARE I/O  │ │  AI VOICE HUB    │
│  (SQLite/FIT)  │ │(Btleplug/ANT+) │ │    (ICARUS)      │
└────────────────┘ └────────────────┘ └──────────────────┘
```

## The Tech Stack

### Core Language & UI Layout
- **Rust**: Provides memory-safe, ultra-low latency execution required for stable 10–50Hz hardware polling.
- **Ratatui**: An immediate-mode terminal graphics library used to design a multi-panel layout.
- **Crossterm**: Handles raw terminal window manipulation, resizing math, and key event listening.

### Concurrency & Hardware Communication
- **Tokio**: The core asynchronous execution engine running the multi-threaded backend.
- **Crossbeam**-**Channel** / Tokio Broadcast: Lock-free pipelines that safely transfer sensor telemetry to the UI thread and push voice override commands to the trainer.
- **Btleplug**: A cross-platform BLE engine used to subscribe to target standard GATT profiles:
  - Cycling Power Measurement (0x2A63)
  - Heart Rate Measurement (0x2A37)
  - Fitness Machine Control Point (0x2AD) for ERG resistance adjustments.

### File Processing & Local Storage
- **Fitparser**: Converts/validates session telemetry; Olympus ships its own minimal FIT **writer** for Garmin-compatible `.fit` output.
- **Nom** / **Serde-XML**: Parsers designed to load text/XML-based .zwo (Zwift workouts) and .mrc/.erg target files.
- **Rusqlite**: An embedded SQLite database engine for local storage of profile weight, current FTP values, and local historical training logs.

### Inter-Process Communication & Voice Assistant ([ICARUS](https://github.com/draconis-engineering/icarus))
- **Maturin** + **PyO3**: Bridges the Rust binary to the Python ecosystem. PyO3 uses macros to translate low-level telemetry structures into native Python data classes, allowing ICARUS to parse live stats and inject direct operational overrides (e.g., lowering trainer target wattage by voice command).

## Target UI Blueprint

The terminal view operates at a stable 30fps utilizing Unicode character patterns for rich visual information density:

- Top Header: Displays elapsed trip timers, virtual mileage tracking, and real-time ASCII elevation profile sparks.
- Telemetry Grid: Highlights major large-text blocks representing instant Power (W), Cadence (RPM), Heart Rate (BPM), and speed calculations.
- Resolution Graphs: Leverages Unicode Braille patterns (⢀⣠⣴⣾) to output smooth, fluid historical lines showing workout tracking curves over elapsed time.
- Workout Matrix: Displays a side-by-side graphical look at the current ERG targets versus actual physical rider output.

## Build & Run (v0.1)

```
cargo run --release
```

To begin a workout, pass a path to a `.erg` or `.zwo` file as the first argument:

```
cargo run --release -- my/workout.zwo
```

The ride clock, metrics (NP/IF/TSS/kJ/kcal), power rolling averages and distance
all update once per second. On quit (`q` or the Quit menu item):

- a Garmin-compatible `.fit` activity is written to `data/.fit/`, and
- a session summary is stored in the SQLite database at `data/olympus.db`.

Rider settings (weight, height, FTP, max HR, name) live in `data/user/profile.json`
and are created with sensible defaults on first run.

### Bluetooth / Smart trainer (Linux)

On Linux the BLE stack is **BlueZ**, and `btleplug` requires the experimental
Bluetooth APIs. Start `bluetoothd` with the `-E` (experimental) flag:

```bash
sudo systemctl edit bluetooth
# add:
#   [Service]
#   ExecStart=
#   ExecStart=/usr/lib/bluetooth/bluetoothd -E
sudo systemctl restart bluetooth
```

Then confirm your adapter/dongle is up:

```bash
bluetoothctl power on
bluetoothctl scan on   # optional; the app scans on start
```

If no trainer/sensor is found the app falls back to simulated data so the UI
stays live. Mac/Windows need no extra setup beyond granting Bluetooth access.

### Running without Bluetooth

The app runs happily with no trainer attached — it emits simulated power,
cadence, heart rate and speed, and keeps the FIT/SQLite persistence working.

## v0.1 Feature Status

- [x] End-to-end ride: BLE acquisition (power / cadence / HR / speed)
- [x] ERG target power pushed to the trainer (FTMS control point)
- [x] `.erg` and `.zwo` workout parsing + interval scheduling
- [x] Metrics: rolling 5/10/20-min power, NP, IF, TSS, kJ, kcal, distance
- [x] Rider profile (JSON) load/save
- [x] FIT activity writer (Strava/Garmin-compatible)
- [x] SQLite session persistence
- [x] Database browsing / workout history UI (v0.2)
- [x] Profile settings screen edits (v0.2)
