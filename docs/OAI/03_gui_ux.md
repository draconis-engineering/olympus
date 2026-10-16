# GUI and User Experience

**Status:** Proposed, 2026-10-16

## Current direction

- OAE currently has a localhost web GUI.
- Keep it for the MVP rather than introducing a native GUI rewrite prematurely.
- Maintain a clear API boundary between the GUI and the backend.
- A native desktop wrapper (for example, Tauri) can be considered later if it improves distribution or operating-system integration.

## Core dashboard areas

1. **Overview**
    - Recent activities and trends.
    - Status of data imports and integrations.
    - Clear indicators when data is incomplete or a metric is unavailable.
2. **Activities**
    - Browse completed activities.
    - Filter by sport, date, source, and available measurements.
    - Inspect data provenance and quality.
3. **Planned workouts**
    - View and manage DTML-based training plans.
    - Validate plans and show useful line/field-level errors.
4. **Analytics**
    - Metrics, personal baselines, anomalies, confidence/uncertainty, and explanations.
    - Separate observed facts from estimated or inferred values.
5. **Data management**
    - Inspect stored records and data sources.
    - Export selected data.
    - Delete selected activities, imports, analytics outputs, and personal baseline/model state as applicable.
6. **Integrations**
    - Show whether Olympus is not installed, installed but unavailable, or available.
    - Show connection/import status and permissions.
7. **Diagnostics**
    - Application, schema, algorithm/model, and feature versions.
    - Useful logs and trace identifiers, with sensitive data excluded or redacted.

## UX principles

- Explain why an anomaly was flagged; avoid presenting a score with no context.
- Show uncertainty, especially when personal history is limited.
- Make missing data visible rather than quietly filling it in.
- Deletion and export should be discoverable and understandable.
- Avoid requiring users to understand AI internals to use analytics.
- Keep privacy-sensitive actions explicit.

## Security note

A localhost UI/API still needs safeguards:

- Bind only to the intended local interface.
- Avoid permissive cross-origin behavior.
- Validate requests and imports.
- Consider a per-session token or equivalent protection for sensitive actions.
- Do not expose private training records in logs by default.

These are design considerations to validate against the current implementation, not a claim about its current security state.
