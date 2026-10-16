# Data, Privacy, Git, and Storage

**Status:** Local-first direction agreed; details open, 2026-10-16

## Local-first default

- Training data is stored locally by default.
- Core functionality should not require cloud storage.
- Users should be able to inspect, export, and delete their data from a dashboard.
- Personal analytics state (such as a personal baseline) must be considered part of the user's data lifecycle.

## Deletion and export

Deletion should account for more than the original activity record:

- Imported source records and normalized records.
- Derived metrics and cached analytics.
- Personal baselines or personalized model state that depends on the deleted data, where appropriate.
- User-visible reports and generated artifacts held by the application.

Explain limits clearly: deleting an item from the app cannot automatically remove copies the user previously exported, committed to Git, or included in an external backup.

Export should preserve useful metadata such as source/provenance, units, and schema version.

## Git integration

Git integration is desired as an easy way to version suitable data and configuration, but should be optional and explicit.

Good candidates may include:

- DTML workout plans.
- Configuration.
- Small, human-readable reports or selected derived artifacts.

Use caution with:

- Large raw sensor files.
- Personal records the user may not want in a repository.
- Secrets, tokens, or private integration credentials.
- Generated model files or caches that are large or reproducible.

Document that deleting a file from the working tree does not necessarily remove it from Git history. Provide clear guidance before enabling tracking of sensitive or bulky records.

## Storage ownership

- Each product owns its writable data store.
- Do not let all Draconis products write to one shared database.
- Share through explicit APIs, versioned file formats, or deliberate import/export flows.
- Keep product-specific caches and runtime state separate from the shared registry/configuration area.

## Privacy and future global models

A global reference model is a possible future concept, not an MVP dependency. Do not upload or reuse local user data for global training without explicit informed opt-in and a clear retention/deletion policy.

## Data model implications

Preserve source and quality information. Missing data should remain missing. Distinguish observed, estimated, and derived values. Make data provenance available in the UI and diagnostic records.
