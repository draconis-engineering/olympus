# OLYMPUS - TUI Cycling Trainer App (Minimalist Zwift/Rouvy Replacement)

Olympus is a minimalist, high-performance Terminal User Interface (**TUI**) for indoor cycling. The application connects directly to smart trainers and fitness sensors via Bluetooth Low Energy (**BLE**), renders real-time telemetry metrics using high-resolution terminal graphics, parses standard workout files, and exposes an interface to the custom Python AI voice assistant, [ICARUS](https://github.com/draconis-engineering/icarus).

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/)

## Architectural Overview 🚀 

The application utilizes a decoupled, multi-threaded design powered by an asynchronous runtime to ensure user interface rendering never blocks real-time hardware data collection.

```markdown
┌────────────────────────────────────────────────────────┐
│                       OLYMPUS                          │
├───────────────┬──────────────────────┬─────────────────┤
│    UI LOOP    │      ASYNC RUN       │    IPC LAYER    │
│   (Ratatui)   │   (Tokio Runtime)    │    (Maturin)    │
└───────┬───────┴──────────┬───────────┴────────┬────────┘
        │                  │                    │
        ▼                  ▼                    ▼
┌───────────────┐  ┌────────────────┐  ┌─────────────────┐
│ STORAGE ENGINE│  │  HARDWARE I/O  │  │  AI VOICE HUB   │
│  (SQLite/FIT) │  │(Btleplug/ANT+) │  │    (ICARUS)     │
└───────────────┘  └────────────────┘  └─────────────────┘
```

## The Tech Stack 🛠️

1. Core Language & UI Layout
- Rust: Provides memory-safe, ultra-low latency execution required for stable 10–50Hz hardware polling.
- Ratatui: An immediate-mode terminal graphics library used to design a multi-panel layout.
- Crossterm: Handles raw terminal window manipulation, resizing math, and key event listening.

2. Concurrency & Hardware Communication
- Tokio: The core asynchronous execution engine running the multi-threaded backend.
- Crossbeam-Channel / Tokio Broadcast: Lock-free pipelines that safely transfer sensor telemetry to the UI thread and push voice override commands to the trainer.
- Btleplug: A cross-platform BLE engine used to subscribe to target standard GATT profiles:
  - Cycling Power Measurement (0x2A63)
  - Heart Rate Measurement (0x2A37)
  - Fitness Machine Control Point (0x2AD) for ERG resistance adjustments.

3. File Processing & Local Storage
- Fitparser / fit-sdk-rust: Converts session telemetry into standard .fit binaries ready for direct upload to platforms like Strava or Garmin Connect.
- Nom / Serde-XML: Parsers designed to load text/XML-based .zwo (Zwift workouts) and .mrc/.erg target files.
- Rusqlite: An embedded SQLite database engine for local storage of profile weight, current FTP values, and local historical training logs.

4. Inter-Process Communication & Voice Assistant (ICARUS)
- Maturin + PyO3: Bridges the Rust binary to the Python ecosystem. PyO3 uses macros to translate low-level telemetry structures into native Python data classes, allowing ICARUS to parse live stats and inject direct operational overrides (e.g., lowering trainer target wattage by voice command).

## Target UI Blueprint 🖥️

The terminal view operates at a stable 30fps utilizing Unicode character patterns for rich visual information density:

- Top Header: Displays elapsed trip timers, virtual mileage tracking, and real-time ASCII elevation profile sparks.
- Telemetry Grid: Highlights major large-text blocks representing instant Power (W), Cadence (RPM), Heart Rate (BPM), and speed calculations.
- Resolution Graphs: Leverages Unicode Braille patterns (⢀⣠⣴⣾) to output smooth, fluid historical lines showing workout tracking curves over elapsed time.
- Workout Matrix: Displays a side-by-side graphical look at the current ERG targets versus actual physical rider output.
