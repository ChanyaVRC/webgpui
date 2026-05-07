# Legacy WebUI Engine Replacement Strategy

## 1. Purpose
Replace the existing WebUI engine incrementally with `webgpui`, while preventing regressions in functionality, quality, and performance.

## 2. Replacement Policy
1. Avoid big-bang migration
- Migrate in stages by screen or feature unit
- Temporarily allow old/new engine coexistence

2. Build compatibility layer first
- Receive legacy API calls via `webgpui-compat`
- Translate internally to `webgpui` APIs

3. Measure optimization while migrating
- Measure FPS, CPU, GPU against legacy
- Switch first on screens where improvement is confirmed

## 3. Compatibility Scope (MVP)
- Node: container, text (basic), image (placeholder)
- Style: position, size, margin, padding, background, border, opacity
- Event: click, pointer move, key down/up
- Lifecycle: mount/update/unmount equivalents

## 4. Non-Compatibility Management
- Emit explicit warnings for unsupported APIs
- For behavior differences, record reason and workaround in migration notes
- Mark out-of-scope compatibility items with feature flags

## 5. Staged Migration Process
1. Inventory
- Extract APIs and styles used by existing screens

2. Mapping
- Build mapping table from legacy APIs to `webgpui-compat` APIs

3. Small-screen PoC
- Validate visual match, input match, and performance metrics

4. Horizontal rollout
- Migrate by groups of similar components
- Manage remaining work as unsupported-item list

5. Completion judgment
- Confirm compatibility ratio, reproduction ratio, and speed metrics satisfy thresholds

## 6. KPI for Completion
- API replacement ratio: >= 80%
- Screen reproduction ratio: >= 90%
- Average FPS: equal to or higher than legacy
- P95 frame time: equal to or lower than legacy
- Critical defects: 0

## 7. Risks and Mitigations
- Risk: visual breakage due to style compatibility gaps
- Mitigation: introduce visual regression tests and screenshot comparisons for major screens

- Risk: interaction feel changes due to event propagation differences
- Mitigation: lock capture/bubble order in tests and absorb differences in compatibility layer

- Risk: no performance improvement on some screens
- Mitigation: isolate bottlenecks with profiler and split optimization targets by screen

## 8. Immediate Actions
- Define minimal API set for `webgpui-compat` (→ completed in api-mapping.md §13)
- Migrate one screen in `apps/demo-migration`
- Create comparative benchmarks for legacy/new

## 9. Frozen Reference Documents
- API mapping table (frozen v0.1): `api-mapping.md`

## 10. M4 Execution Plan

### 10.1 `apps/demo-migration` Structure
```
apps/demo-migration/
  Cargo.toml
  src/
    main.rs          — app entry point; selects scene via CLI arg
    scenes/
      mod.rs
      screen_a.rs    — Screen A: container + text + event (simplest)
      screen_b.rs    — Screen B: list + dynamic update + keyboard nav
    metrics.rs       — records migration cost (line count, unsupported API count)
    compare.rs       — runs legacy and new side by side; outputs frame-time delta
```
Both `screen_a` and `screen_b` must build against `webgpui-compat` only; no direct `webgpui-core` calls.

### 10.2 Visual Regression Testing
Tool: `insta` (snapshot testing) or a custom PNG diff harness.
- Each scene renders N frames offline; the first stable frame is saved as the reference snapshot.
- On CI, the scene re-renders and diffs against the reference (pixel diff threshold: <= 1%).
- Snapshots stored in `apps/demo-migration/snapshots/`.
- Known acceptable differences (e.g., font antialiasing) documented in `KNOWN_DIFFS.md` alongside snapshots.

### 10.3 Comparative Benchmark
Run with `--benchmark compare` flag:
1. Render Scene A and B with the legacy engine for 300 frames; record avg/p95/draw-calls.
2. Render the same scenes with `webgpui-compat` for 300 frames; record same metrics.
3. Output a Markdown table to stdout and to `migration-report.md`.

Acceptance: new engine avg frame time <= legacy, p95 <= legacy, draw-calls <= legacy.

### 10.4 Migration Cost Measurement
Tracked in `metrics.rs`:
- Lines of app code changed (manual count in PR description)
- Number of MUST-tier API call sites converted
- Number of `UNIMPLEMENTED` stubs remaining (unsupported API count)

These values are reported in the M4 completion PR description.

### 10.5 Equivalence Test Scenarios (Reference to api-swapping-quality-plan.md §4)
| Scenario | Coverage |
|---|---|
| Basic Shapes | container + rect + opacity + 3 resizes |
| Interactive Panel | hover / click / key + focus movement |
| Dynamic List | append/remove/update + frequent dirty-rect |
| Stress Batch | large count of same-style elements + draw-call verification |
