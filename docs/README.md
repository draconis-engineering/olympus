![](../assets/logo.png)

# OLYMPUS — Cycling in Your Terminal

Olympus is a **free, offline-first indoor-cycling app that runs in a terminal**. Think Zwift or TrainerRoad, but keyboard-driven, in your terminal: it connects straight to your smart trainer over Bluetooth, runs structured workouts with live resistance control, tracks you as you ride, and exports a standard fitness file you can upload to Garmin Connect / Strava. No subscription, no cloud, no account.

[![Built With Ratatui](https://img.shields.io/badge/Built_With_Ratatui-000?logo=ratatui&logoColor=fff)](https://ratatui.rs/) [![Roadmap](https://img.shields.io/badge/Roadmap-v1.0-blue)](ROADMAP.md)

## Quick Start

**What you need**

- A computer on Linux, macOS, or Windows 10+
- Any modern smart trainer with Bluetooth (an HR strap is optional)
- The Rust toolchain (`cargo`)

**Get riding**

1. `cargo run --release`
2. **Settings → Bluetooth** — scan and pair your trainer. An optional heart-rate strap pairs automatically as a second device.
3. **Database → Workouts** — pick a workout and start the ride. Brand-new riders are pointed at the **Ramp Test**, which estimates a starting FTP.
4. Ride on the **Control** panel, then `Q` to finish and **Save** the ride.

That's it. Your rides land as `.fit` files in `data/.fit/` plus a row in the local history database.

## What Olympus Does

- **Drives any modern smart trainer** — it speaks the open Bluetooth Fitness Machine Service, so no trainer brand is special-cased.
- **Runs structured workouts** — `.zwo` / `.erg` files with live ERG resistance: the trainer holds the exact watts of each step (warm-up, intervals, ramps, cooldown).
- **Estimates your FTP** — a Ramp Test that suggests a functional threshold power from your best 60-second effort, applied with one keystroke.
- **Tracks you live** — big power/HR readouts, Braille power/cadence/HR/speed graphs, riding zones, and live NP/IF/TSS/kJ while you pedal.
- **Keeps everything** — per-second samples in local SQLite: session history with drill-down, weekly training load (TSS), and 1m/5m/20m power records.
- **Exports to the platforms you already use** — saves Garmin-valid `.fit` files you can drag into Garmin Connect (auto-syncs to Strava), and **auto-uploads to Strava on Save** when connected (`Settings → Strava` PKCE, `src/strava.rs`), queued offline and retried next boot.

## Hardware & Sensors

| Thing                       | How it works                                                                                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **Smart trainer**           | Connects over Bluetooth as a standard Fitness Machine; Olympus sets the ERG target power so the trainer never freewheels. |
| **Heart-rate strap**        | Optional second Bluetooth device; its readings win over the trainer's relayed HR whenever both are live.                  |
| **No trainer / no sensors** | Everything still works — you just get honest `--` stats instead of fabricated numbers.                                    |

## Controls

While riding (Control panel):

| Key               | Action                                        |
| ----------------- | --------------------------------------------- |
| `Space` / `Enter` | Pause / resume (clock and distance freeze)    |
| `+` / `-`         | Nudge the ERG target up / down by 5 W         |
| `e`               | Toggle auto-follow vs. hold the current watts |
| `n` / `p`         | Skip / go back a workout step                 |
| `Q`               | Finish ride → Save / Discard / Resume         |
| `?`               | Keybind reference overlay                     |

Rider settings (name, weight, height, FTP, max HR) live in `data/user/profile.json` and are edited under **Settings → User**.

## Uploading a Ride

Every saved ride is written as a Garmin-valid FIT activity to `data/.fit/ride_*.fit`.

**Auto-upload (Phase 12):** if Strava is connected (`Settings → Strava` — `STRAVA_CLIENT_ID`/`STRAVA_CLIENT_SECRET` + `cargo run --bin strava-auth`), Olympus uploads the FIT on **Save** via `POST /api/v3/uploads` (`src/strava.rs:370`); offline files are queued at `data/user/strava_queue.json` and retried next boot. Disconnect with `Enter` on the Strava panel (`src/app.rs:1077`).

**Manual:** drag the newest file onto Garmin Connect (auto-syncs to Strava/watch):

```bash
xdg-open data/.fit        # Linux
open data/.fit            # macOS
explorer data\.fit        # Windows
```

Garmin Connect then auto-syncs to Strava. Direct Garmin upload isn't planned (no public API); Olympus only talks to Strava when you connect it — otherwise it stays offline.

## Bluetooth Setup

**Linux (BlueZ):** `btleplug` needs the experimental Bluetooth APIs, so start `bluetoothd` with the `-E` flag:

```bash
sudo systemctl edit bluetooth
# add:
#   [Service]
#   ExecStart=
#   ExecStart=/usr/lib/bluetooth/bluetoothd -E
sudo systemctl restart bluetooth
```

Then bring your adapter up (the app scans on start):

```bash
bluetoothctl power on
bluetoothctl scan on   # optional
```

**macOS / Windows:** no extra setup — just grant Bluetooth permission the first time the app asks.

## Architecture

Olympus is split into a **UI loop** (renders the TUI) and an **async runtime** (talks to hardware), connected by non-blocking message channels — so the interface never blocks real-time data collection.

```
┌─────────────────────────────────────┐
│              OLYMPUS                │
├─────────────────┬───────────────────┤
│    UI LOOP      │     ASYNC RUN     │
│   (Ratatui)     │  (Tokio Runtime)  │
├────────┬────────┴────────┬──────────┤
│ STORAGE ENGINE           │ HARDWARE │
│ (SQLite / FIT)           │  (BLE)   │
└──────────────────────────┴──────────┘
```

- **Storyboard / screens** — Main menu, Control (live ride), Database (workouts + session history), Settings, Stats — all in `src/render.rs`.
- **Ride engine & app state** — `src/app.rs` owns the ride lifecycle, live metrics, and the ERG scheduler.
- **BLE driver** — `src/ble.rs` discovers, connects, and subscribes to trainer + HR strap, and ramps ERG targets smoothly.
- **Workout parsing** — `src/erg.rs` reads `.zwo` (XML) and `.erg` (key-value) files and schedules interval targets.
- **Metrics** — `src/math.rs` computes NP / IF / TSS / rolling power / zone time.
- **Fitness files** — `src/fit_writer.rs` emits Garmin-valid `.fit` activities.
- **Persistence** — `src/data.rs` owns the SQLite database and the JSON rider profile.
- **Strava sync** — `src/strava.rs` handles PKCE OAuth, token refresh, and queued multipart FIT upload; `src/bin/strava-auth.rs` is the one-time connect helper.

### Tech Stack

- **Rust** — memory-safe, low-latency; comfortably drives ~1 Hz ride metrics and 60 fps rendering.
- **Ratatui + Crossterm** — immediate-mode terminal UI.
- **Tokio** — the async runtime; `tokio::sync::mpsc` shuttles live sensor telemetry to the UI.
- **btleplug** — cross-platform BLE (BlueZ / CoreBluetooth / WinRT), subscribing to the standard Cycling Power, Heart Rate, and Fitness Machine GATT profiles.
- **rusqlite** — embedded SQLite for session summaries + per-second samples.
- **serde** — JSON rider profile.

## Running Without a Trainer

Start the app with no sensors paired and Olympus stays honest: the Control panel shows a plain `READY — NO RIDE IN PROGRESS` panel (never fake power/HR numbers), and you can still start a ride and save a FIT. Great for testing on an empty laptop.

## Feature Status

### Shipped (v1.0.0)

- [x] End-to-end ride: BLE acquisition (power / cadence, HR / speed) — trainer + optional HR strap, strap-priority HR merge
- [x] ERG target power pushed to the trainer (Fitness Machine Control Point) over live `Idle / Connecting / Connected / Error` connection states
- [x] `.zwo` / `.erg` parsing + interval scheduling (`Warmup / SteadyState / IntervalsT / Cooldown / Ramp`, `≤10 → ×FTP` scaling; multi-minute `Ramp` expands into a 1-minute ascending staircase for the FTP ramp test)
- [x] Metrics: rolling `5/10/20-min` (+ `3m/1m/30s/10s/3s`) power, NP/IF/TSS/kJ/kcal, distance
- [x] Rider profile (JSON) load/save with a clamped editor
- [x] Garmin-valid FIT writer — `data/.fit/ride_*.fit`
- [x] SQLite session + per-second `samples` persistence
- [x] Database browsing (Workouts + Sessions)
- [x] Settings (Bluetooth / System / User profile)
- [x] Ride lifecycle: pause, summary Save/Discard/Resume, pause-proof stats

### Locked slice · Phases 7–12 (shipped)

Docs as source of truth ✅, trust the numbers ✅, HR-strap merge ✅, FTP ramp test + first-ride onboarding ✅, Strava auto-upload (PKCE + queued retry) ✅ — running as `1.1.0`, tag `v1.1.0`.

### Next — the complete training app (`ROADMAP.md` §2, Phases 11–14)

- [x] **Phase 11 — Distribution & installation:** install wizards shipped — `scripts/install.sh` (Linux/macOS) + `scripts/install.ps1` (Windows) — OS/arch detection, SHA-256 verify, data-home seeding, per-OS Bluetooth notes, re-run to update; release script + future Downloads page still open
- [x] **Phase 12 — Strava auto-upload:** OAuth PKCE (`src/strava.rs` + `src/bin/strava-auth.rs`), queued retry on `Save`, token at `data/user/strava.json` gitignored
- [ ] **Phase 13 — Training depth:** workout creator, plans / fitness-freshness, adherence, virtual shifting
- [x] **Phase 14 — In-app update check:** silent latest-release poll on boot (`src/update.rs`) → banner "re-run scripts/install.sh" (`Esc`/`u` dismiss)

> Full phased plan, competitive gap table, and out-of-scope list: [`ROADMAP.md`](ROADMAP.md)
