# System Architecture

**Status:** Proposed architecture, 2026-10-16

## High-level data flow

```text
Planned workout:
DTML document -> DTML parser/validator -> DTIR PlannedWorkout
              -> analytics/planning consumers
DTIR PlannedWorkout -> shared DTML writer -> DTML document

Completed activity:
FIT file ----> FIT importer ---\
GPX file ----> GPX importer ----+--> DTIR CompletedActivity --> OAE analytics
Strava ------> adapter --------/
Manual entry -> adapter -------/

OAE analytics -> metrics / personal baseline / anomaly explanations
               -> local storage / dashboard / export
```

The format-specific inputs should normalize into a common internal representation before analytics logic consumes them.

## Architectural layers

1. **Input and adapters**
    - DTML parser and sport-specific validation for planned workouts.
    - FIT, GPX, Strava, manual-entry, and later other import adapters for completed activities.
2. **DTIR**
    - Shared internal data model.
    - Separate planned-workout and completed-activity types.
    - Versioned schema and extension rules.
3. **Analytics core**
    - Deterministic metrics and feature extraction.
    - Personal baselines and uncertainty.
    - Anomaly detection and explanations.
4. **Storage**
    - Local activity store, configuration, model state, and results.
    - Explicit export/deletion workflows.
5. **API and product integration**
    - Versioned contracts for GUI, Olympus, and future Draconis tools.
    - Product/service discovery through a registry rather than a shared writable database.
6. **GUI**
    - Localhost web GUI can remain for the early phase.
    - Keep a stable backend/API boundary so a native shell such as Tauri can be considered later.

## Suggested crate layout (illustrative only)

```text
crates/
├── dtir/                 # Internal types and schema/version definitions
├── dtml-core/            # Shared XML infrastructure
├── dtml-swim/            # Swim-specific parse/validation rules, if needed
├── dtml-cycle/           # Cycling-specific parse/validation rules, if needed
├── dtml-run/             # Run-specific parse/validation rules, if needed
├── dtml-writer/          # Shared DTML serialization
├── oae-analytics/        # Metrics, features, baselines, anomaly detection
├── oae-api/              # API contracts and integration endpoints
└── oae-app/              # Application composition, if appropriate
```

Do not create all of these crates automatically. Start with clear module boundaries and split into crates when independent reuse, ownership, or build boundaries justify it.

## Parser nuance

Three sport-specific parser entry points may be useful if the XML structures truly differ. If the languages share a common XML grammar and differ mostly in domain rules, a shared syntax parser plus three sport-specific validators may be simpler. Decide after reviewing the current DTML design.

## Key boundaries

- DTIR should not depend on OAE; Olympus and other Draconis tools may need it independently.
- Analytics should consume normalized typed data, not parse XML/FIT/GPX itself.
- Each product owns its data. Products may expose explicit APIs, but should not casually write to another product's database.
- Keep local service exposure minimal and avoid treating localhost as automatically secure.
