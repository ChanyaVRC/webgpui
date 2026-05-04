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
- Define minimal API set for `webgpui-compat`
- Migrate one screen in `apps/demo-migration`
- Create comparative benchmarks for legacy/new

## 9. Frozen Reference Documents
- API mapping table (frozen v0.1): `api-mapping.md`
