# Design Philosophy

## 1. Product Axes
This engine is designed to balance two goals: replacement viability for existing WebUI engines and rendering speed.

- Compatibility: support staged migration from legacy APIs
- Speed: continuously improve with FastPath and measurement-driven work
- Maintainability: enforce crate split and responsibility separation

## 2. Principles
### 2.1 Measure First
- No optimization without measurement
- CPU/GPU metrics drive decisions
- Improvements are enforced by CI gates

### 2.2 Incremental Migration
- Start with Compat path, then swap hotspots to FastPath
- Allow old/new engine coexistence during migration

### 2.3 API Stability
- Public API is exposed through the facade layer
- Semver and migration notes are mandatory
- MUST APIs require equivalence tests

### 2.4 Performance-First Order
- P0: profiling + hot path
- P1: batching
- P2: partial redraw
- P3: transfer/cache optimization
- P4: parallelization and structural optimization

## 3. Architecture Principles
- Maintain one-way dependencies and prohibit cyclic dependencies
- Limit low-level optimization to necessary areas only
- Consolidate shared types in the geometry layer to avoid duplication

## 4. Quality Assurance Principles
- Guarantee Compat/FastPath equivalence by tests
- Prevent regressions across four axes: visual, input, state, and performance
- Operate P0/P1 CI gates first and keep thresholds explicit

## 5. References
- Requirements: requirements.md
- Architecture proposal: workspace-architecture.md
- Performance plan: rendering-performance-plan.md
- API mapping: api-mapping.md
- API swap quality: api-swapping-quality-plan.md
