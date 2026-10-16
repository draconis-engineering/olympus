# DTML and DTIR

**Status:** DTML MVP scope agreed; DTIR proposed, 2026-10-16

## DTML: current MVP scope

DTML (Draconis Training Markup Languages) is an XML-based family of extensible languages for **planned workouts and training plans**.

The MVP is not intended to use DTML as the primary recording format for completed activities. Completed activities will initially be imported from formats such as FIT and GPX.

### Sport coverage

- Swimming
- Cycling
- Running

### Parsing and writing

Current concept:

- Sport-specific parser/validation logic for each sport.
- Shared XML infrastructure.
- One shared writer/serializer.
- Extensibility for sport-specific and future custom fields.

Open design detail: decide whether each sport truly needs a separate syntax parser or whether a common XML parser plus sport-specific validators is sufficient.

### Planned workout examples

DTML may represent:

- Workout metadata and target sport.
- Workout steps/blocks.
- Repetitions and rest periods.
- Duration or distance targets.
- Intensity targets and units.
- Optional descriptions and annotations.
- Extensions that older implementations can preserve even if they cannot interpret them.

The exact schema is not defined yet.

## DTIR: Draconis Training Intermediate Representation

DTIR is a proposed shared internal representation for normalized training data. It should make it easier for OAE, Olympus, and future Draconis products to exchange and analyze information.

**Recommendation:** DTIR should initially be a set of typed Rust data structures and versioned serialization, not automatically another XML language.

### Core object types

Keep these distinct:

- `PlannedWorkout`: a workout prescription or plan.
- `CompletedActivity`: a recorded activity with observed data.

Avoid one large type where many optional fields make it unclear whether a record describes a plan or something that actually happened.

### Shared fields to consider

- Stable identifier.
- Sport/activity type.
- Start time and timezone/offset where known.
- Duration and distance when available.
- Data source and provenance.
- Units and measurement definitions.
- Data-quality metadata.
- Schema version.
- Optional notes and extensions.

### Completed activity data

Potentially includes:

- Timestamped streams such as heart rate, power, cadence, pace/speed, position, and elevation.
- Summary metrics.
- Source device or service metadata when available.
- Missing/unknown fields.
- Whether a value was directly measured, derived, or estimated.

Do not imply that every activity contains every stream. FIT can carry a broad range of workout/sensor data; GPX often primarily supplies GPS track information and optional elevation/time.

### Data-quality and provenance requirements

Represent the difference between:

- Not provided.
- Unknown.
- Invalid or rejected.
- Estimated.
- Derived from other values.
- Directly measured.

Preserve source attribution and avoid silently fabricating missing measurements.

### Extensions and compatibility

- Define namespacing and versioning rules.
- Preserve unknown extensions during round-tripping where feasible.
- Validate required fields without rejecting all documents that contain a harmless extension.
- Make incompatible schema changes explicit.
- Document unit conventions and timestamp semantics.

### Suggested flow

```text
DTML -> DTML parser/validator -> DTIR PlannedWorkout
FIT  -> FIT importer ---------\
GPX  -> GPX importer ----------+-> DTIR CompletedActivity -> OAE
Strava/manual adapters --------/
```

For planned workouts:

```text
DTIR PlannedWorkout -> shared DTML writer -> DTML
```

## Naming note

The name `DTIR` is a working name from planning. Confirm the project naming and avoid confusing it with other existing Draconis formats such as DTIR/DSML/DRML or any previously discussed training-format names before publishing a spec.
