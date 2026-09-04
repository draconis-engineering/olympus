# OLYMPUS - TUI Cycling Trainer App (Minimalist Zwift/Rouvy Replacement)

Olympus is a minimalist, high-performance **TUI** for indoor cycling. The application connects directly to smart trainers and fitness sensors via **BLE** (Tacx Flux S2 via FTMS), renders real-time telemetry with high-res terminal graphics, schedules `.erg`/`.zwo` workouts, and writes Garmin-valid `.fit` files — no subscription, no cloud.

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/) [![Roadmap](https://img.shields.io/badge/Roadmap-v1.0-blue)](../ROADMAP.md)

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
- **Tokio mpsc channels**: Non-blocking pipelines that transfer sensor telemetry to the UI thread (60 fps render loop, 1 Hz ride-engine tick in `src/main.rs:115`).
- **Btleplug**: A cross-platform BLE engine (BlueZ on Linux, CoreBluetooth on macOS, WinRT on Windows) subscribing to standard GATT profiles:
  - Cycling Power Measurement (0x2A63) + CrankTracker cadence `src/ble.rs:40`
  - Heart Rate Measurement (0x2A37)
  - Fitness Machine Control Point (0x2AD9) for ERG resistance adjustments.

### File Processing & Local Storage
- **Fitparser + custom FIT writer** (`src/fit_writer.rs:191`): Validates and emits Garmin-compatible `.fit` (FileId + Records + Session, CRC `0xBB3D`) — `data/.fit/ride_*.fit` uploads cleanly to Garmin Connect → Strava.
- **Parsers** (`src/erg.rs:113`, `src/erg.rs:190`): `.erg` key-value (`TARGET_POWER`/`DURATION`/… ) and `.zwo` XML (`Warmup`/`SteadyState`/`IntervalsT`/`Cooldown`/`Ramp`, `≤10→×FTP` scaling) via `xml` crate.
- **Rusqlite** (`src/data.rs:137`): Embedded SQLite at `data/olympus.db` with `fit_sessions` summary + per-second `samples(session_id, t, power, cadence, hr, speed)` for retro analytics (`save_ride()` `src/data.rs:200`).
- **Serde JSON**: Rider profile at `data/user/profile.json` (`username/weight/height/ftp/max_hr`) with clamped editor `src/app.rs:347`.

### Inter-Process Communication & Voice Assistant ([ICARUS](https://github.com/draconis-engineering/icarus))
- **Maturin** + **PyO3**: Bridges the Rust binary to the Python ecosystem. PyO3 uses macros to translate low-level telemetry structures into native Python data classes, allowing ICARUS to parse live stats and inject direct operational overrides (e.g., lowering trainer target wattage by voice command).

## Target UI Blueprint

The terminal view runs at 60 fps (1 Hz ride-engine tick) using Unicode Braille for rich visual density:

- **Header/Footer** (`src/render.rs:287`/`250`): `olympus` + version + local time; footer highlights current page and rider name.
- **Main** (`src/render.rs:308`): ASCII `OLYMPUS` slant logo + 6-item nav (`New Ride` → Control, Workouts → Database, Settings, Stats, Quit with confirm `src/render.rs:121`).
- **Control** (`src/render.rs:392`): Big Power/HR (`tui-big-text`), Braille history for power/HR/cadence/speed `tail_points()` `41`, zone gauges `726`, Ride Stats `TIME/DIST/ELEV/GRAD/CAL/TSS/IF` `778`, Intervals `w.step_at(elapsed)` `822`, System `BT [STATE]` color `926` (`Connected` green / `Simulated` yellow / `Error` red). Overlays: `Paused` banner `978` (`Space` resume, `Q` finish) and end-of-ride Summary `162` (`Save/Discard/Resume`).
- **Database** (`src/render.rs:1008`): Two-tab `Workouts` (`data/workouts/*.zwo|*.erg` `src/data.rs:329`) and `Sessions` (`fit_sessions` `src/data.rs:285`).
- **Settings** (`src/render.rs:1160`): `General`/`Appearance` (stubs) / `Bluetooth` (live `state.label()` `1283`) / `System` / `User` profile editor.
- **Stats** (`src/render.rs:1399`): Stub in v0.1.5 — weekly TSS + PR curve planned for v1.0 (see Roadmap).

## Build & Run (v0.1.5 → v1.0)

```
cargo run --release
```

To begin a workout, pass a path to a `.erg` or `.zwo` file as the first argument:

```
cargo run --release -- my/workout.zwo
```

Or pick one from **Database → Workouts** (press `Enter` on a `.zwo`/`.erg` in `data/workouts/`).

**During a ride (Control panel):**
- `Space` / `Enter` — pause / resume (clock + distance + FIT freeze via `is_recording()` `src/app.rs:500`)
- `Q` — open end-of-ride summary (`Save [S/Y]` / `Discard [D/N]` / `Resume [R/Esc]` `src/render.rs:162`); `Save` writes `data/.fit/ride_*.fit` + `data/olympus.db` `src/data.rs:200`, `Discard` drops the recording, both return to Main.
- Ride clock, `NP/IF/TSS/kJ/kcal`, rolling `5/10/20-min` power and distance update once per second (`src/app.rs:634` + `src/main.rs:115`).

**On quit** (Main → Quit, confirm `Y`) — any unsaved in-memory FIT is flushed as a safety net. Prefer finishing via the summary.

Rider settings (weight, height, FTP, max HR, name) live in `data/user/profile.json`
and are edited live in **Settings → User** (`src/app.rs:347`).

> **Roadmap to 1.0:** See [`../ROADMAP.md`](../ROADMAP.md) for the locked build plan (phases 0–6, out-of-scope, and competitive positioning vs. Strava/Rouvy/Tacx/TrainerRoad/Zwift).

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

## Feature Status

### Shipped (v0.1.5)

- [x] End-to-end ride: BLE acquisition (power / cadence via `CrankTracker` `src/ble.rs:40` / HR / speed)
- [x] ERG target power pushed to trainer (FTMS `0x2AD9` `src/ble.rs:464`, with `Scanning/Connecting/Simulated/Error` states `src/ble.rs:103`)
- [x] `.erg` and `.zwo` workout parsing + interval scheduling (`Warmup/SteadyState/IntervalsT/Cooldown/Ramp`, `≤10→×FTP` `src/erg.rs:113`)
- [x] Metrics: rolling `5/10/20-min` (+`3m/1m/30s/10s/3s`) power, NP/IF/TSS/kJ/kcal, distance (`src/math.rs:92`, `src/app.rs:634`)
- [x] Rider profile (JSON) load/save with clamped editor (`src/data.rs:119`, `src/app.rs:347`)
- [x] FIT activity writer — Garmin-valid (`src/fit_writer.rs:191`, `data/.fit/ride_*.fit`)
- [x] SQLite session + per-second `samples` persistence (`src/data.rs:137`, `save_ride()` `200`)
- [x] Database browsing (Workouts + Sessions) (`src/render.rs:1008`, `src/app.rs:232`)
- [x] Profile & Bluetooth settings (`src/render.rs:1160`)
- [x] Ride lifecycle: `Running/Paused/Summary` (`src/app.rs:170`) — pause (`Space`), summary (`Q` → `Save/Discard/Resume` `src/render.rs:162`), `is_recording()` gate `500`, `Paused` banner `978`

### Next — v1.0 (see `ROADMAP.md`)

- [ ] **Correctness:** `total_calories` fix (`src/fit_writer.rs:223`), `env_logger::init()`, dead-code cleanup (`is_loading`/`render_loading`)
- [ ] **Ride control:** `+/-` ERG nudge, `n`/`p` skip step, `e` ERG↔hold (Phase 1)
- [ ] **BT robustness:** `Scan` button in Settings, error banner, `Simulated` qualifier (Phase 2)
- [ ] **History & Stats:** Sessions drill-down (samples replay) + minimal Stats (weekly TSS, PR `1m/5m/20m`) (Phase 3)
- [ ] **Content & FTP:** 4 curated workouts + `TSS|duration` subtitles + `best20×0.95` FTP suggestion (Phase 4, single `ftp_test_20min.zwo` for 1.0)
- [ ] **Export:** Manual `data/.fit` hint in summary; Garmin/Strava/Zwift auto-upload deferred to 1.1 (Phase 5)
- [ ] **Polish:** `?` help overlay, `1.0.0-rc1` bump (Phase 6)

> Full phased plan, competitive gap table, and out-of-scope list: [`ROADMAP.md`](../ROADMAP.md)
