# Versioning, Observability, and Testing

**Status:** Direction agreed; implementation details open, 2026-10-16

## Versioning

Versioning is intended to make debugging and tracing easier. Consider separate versions for:

- Application/build.
- DTML schema/language version.
- DTIR schema version.
- Importer/parser versions.
- Feature definitions and feature pipeline.
- Algorithm version.
- Model artifact and model metadata.
- API contract version.
- Configuration that materially affects results.

Do not collapse all of these into one version number: they evolve at different rates.

## Reproducibility record

For an analytics result, record enough information to identify:

- The input activity or dataset version.
- Relevant source/provenance metadata.
- DTIR schema version.
- Feature pipeline/version.
- Algorithm or model version.
- Material configuration parameters.
- Application/build version where useful.
- Timestamp and trace/job identifier.

Avoid storing unnecessary sensitive raw values in logs. Use stable IDs and references where possible.

## Logging and tracing

- Use structured logs rather than unstructured strings alone.
- Include request/job/trace IDs across GUI/API/import/analytics boundaries.
- Record import and validation errors with enough context to debug them.
- Separate diagnostic metadata from raw training data.
- Redact credentials and sensitive tokens.
- Make logs inspectable from the UI or exportable for debugging where appropriate.

## Testing

### Data-format tests

- Valid and invalid DTML documents.
- Sport-specific validation.
- Unknown extension preservation where supported.
- Round-trip parse/write behavior.
- FIT/GPX samples with missing or unusual fields.

### Analytics tests

- Synthetic data with known expected results.
- Normal sessions and intentionally anomalous cases.
- Missing, noisy, duplicated, and out-of-order measurements.
- Unit conversion and timestamp edge cases.
- Cold-start behavior with little personal history.

### Integration tests

- Product absent, installed-but-unavailable, and available.
- API version mismatch and structured errors.
- Import/export and deletion behavior.
- Local storage failures.

### Resource tests

- CPU-only inference/analysis.
- Reasonable latency and memory usage on a normal target machine.
- Optional acceleration must not change semantics unexpectedly.

## Evaluation metrics

Detailed thresholds can be selected after a prototype exists. Before claiming that anomaly detection is useful, evaluate false positives, false negatives where labels exist, stability across sports/data sources, uncertainty, and behavior with limited history.
