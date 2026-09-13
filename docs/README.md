# OLYMPUS - TUI Cycling Trainer App (Minimalist Zwift/Rouvy Replacement)

Olympus is a minimalist, high-performance **TUI** for indoor cycling. The application connects directly to smart trainers and fitness sensors via **BLE**, renders real-time telemetry with high-res terminal graphics, schedules `.erg`/`.zwo` workouts, and writes Garmin-valid `.fit` files — no subscription, no cloud.

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/) [![Roadmap](https://img.shields.io/badge/Roadmap-v1.0-blue)](ROADMAP.md)

## Architectural Overview

The application utilizes a decoupled, multi-threaded design powered by an asynchronous runtime to ensure user interface rendering never blocks real-time hardware data collection.

```markdown
┌─────────────────────────────────────┐
│              OLYMPUS                │
├─────────────────┬───────────────────┤
│    UI LOOP      │     ASYNC RUN     │
│   (Ratatui)     │  (Tokio Runtime)  │
└───────┬─────────┴────────┬──────────┘
        │                  │
        ▼                  ▼
┌────────────────┐ ┌──────────────────┐
│ STORAGE ENGINE │ │   HARDWARE I/O   │
│  (SQLite/FIT)  │ │ (Btleplug/ANT+)  │
└────────────────┘ └──────────────────┘
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

## Target UI Blueprint

The terminal view runs at 60 fps (1 Hz ride-engine tick) using Unicode Braille for rich visual density:

- **Header/Footer** (`src/render.rs:287`/`250`): `olympus` + version + local time; footer highlights current page and rider name.
- **Main** (`src/render.rs:308`): ASCII `OLYMPUS` slant logo + 6-item nav (`New Ride` → Control, Workouts → Database, Settings, Stats, Quit with confirm `src/render.rs:121`).
- **Control** (`control_draw`): Big Power/HR (`tui-big-text`), Braille history for power/HR/cadence/speed `tail_points()`, zone gauges, Ride Stats `TIME/DIST/ELEV/GRAD/CAL/TSS/IF` (`ELEV`/`GRAD` read `-- (ERG mode)` when a trainer reports none), Intervals `w.step_at(elapsed)`, System `BT [STATE]` color (`Connected` green / `Error` red), `STRAP` row shows the paired HR strap when live. With no ride in progress it swaps the live dashboard for an idle `READY — NO RIDE IN PROGRESS` panel (`render_control_idle`). Overlays: `Paused` banner, end-of-ride Summary, and a global keybind reference on `?`.
- **Database** (`src/render.rs:1008`): Two-tab `Workouts` (`data/workouts/*.zwo|*.erg` `src/data.rs`) and `Sessions` (`fit_sessions` `src/data.rs`).
- **Settings** (`src/render.rs`): `Bluetooth` (live `state.label()`, trainer + optional HR-strap rows) / `System` / `User` profile editor — placeholder `General`/`Appearance` panels were dropped in Phase 8.
- **Stats** (`src/render.rs:1399`): weekly TSS bars (last 8 weeks), power-curve PR `1m/5m/20m`, volume (km / hours / km·h) — computed once from the `samples` table with the rider's FTP.

## Build & Run (v1.0.0)

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

> **Roadmap to 1.0:** See [`ROADMAP.md`](ROADMAP.md) for the locked build plan (phases 0–6, out-of-scope, and competitive positioning vs. Strava/Rouvy/Tacx/TrainerRoad/Zwift).

### Exporting rides (manual — no OAuth in 1.0)

Each saved ride is written as a Garmin-valid FIT activity to
`data/.fit/ride_*.fit` (plus its summary in `data/olympus.db`). To get a ride
onto a device/account, open the folder and drag the newest file onto Garmin
Connect:

```bash
xdg-open data/.fit        # Linux
open data/.fit            # macOS
explorer data\.fit        # Windows
```

1. Drag the newest `ride_*.fit` into your browser at <https://connect.garmin.com>.
2. Garmin Connect auto-syncs to Strava (and your watches) from there.
3. Direct Garmin upload is not planned (Garmin has no public upload API);
   **Strava auto-upload** (OAuth) is Phase 12 in the roadmap — until then
   Olympus never talks to a cloud service.

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

If no trainer/sensor is paired the Control panel stays on its idle
`READY — NO RIDE IN PROGRESS` screen — Olympus never fabricates data, so you
won't see misleading power/HR numbers. Mac/Windows need no extra setup beyond
granting Bluetooth access.

### Running without Bluetooth

The app runs happily with no trainer attached. The idle Control panel shows a
starting ride with `Enter`/`Space`, and FIT/SQLite persistence still works once
you ride — you simply get honest `--` stats instead of invented telemetry.

## Feature Status

### Shipped (v1.0.0)

- [x] End-to-end ride: BLE acquisition (power / cadence via `CrankTracker`, HR / speed) — dual peripheral: FTMS trainer + optional HR strap (`0x2A37`) with strap-priority HR merge (`find_sensors`, `parse_notification`)
- [x] ERG target power pushed to trainer (FTMS `0x2AD9` `set_target_power`, with `Idle/Scanning/Connecting/Connected/Error` states)
- [x] `.erg` and `.zwo` workout parsing + interval scheduling (`Warmup/SteadyState/IntervalsT/Cooldown/Ramp`, `≤10→×FTP` scaling; multi-minute `Ramp` expands into a 1-minute ascending staircase for the ramp FTP test)
- [x] Metrics: rolling `5/10/20-min` (+`3m/1m/30s/10s/3s`) power, NP/IF/TSS/kJ/kcal, distance (`src/math.rs`, `LiveData`)
- [x] Rider profile (JSON) load/save with clamped editor
- [x] FIT activity writer — Garmin-valid (`src/fit_writer.rs`, `data/.fit/ride_*.fit`)
- [x] SQLite session + per-second `samples` persistence (`src/data.rs`, `save_ride()`)
- [x] Database browsing (Workouts + Sessions)
- [x] Profile & Bluetooth settings (`src/render.rs`)
- [x] Ride lifecycle: `Running/Paused/Summary` — pause (`Space`), summary (`Q` → `Save/Discard/Resume`), `is_recording()` gate, `Paused` banner

### The 1.0 roadmap is shipped — Phases 0–6 all landed (see `ROADMAP.md` §1)

Correctness + ride control + BLE (`3ef2f08`), History & Stats (`9180f34`),
Content & FTP (`7f71b0b`), manual Export (`67f5165`), Polish + `1.0.0-rc1`
(`d8f76a1`).

### Next — the complete training app (`ROADMAP.md` §2, Phases 7–13)

- [x] **Locked slice · Phases 7–10:** docs as source of truth ✅, pause-TSS/ELEV trust fixes ✅, HR-strap merge ✅, FTP ramp test + first-ride onboarding ✅ (`ramp_test.zwo`, `ftp_estimate()`) — running as `1.0.0`, tag `v1.0.0`
- [ ] **Phase 11 — Distribution & installation:** release binaries, `setup.sh`/`setup.ps1` install wizard (OS/arch detection, checksum verify), per-OS Bluetooth notes + Downloads page
- [ ] **Phase 12 — Strava auto-upload:** OAuth (PKCE), queued retry, token gitignored
- [ ] **Phase 13 — Training depth:** workout creator, plans/fitness-freshness, adherence, virtual shifting

> Full phased plan, competitive gap table, and out-of-scope list: [`ROADMAP.md`](ROADMAP.md)
