# Product Scope and MVP

**Status:** Planning draft, 2026-10-16

## Product vision

OAE (Olympus Analytics Engine) is a local-first training analytics engine intended to become part of a broader, free Draconis triathlon ecosystem. The longer-term scope includes swimming, cycling, and running. OAE should remain useful on its own and should not require Olympus to be installed.

## Operating modes

1. **Standalone mode**
    - User can import supported files, use supported integrations such as Strava if implemented, and enter data manually.
    - OAE should not require Olympus to be present.
2. **Olympus-integrated mode**
    - OAE can integrate with Olympus when it is installed and its integration endpoint/service is available.
    - Current Olympus scope is cycling only.
    - Until Olympus supports swimming and running, those activities can come from imports, manual entry, and any supported external integration.

## MVP principles

- Local-first: training data remains on the user's machine by default.
- CPU-first: analytics and inference must work without a dedicated GPU.
- Optional acceleration: use available hardware acceleration where practical, for AI and classical analysis, without making it a requirement.
- Incremental scope: do not require JEPA, multiple neural networks, complete triathlon support, or every external integration to ship the first useful version.
- Observable and reproducible: results should be traceable to data, feature, algorithm/model, and schema versions.
- Extensible: the architecture should allow additional sports, file formats, and Draconis products without redesigning the core.

## Suggested MVP vertical slice

A narrow end-to-end slice is preferable to building every subsystem in parallel:

1. Import one completed-activity format reliably (FIT is a candidate; GPX may be a simpler complementary format).
2. Validate and normalize the activity into DTIR.
3. Store and retrieve it locally, retaining source/provenance and data-quality information.
4. Compute a small set of deterministic metrics when source data supports them.
5. Establish a basic personal baseline, with uncertainty/low-confidence behavior when history is limited.
6. Detect and explain a small class of anomalies using a transparent statistical method.
7. Export and delete user data, including personal baseline state where applicable.
8. Run correctness and resource tests on a normal CPU machine.

DTML's MVP is for **planned workouts**, not completed activity recordings. FIT/GPX import covers completed activities separately.

## MVP acceptance ideas

- Missing metrics remain missing rather than being silently invented.
- Unsupported or partially supported fields are reported clearly.
- An anomaly result can explain which input features contributed and what data/algorithm version was used.
- Cold-start behavior is explicit; the system does not overstate confidence when little history exists.
- A user can inspect, export, and delete stored data.
- No cloud account is required for core local analytics.

## Not required for the first MVP

- JEPA or another specific neural architecture.
- A global model trained from user data.
- Full swim/bike/run analytics parity on day one.
- A full integration marketplace.
- GPU-only execution.
- Git tracking of all raw activity data by default.

## Existing-code audit before implementation

Review the current OAE workspace before creating more crates:

- Existing `oae-analytics` functionality.
- Existing `oae-api` boundaries and current API contract.
- Current localhost GUI and how it communicates with the backend.
- What DracoLIX currently implements and where benchmarks show it is useful.
- Existing tests, storage choices, and data types.

Avoid duplicating functionality already present.
