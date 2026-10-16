# AI and Analytics Architecture

**Status:** Direction agreed; implementation details open, 2026-10-16

## Guiding principles

- Use classical statistics and deterministic analytics first where they solve the problem.
- Keep CPU inference and CPU analytics as baseline requirements.
- Hardware acceleration is optional, not a dependency.
- Do not require JEPA or a large neural model for the MVP.
- Explain results and retain enough metadata to reproduce or debug them.
- Be careful not to present correlation or an anomaly as a medical diagnosis or definitive judgment about an athlete.

## Logical model roles

The long-term concept includes three logical levels:

1. **Global reference model**
    - Captures broad patterns across suitable, lawfully usable data.
2. **Athlete archetype/reference profiles**
    - Provides context for different sports, training backgrounds, or experience levels.
3. **Personal adaptive model**
    - Learns an individual's own baseline over time.

These are logical roles, not a requirement for three separate neural networks. MVP implementations can use statistics, robust baselines, and transparent rules.

## Suggested progression

### Phase 1: deterministic foundations

- Normalize data and units.
- Compute reliable metrics only when the necessary inputs exist.
- Add data-quality checks.
- Build a basic personal baseline with explicit cold-start behavior.
- Use simple, explainable anomaly detection.

### Phase 2: evaluated adaptive analytics

- Evaluate false positives and false negatives on labelled/synthetic cases.
- Compare methods across sports and data availability levels.
- Add uncertainty and more robust personalization.
- Introduce more complex models only when they demonstrate measurable value.

### Phase 3: optional learned models

- Train models separately from routine local inference when useful.
- Evaluate export/runtime options, including ONNX where compatible.
- Consider a Rust runtime integration such as ONNX Runtime; keep the implementation choice benchmark-driven.
- Treat DracoLIX as an option for workloads where benchmarks show a real benefit, not as a requirement to replace every library.

## JEPA note

JEPA is an architecture family, not a product feature or an automatic fit for training time-series. Image/video-oriented variants are not ready-made triathlon analytics models. Revisit only if a concrete use case and suitable data justify it.

## Data sources and datasets

- Public datasets can support experimentation, but check licensing, provenance, measurement definitions, quality, and whether they match the target sport/use case.
- Do not assume one dataset provides synchronized, complete triathlon data.
- Synthetic datasets with known ground truth are valuable for testing anomaly detection and edge cases.
- Do not train a global model from users' local data without explicit opt-in and a clear data-handling design.

## Evaluation questions to settle later

- What is a useful anomaly, and who labels it?
- What false-positive rate is acceptable for each use case?
- How should confidence be communicated?
- How does behavior change with little personal history?
- How do we evaluate different sports, sensors, and missing-data patterns?
- What latency, memory, and storage budget should be targeted?

Evaluation thresholds can wait until the prototype exists, but test design should begin early.
