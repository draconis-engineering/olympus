# Draconis / Olympus Adaptive Intelligence — Planning Notes

**Status:** Architecture sketch, 2026-10-16  
**Purpose:** Capture the current planning decisions and open questions before implementation.  
**Important:** This is a planning snapshot, not a final technical specification. Items marked _proposed_ still need validation against the existing codebase.

## Documents

- [Product scope and MVP](01-product-scope-mvp.md)
- [System architecture](02-system-architecture.md)
- [GUI and user experience](03-gui.md)
- [AI and analytics architecture](04-ai-analytics-architecture.md)
- [DTML and DTIR](05-dtml-dtir.md)
- [APIs and Draconis integration](06-apis-and-integration.md)
- [Data, privacy, Git, and storage](07-data-privacy-storage.md)
- [Versioning, observability, and testing](08-versioning-observability-testing.md)
- [Open questions and decisions](09-open-questions.md)

## Current direction in one paragraph

OAE (Olympus Analytics Engine) should work as a standalone, local-first training analytics application while optionally integrating with Olympus and other Draconis products. It should support swimming, cycling, and running over time.
DTML is planned as an XML-based **training-plan format** for the MVP, with sport-specific parsing/validation for swimming, cycling, and running and a shared writer. Completed activities are imported separately, initially from FIT and GPX (and potentially Strava or manual entry). Inputs should normalize into a versioned internal representation called DTIR (Draconis Training Intermediate Representation), with distinct types for planned workouts and completed activities. CPU inference and classical analysis are baseline requirements; hardware acceleration is optional.
Local storage and user-controlled deletion are core principles. Git integration is optional and explicit. Model, algorithm, feature, and schema versions should make results easier to trace and reproduce.

## Status vocabulary

- **Decided direction:** Explicitly agreed in the planning conversation.
- **Proposed:** Recommended architecture that still needs review.
- **Open:** Requires a later decision.

# Open Questions and Decisions

**Status:** Living decision log, 2026-10-16

## Direction agreed in planning

- OAE should be standalone-capable and local-first.
- OAE can integrate with Olympus when Olympus is installed and available.
- Olympus currently supports cycling; swimming and running can enter OAE via imports, manual entry, and supported integrations until native support exists.
- DTML's MVP is a plan/workout description format, not the main completed-activity recording format.
- Completed activities are initially imported from FIT and GPX; other adapters such as Strava and manual entry may be added.
- DTML is XML-based and targets swimming, cycling, and running, with extensibility.
- The current DTML concept includes sport-specific parsing/validation and a shared writer.
- DTIR is proposed as the shared internal representation; planned workouts and completed activities should be separate object types.
- Local storage and user-controlled inspection/deletion are core requirements.
- Git integration is desired but should be optional and explicit.
- CPU inference and classical analytics are required; hardware acceleration is optional.
- Model, algorithm, feature, schema, and relevant API versions should support tracing/debugging.
- Detailed evaluation thresholds can be settled later, but testing should start early.

## Important open questions

### DTML / DTIR

- Q: Do the three sports need distinct syntax parsers, or a shared XML parser plus sport-specific validators?
  A: ...
- What is the initial DTML schema for workout steps, repeats, rest, targets, and units?
  A: ...
- What are the namespace and unknown-extension preservation rules?
  A: ...
- Which DTIR fields are required, optional, or represented as unavailable?
  A: ...
- What serialization format, if any, should be used for persisted DTIR data?
  A: ...
- Confirm final naming to avoid conflicts with other Draconis format names.
  A: ...

### MVP

- Which exact import path is the first end-to-end slice: FIT, GPX, or both?
  A: ...
- Which small set of metrics is in scope?
  A: ...
- Which anomaly class should be the first explainable feature?
  A: ...
- What target machine/resource budget should be used for tests?
  A: ...

### Analytics / AI

- What constitutes a useful anomaly for each sport and use case?
  A: ...
- Which public datasets are licensed and suitable?
  A: ...
- What personal-history threshold should affect confidence?
  A: ...
- When, if ever, is a global model worth implementing?
  A: ...
- Which workloads, if any, justify DracoLIX over established optimized libraries/runtimes?
  A: ...
- Is a model runtime such as ONNX Runtime useful for the selected model?
  A: ...

### GUI

- Which dashboard views are essential for MVP?
  A: ...
- Should the current localhost web GUI remain the only UI for the first release?
  A: ...
- What authentication/CSRF/origin protections are appropriate for the local API?
  A: ...

### APIs / integration

- REST/HTTP, local IPC, or hybrid?
  A: ...
- What exact contract will Olympus expose?
  A: ...
- How will product discovery and API version negotiation work?
  A: ...
- Which platform-specific directories should the Draconis registry use?
  A: ...

### Storage / Git

- Which records are stored as files versus a local database?
  A: ...
- Which data types may Git track by default, if any?
  A: ...
- How should deletion interact with derived data and personalized model state?
  A: ...
- How should users be warned about Git history and external backups?
  A: ...

## Recommended next actions

- [ ] Audit the current OAE code before changing structure.
- [ ] Locate/review existing DTML design and any prior parser/writer work.
- [ ] Draft DTIR v0.1 with distinct `PlannedWorkout` and `CompletedActivity` types.
- [ ] Select the first import format and build a tested adapter.
- [ ] Define a minimal analytics vertical slice with synthetic test data.
- [ ] Write down a small set of MVP acceptance criteria and a target CPU/memory budget.
