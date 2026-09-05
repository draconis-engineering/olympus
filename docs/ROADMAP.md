# Olympus Roadmap — to 1.0 and Beyond

> **Vision:** *The TrainerRoad for terminals.* A free, offline-first, privacy-minded TUI that drives a Tacx Flux S2 (or any FTMS trainer) with flawless ERG execution and writes a Garmin-valid `.fit` you can drop into Garmin Connect → Strava. No video worlds, no MMO server, no social feed — just perfect workouts.

**Current version:** `0.1.5` (`src/app.rs:467`) · **Target:** `1.0.0-rc1` → `1.0.0`
**Minimum viable 1.0 promise:** Fresh install → pair Flux S2 → pick a workout → ride ERG with live Braille graphs and pause/skip → finish with Save/Discard summary → find a FIT in `data/.fit` that Garmin/Strava accept *and* see the ride in local history/stats. No JSON hand-editing, no restart on Bluetooth hiccup.

This roadmap was cut 2026-09-04 after a competitive pass against Strava / Rouvy / Tacx Training / TrainerRoad / Zwift (see analysis in PR discussion). It is the **locked** build plan.

---

## 0. Competitive Positioning (Why This Roadmap)

| Competitor | What they are best at | What Olympus does **not** try to replicate |
|---|---|---|
| **Strava** | Social feed, segments, 135 M users | Social graph, clubs, Local Legends. Olympus *exports to* Strava; it doesn't host. |
| **Rouvy** | 1 300+ real-video AR routes, Route Creator | Video streaming / AR overlay. Terminal Braille `Marker::Braille` `src/render.rs:41` cannot compete. |
| **Tacx Training** | OEM Flux S2 support + Garmin Connect gateway | Being Tacx-locked. Olympus supports any FTMS trainer and stays offline. |
| **TrainerRoad** | Adaptive AI, Plan Builder, 3 000 workouts | Full AI coaching. Olympus ships a deterministic `best20×0.95` FTP heuristic `src/math.rs:78` — 80 % of the value, 5 % of the effort. |
| **Zwift** | 12 worlds, XP/Drops, 1 000s concurrent racing, drafting | MMO worlds, physics, anti-cheat. Integrate *out* via FIT. |

**Olympus wins on:** precision ERG (`src/ble.rs:464` FTMS `0x03`), accurate NP/IF/TSS/kJ (`src/math.rs:92`, `src/app.rs:634`), `rusqlite` + `samples` retro-analytics (`src/data.rs:137`), headless `tokio` + RPi-in-garage, zero subscription.

---

## 1. What Is Real Today (v0.1.5 inventory)

**Screens** `src/render.rs:308` / `src/app.rs:154` / `src/nav.rs:144`:
- **Main** — ASCII `OLYMPUS` `332`, 6-item nav (`NewRide → start_ride(true)`, Control, Workouts→Database, Settings, Stats, Quit→confirm `121`) + globals `m/c/d/s` `src/app.rs:1028`.
- **Control** `392` — Big Power/HR (`tui-big-text` `467`), Braille history `tail_points()` `41`, zone gauges `726`, Ride Stats `TIME/DIST/ELEV/GRAD/CAL/TSS/IF` `778`, Intervals `w.step_at(elapsed)` `822`, System `BT [STATE]` color `926` + `UPTIME/FTP/MAX HR`, Paused banner `978`. Keys `Space/Enter` `toggle_pause()` `530`, `Q` `open_summary()` `539`.
- **Database** `1008` — two-tab `Workouts|Sessions` `232`; Workouts from `data/workouts/*.zwo|*.erg` `src/data.rs:329`; Sessions table `fit_sessions` 50 newest `src/data.rs:285`; drill-down is no-op `src/app.rs:864`.
- **Settings** `1160` — sidebar `General/Appearance/Bluetooth/System/User` `src/nav.rs:95`; Bluetooth live `state.label()` `1283`, User profile `Name/Weight/Height/FTP/MaxHR` editable with clamped `commit_edit()` `347` + `save_profile()` JSON `119`. General/Appearance/System stubs.
- **Stats** `1399` — stub `Paragraph("Stats\n------")`; `StatsSelection{Overview,Rides}` `124` unused.

**Engine:** `LiveData` `src/app.rs:14` + `power/hr/rpm/vel_history` cap 300 `218`, `recompute_metrics()` `634`, `tick_second()` `713` + `accumulate_distance()` `734` frozen when `is_recording()==Screen::Control && Running` `500`, `CrankTracker` cadence `src/ble.rs:40` (`Δrevs*1024*60/Δtime`), `find_trainer()` FTMS scan `264`, `set_target_power()` `464`, `emit_simulated()` fallback `488`, `.erg` KV `113` + `.zwo` XML `190` (`Warmup/SteadyState/IntervalsT/Cooldown/Ramp`, `≤10→×FTP` `311`), FIT writer `FileId 0 + Record 20 + Session 18` + CRC `src/fit_writer.rs:191` round-trip tested `356`, SQLite `fit_sessions + samples` `137` + `save_ride()` `200`, `finish_ride()` `src/main.rs:231` with `Save/Discard/Resume` overlay `src/render.rs:162`.

**Debt / stubs to fix before 1.0:** `is_loading()` dead `src/app.rs:566` → `render_loading()` `77` never shown; `paused_seconds` `417` never incremented; `ELEV/GRAD/Egain` always `0.0` `src/main.rs:143`; FIT `total_calories=0` `src/fit_writer.rs:223`; `env_logger` never `init()`; `crossbeam-channel`/`uuid`/`serde-xml-rs` shadowed by `xml` unused `Cargo.toml:19`.

---

## 2. v1.0 — The Build Plan

### Phase 0 — Correctness & Hygiene (1–2 days) · *unblocks everything*

- [ ] `src/fit_writer.rs:223` compute `total_calories = calories_kcal(kJ)` from `LiveData.calories` instead of `0`; stamp `start = samples[0].timestamp` not `FitWriter::new()` `128`
- [ ] `src/main.rs:51` call `env_logger::init()` before `init().await` (today `log::error!` is silent)
- [ ] Ensure `data/.fit`, `data/olympus.db`, `data/workouts` on boot (`init_db` `src/data.rs:139` already `create_dir_all` for `data/`; extend)
- [ ] Remove `is_loading`/`render_loading` or wire to `Workout→FTMS ready` gate only (`src/app.rs:566`+`src/render.rs:77` — dead code confuses contributors)
- [ ] Cargo cleanup: drop `crossbeam-channel`, `uuid`, `serde-xml-rs` (keep `xml` `src/erg.rs:193`) `Cargo.toml:19`; add `--help`/`--version` to `resolve_workout()` `src/main.rs:27`

### Phase 1 — Ride Control (the daily-driver gap, 3–4 days)

- [ ] `src/app.rs:969 handle_control_key()` add `+/-` ±5 W nudge to `livedata.target_pwr` + `cmd_tx SetTargetPower`, `n` next step / `p` prev step (jump `elapsed_secs` to `step.end_secs`), `e` toggle ERG↔hold (hold last target; SIM slope deferred — see Phase 7)
- [ ] `src/ble.rs:464` 2–3 s linear ramp on `SetTargetPower` (avoid Flux jolt on 300 W jump)
- [ ] `src/app.rs:713` increment `paused_seconds` while `Paused`; TSS/dist denominator becomes `elapsed - paused_seconds` `src/math.rs:126` (today `paused_seconds` cleared `510` but never counted)
- [ ] `src/render.rs:392` footer hint `[+/-]W [n/p]step [e]ERG [Space]pause [Q]finish` + keep Paused banner `978`
- [ ] Tests in `src/app.rs:1063` for `erg_nudge_applies`, `skip_advances_step`, `paused_excluded_from_tss`

### Phase 2 — Trainer Pairing Robustness (2 days)

- [ ] `Settings → Bluetooth` `Enter → BleCommand::Scan` `src/ble.rs:91` (today static `src/render.rs:1160`)
- [ ] Control footer error banner when `BleState::Error` `src/ble.rs:103` + `Simulated` yellow watermark `src/render.rs:926` already — add qualifier `(simulated — no trainer)` `1283`
- [ ] *(deferred from 1.0 — see below)* second `HRM 0x2A37` peripheral merge for chest strap — keep single-peripheral `find_trainer()` `264` for 1.0; document as 1.1.

### Phase 3 — History & Stats (minimal, 2–3 days)

- [ ] `Database Sessions Enter` `src/app.rs:864` → detail screen (`Screen::Stats` reuse or new `SessionDetail` `src/app.rs:154`) showing `FitSession` fields + Braille `power(t)` vs `target(t)` replay from `samples` `SELECT t,power FROM samples WHERE session_id=? ORDER BY t` `src/data.rs:285` via `line_chart()` `src/render.rs:41`
- [ ] `src/render.rs:1399 stats_draw` replace stub: **weekly TSS bars (last 8 weeks)**, **PR curve `1m/5m/20m`** scanning `samples` (`max(power) WHERE t window`), **volume km/h** — all from `fit_sessions+samples` indexed `samples.session_id` `src/data.rs:185`. No seasons/interval adherence until post-1.0.

### Phase 4 — Content & FTP (2 days)

- [ ] Ship 4 workouts in `data/workouts/`: `ftp_test_20min.zwo` *(the one for 1.0)*, `sweet_spot.zwo` (exists), `vo2max_30_30.zwo`, `recovery.zwo` — all validated via `erg::parse_zwo_workout` `190`
- [ ] Workouts list subtitle `TSS | duration` per row (`TSS ≈ Σ(target/FTP)²·dur/3600·100` on `list_workout_files` load `src/data.rs:329`)
- [ ] Heuristic in `render_summary` `src/render.rs:162`: `best20 = max rolling_mean(power_history,1200)`; if `best20*0.95 > ftp+5` prompt `Update FTP to X? [Y/N]` — deterministic 80 % of TrainerRoad AI Detection, no ML.

> **FTP test choice for 1.0:** Single **20-min test** (`Warmup 10m + 20m all-out + Cooldown`) for simplicity. It reuses existing `rolling_mean(...,1200)` `src/app.rs:498` and `≤10→×FTP` `src/erg.rs:311`. Ramp test added in 1.1.

### Phase 5 — Export (1.0 = manual; bridge in 1.1)

- [ ] Summary hint `FIT ready at data/.fit/ride_*.fit — drag to Garmin Connect (auto-syncs to Strava)` — **no OAuth in 1.0**. Garmin Connect direct upload deferred to 1.1 (it then fans out to Strava, so Strava direct is never needed separately; Zwift direct also 1.1 if desired).
- [ ] Document manual flow in `docs/README.md` (`xdg-open data/.fit`).

### Phase 6 — Polish & Release (½ day)

- [ ] `?` help overlay via `centered_rect` `src/render.rs:58` enumerating `m/c/d/s` globals `src/app.rs:1028` + `Space/Q/+/−/n/p/e` ride keys
- [ ] `render_summary` `src/render.rs:162` FTP suggestion line + export hint
- [ ] Bump `0.1.5` `src/app.rs:467` → `1.0.0-rc1`, `git tag v1.0.0-rc1`

---

## 3. Out of 1.0 — Tracked, Not Built

These are intentionally **not** in 1.0 (see competitive table). Build them after the MVP is solid.

- **GPX / SIM gradient mode** — Requires GPX parsing, gradient→watts physics, Braille elevation profile. 1.0 stays ERG-only; `ELEV/GRAD` `0.0` with `TODO SIM` note. No average cyclist needs it (per project owner, an ambitious rider).
- **Virtual shifting / Cog+Click** — Zwift 2025 moat; needs second FTMS field.
- **Workout creator TUI beyond 4 files, Training plan calendar, Adaptive AI / Plan Builder** — TrainerRoad's moat; needs weeks of history + ML. 1.0 ships static workouts; plans deferred.
- **Power curve history, seasons compare, interval adherence (Rouvy Execution Score), fitness/freshness, VO2max** — Add schema (`load/fatigue` columns) in 1.0, compute in 1.1.
- **Second HR strap / dual-peripheral merge** — Deferred to 1.1 (simplest path: keep single peripheral for 1.0).
- **Mobile / companion app** — `ratatui` `CrosstermBackend` `src/boot.rs:37` is terminal-only; companion is a different product.
- **Multi-user / profiles** — Single `profile.json` + `username` footer `src/render.rs:174` is fine.
- **Garmin/Strava/Zwift auto-upload** — FIT is already Garmin-valid `src/fit_writer.rs:356`; OAuth (`oauth2`/`reqwest`) deferred to 1.1 with token at `data/user/strava.json` (never in repo). Manual upload is 1.0.

## 4. Dropped Entirely — Do Not Build

- **3D / Video / AR worlds** (Zwift 12 worlds, Rouvy 1 300 videos, Tacx 500 HD) — Terminal `Braille` cannot compete; at most a GPX→sparkline later.
- **MMO racing / live opponents / drafting** — Needs server, physics, anti-cheat. Integrate *out* via FIT.
- **Social feed / kudos / clubs / segments hosting** — Strava network effect (135 M users). Olympus uploads *to* Strava; it never hosts.
- **Subscription / marketplace** — Contradicts local SQLite + `profile.json` git-trackable + RPi garage `ssh` promise. Prune `crossbeam-channel`/`uuid`/`serde-xml-rs`.
- **Dead `render_loading()` overlay** `src/render.rs:77` — Drop or keep only as `Workout→FTMS ready` gate.

---

## 5. Risks & Mitigations

- **BT flake on Linux BlueZ** → `Scan` button + `Simulated` fallback `src/ble.rs:488` + heartbeat `253` + `docs/README.md` `bluetoothd -E` guide. Test with `bluetoothctl` on Flux S2.
- **FIT rejected by Garmin** → `fitparser` round-trip `src/fit_writer.rs:356` extended to assert `total_calories` + `start_ts`.
- **TSS inflation when paused** → `paused_seconds` exclusion (Phase 1).
- **Stats perf on 1 000s samples** → `LIMIT` + indexed `samples.session_id` `src/data.rs:185` + pre-aggregated `fit_sessions` summary.

---

## 6. How to Use This Roadmap

- Issues/PRs should reference a Phase (e.g. `Phase 1: +/- ERG nudge`).
- 1.0 is feature-frozen to the checkboxes above; anything else targets `1.1` label.
- When a Phase lands, update `docs/README.md` Feature Status and bump `src/app.rs:467` version.

*Last updated: 2026-09-04 · Owner: @amundgaard · Status: locked for build.*
