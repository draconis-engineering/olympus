# Olympus Roadmap — 1.0 Shipped, dashed toward the Complete Training App

> **Vision:** *The TrainerRoad for terminals.* A free, offline-first, privacy-minded TUI that drives a Tacx Flux S2 (or any FTMS trainer) with flawless ERG execution and writes a Garmin-valid `.fit` you can drop into Garmin Connect → Strava. No video worlds, no MMO server, no social feed — just perfect workouts.

**Current version:** `1.0.0` (`App::version_static()` · `src/app.rs`) · dashed toward `1.1.0`
**1.0 MVP promise — met.** Fresh install → pair trainer → pick a workout → ride ERG with live Braille graphs and pause/skip/nudge/hold → finish with a Save/Discard summary → a FIT lands in `data/.fit` that Garmin/Strava accept → the ride shows up in local history + stats. No JSON hand-editing, no restart on a Bluetooth hiccup.

This roadmap was cut 2026-09-04 after a competitive pass against Strava / Rouvy / Tacx Training / TrainerRoad / Zwift. Phases 0–6 shipped on 2026-09-08; the "complete training app" gap work is tracked here as Phases 7–13.

---

## 0. Competitive Positioning

| Competitor | What they are best at | What Olympus does **not** try to replicate |
|---|---|---|
| **Strava** | Social feed, segments, 135 M users | Social graph, clubs, segments hosting. Olympus *exports to* Strava; it doesn't host. |
| **Rouvy** | 1 300+ real-video AR routes | Video / AR overlay. Terminal Braille cannot compete. |
| **Tacx Training** | OEM Flux S2 support + Garmin Connect gateway | Being Tacx-locked. Olympus supports any FTMS trainer and stays offline. |
| **TrainerRoad / Zwift** | Adaptive AI, 3 000 workouts, virtual worlds | Full AI coaching, MMO racing. Olympus ships a deterministic `best20×0.95` FTP heuristic and integrates *out* via FIT. |

**Olympus wins on:** precision ERG (`SetTargetPower` + `ramp_target` FTMS `0x03/0x2AD9`, `src/ble.rs`), accurate NP/IF/TSS/kJ (`src/math.rs`), `rusqlite` + per-second `samples` retro-analytics (`src/data.rs`), headless `tokio` + RPi-in-garage, zero subscription, terminal-native.

---

## 1. Shipped — 1.0 · Phases 0–6

| Phase | Delivered in | Headline |
|---|---|---|
| **0–2 — Correctness, ride control, BLE.** | `3ef2f08` | FIT `total_calories` from avg power + `start` from first sample; `env_logger::init()`; `ensure_data_dirs()` on boot; dead `render_loading` removed; `--help`/`--version` CLI; deps trimmed (dropped `crossbeam-channel`, `serde-xml-rs`; `uuid` kept for GATT UUIDs). Ride control: `+/-` ±5 W ERG nudge, `n`/`p` skip/prev step, `e` ERG↔hold, control footer key hints, `paused_seconds` accrual, tests. BLE: Settings `Scan` button, error banner, `ramp_target` 2–3 s linear ramp on target changes. |
| **3 — History & Stats.** | `9180f34` | Sessions drill-down (Braille power(t) vs target(t) replay from `samples`); Stats screen: weekly TSS bars (last 8 weeks), PR curve `1m/5m/20m`, volume km/h — all from local SQLite. |
| **4 — Content & FTP.** | `7f71b0b` | 4 curated workouts (`ftp_test_20min`, `sweet_spot`, `vo2max_30_30`, `recovery`); Workouts lists show `TSS \| duration`; summary prompt `best20×0.95 → Update FTP? [Y/N]`. |
| **5 — Export (manual).** | `67f5165` | Summary hint `FIT ready at data/.fit/ride_*.fit — drag to Garmin Connect`; manual export docs. |
| **6 — Polish & release.** | `d8f76a1` | `?` keybind help overlay; bump to `1.0.0-rc1` (app + Cargo.toml); tag `v1.0.0-rc1`. |

Tests: **82 passing**, clippy baseline **18 pre-existing warnings** (dropped 2 with the simulated-data removal). Workouts directory: `data/workouts/*.zwo`.

### What a user can do today
Run `cargo run --release` (optionally `-- path/workout.zwo`), pair any FTMS trainer in Settings → Bluetooth (an HR strap like a Polar H10 pairs automatically as a second device — no trainer attached is fine, the idle Control panel shows `READY — NO RIDE IN PROGRESS` instead of fabricated numbers), pick one of 5 workouts (including the Ramp Test — a fresh install points new riders straight at it) or the FTP test, ride ERG with big-text power/HR, Braille history, zone gauges, live TSS/IF/NP and interval stepping; pause (`Space`), nudge (`+/-`), hold (`e`); finish (`Q`) → Save writes `data/.fit/ride_*.fit` + `data/olympus.db`; browse rides in Database → Sessions with drill-down, and weekly TSS / PRs in Stats.

---

## 2. Build Plan — Phases 7–13 (the complete training app)

> **Locked slice: Phases 7–10 shipped** (source-of-truth → trust → HR → FTP ramp). Phase 10 landed the version gate: **`1.0.0` + tag `v1.0.0`**. Phases 11–13 follow, then `1.1.0`.

### Phase 7 — Docs = source of truth (this commit, small)
- [x] Grep-verify every "shipped" claim before writing (the old checkboxes drifted — Phases 0–2 were done but still `[ ]`).
- [x] Compact Phases 0–6 into the table above; reference code **function-first** (`handle_control_key`, `ramp_target`, `version_static`) so line numbers don't rot.
- [x] **Anti-drift rule:** code landings update ROADMAP/README checkboxes in the *same commit*.
- Files: `docs/ROADMAP.md`, `docs/README.md`. Verify: `cargo test`.

### Phase 8 — Trust the numbers (small, high credibility)
- [x] Pin pause-exclusion end-to-end: `paused_seconds` already freezes clock + accrues (`paused_seconds_excluded_from_ride_time`) — added `tss_frozen_during_pause` asserting TSS's `elapsed_secs` denominator never includes paused time.
- [x] `ELEV/GRAD` are always `0.0` in ERG mode — render `-- (ERG mode)` instead of a confident zero.
- [x] **No made-up data, anywhere:** removed the simulated BLE fallback (`emit_simulated`, `BleState::Simulated`, `rand` dep), the `General`/`Appearance` settings stubs, and the two `(simulated — no trainer)` footers. Control now shows an idle `READY — NO RIDE IN PROGRESS` panel when no ride is running (`render_control_idle`); `c` / Main→Control just navigate, and `Space`/`Enter` on the idle panel starts a ride.
- Files: `src/app.rs`, `src/ble.rs`, `src/math.rs`, `src/render.rs`, `src/main.rs`, `src/nav.rs`, `Cargo.toml`. Verify: new TSS-pause test, idle-render path covered by the existing control smoke tests.

### Phase 9 — HR strap merge (medium–large, biggest data-quality gap)
- [x] Second peripheral `0x2A37` Heart Rate Measurement alongside the FTMS trainer; merged into `LiveData.hr` with source priority (strap wins when present); cadence stays `CrankTracker`.
- [x] `ble.rs`: dual GATT subscription + grouped scan (`find_sensors` classifies peripherals by service UID: FMS/CPS → trainer, HRM-only → strap); `Connected { trainer, strap }` reports both names; Settings + System show the strap row.
- [x] Unit tests: strap-over-trainer priority (`parse_notification` suppress path + end-to-end merge), dropped-strap fallback (trainer HR resumes), no-trainer idle state (Phase 8).
- Risk: multi-peripheral flake on BlueZ → reuse Phase 0–2 reconnect/Scan; keep single-trainer ERG until stable.

### Phase 10 — FTP ramp test + first-ride onboarding (small–medium)
- [x] Ship `data/workouts/ramp_test.zwo` (**Warmup 10 m → 25 m Ramp from 100 W at +20 W/min → Cooldown 5 m**) via the existing `Ramp` parser (`src/erg.rs`): a `Ramp` with `PowerLow`+`RampRate` (or `PowerHigh`) expands to a 1-minute ascending staircase and tags the workout `is_ramp_test`.
- [x] Detection formula: **FTP = 0.75 × best 60-s power**, reusing `best_rolling_mean` (`src/math.rs`) inside `ftp_estimate()` (`src/app.rs`); ramp-test rides switch the summary prompt's window while normal rides keep `best20×0.95` — all through the existing `confirm_ftp` Y/N prompt (Phase 4).
- [x] Onboarding: with no session history + no trainer, Main shows a `NEW RIDER? … Ramp Test` hint under the menu (`main_draw`, `has_ride_history` seeded from the DB at startup); it clears after the first saved ride.
- Verify: `ramp_rate_expands_to_ascending_minute_steps`, `ramp_test_suggestion_uses_best60_times_075`, `onboarding_hint_guides_new_rider_until_history_exists`, and `shipped_workouts_parse_and_have_totals` covers `ramp_test`. Version gate: bump to `1.0.0`, tag `v1.0.0`.

### Phase 11 — Distribution & installation (medium–large)
- [ ] Release script: `cargo build --release` → per-OS artifacts (Windows `.zip` + optional `.msi`, macOS/`.deb`; `cargo install --path .` path documented).
- [ ] **Setup wizard / install scripts** for a future Downloads page: `setup.sh` (Linux/macOS) + `setup.ps1` (Windows) that detect the OS/arch, fetch the matching release artifact, install the binary, create `data/` dirs, write a first-run `profile.json`, and offer the platform packaging hash / signature check.
- [ ] Per-OS Bluetooth setup notes — Windows/macOS (grant Bluetooth permission; no `bluetoothd -E` tweak) alongside the existing Linux guide; the wizard prints the right one for the detected OS.
- [ ] Downloads page (once the website is ready) links the platform artifacts + wizard, with a pinned release hash so the scripts are auditable.
- Verify: release script runs; setup wizard heads for install on an untouched machine (VM smoke test); `cargo install` smoke test; docs accurate.

### Phase 12 — Auto-upload to Strava (large)
- [ ] `reqwest` + `oauth2` (PKCE); token + refresh persisted at `data/user/strava.json`, **gitignored, never committed** (mirrors `profile.json` location).
- [ ] On Save (`finish_ride`) upload the FIT via Strava multipart; offline → queue the file and retry next boot.
- [ ] Strava-first: Garmin Connect has no public upload API, so Garmin→Strava fan-out stays manual; a Zwift bridge is out of scope here.
- Verify: token-refresh unit tests; one manual upload e2e against a real account.

### Phase 13 — Training depth (large, sub-checklist)
- [ ] In-app workout creator (TUI editor → writes `data/workouts/name.zwo`).
- [ ] Plans / weekly calendar reusing `samples` + TSS; `load`/`fatigue` schema columns → fitness/freshness chart.
- [ ] Interval-adherence score (Rouvy-execution style); power-curve history + season compare.
- [ ] Virtual shifting / second FTMS field — staged *after* Garmin accepts a shifting-flagged FIT.
- Each sub-item ships with tests + doc checkbox (Phase 7 rule).

---

## 3. Tracked, Not Built

- **GPX / SIM gradient mode** — needs GPX parsing, gradient→watts physics, Braille elevation profile. ERG stays default; `ELEV/GRAD` show `--` (Phase 8) until then.
- **Adaptive AI / Plan Builder full moat** — the deterministic `best20×0.95` FTP heuristic is the shipped 80 %; ML/plan-generation needs weeks of history and stays beyond.
- **Mobile / companion app** — terminal-only product; a companion is a different product.
- **Multi-user / profiles** — single `profile.json` is fine for 1.0.
- **Garmin Connect direct upload** — no public API; Strava path in Phase 12.

## 4. Dropped Entirely — Do Not Build

- **3D / video / AR worlds** (Zwift 12 worlds, Rouvy 1 300 videos) — terminal Braille cannot compete.
- **MMO racing / live opponents / drafting** — needs server, physics, anti-cheat; Olympus integrates *out* via FIT.
- **Social feed / kudos / clubs / segments hosting** — Strava's network effect; Olympus uploads *to* Strava, never hosts.
- **Subscription / marketplace** — contradicts the local SQLite + git-tracked `profile.json` + RPi-garage promise.

## 5. Risks & Mitigations

- **BT flake on Linux BlueZ** (single peripheral) → `Scan` button + heartbeat (live) + honest idle panel when nothing to read. Phase 9 adds a second peripheral — grow cautiously, keep single-trainer ERG until stable.
- **FIT rejected by Garmin** → `fitparser` round-trip tests (extend to assert `total_calories` + `start`); Phase 12 asserts on the upload API response.
- **TSS inflation when paused** → Phase 8 pins the exclusion as a test.
- **Stats perf on thousands of samples** → `LIMIT` + indexed `samples.session_id` (live); Phase 13 reuses pre-aggregated `fit_sessions`.
- **Strava OAuth token exposure** → token file gitignored, least-privilege scopes, refresh-token rotation, never in logs.
- **Roadmap drift** → Phase 7 rule: same-commit doc updates + grep-verify shipped claims before writing.

## 6. How to Use This Roadmap

- Issues/PRs reference a Phase (e.g. `Phase 9: HR strap merge`).
- 1.0 is shipped; the **locked slice is Phases 7–10**. Anything else targets `1.1` or the backlog beyond.
- When a Phase lands, update `docs/README.md` Feature Status **in the same commit** and bump the version string at `App::version_static()`.

*Last updated: 2026-09-08 · Owner: @amundgaard · Status: 1.0 shipped · Phases 7–10 locked.*