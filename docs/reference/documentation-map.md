# Documentation Map

## 1. How to Use
- Want overall direction quickly: `design-philosophy.md`
- Want requirements quickly: `requirements-summary.md`
- Want CI operations: `ci-gates.md`
- Want metrics format details: `metrics-format.md`

## 2. Detailed Specifications (Root Documents)
- requirements.md: functional/non-functional requirements and acceptance criteria
- workspace-architecture.md: crate split and dependency policies
- rendering-performance-plan.md: optimization roadmap and P0/P1 gates
- api-mapping.md: frozen mapping table from legacy APIs to new APIs
- replacement-strategy.md: migration strategy for replacing legacy engines
- api-swapping-quality-plan.md: Compat/FastPath equivalence test plan

## 3. Operations Documents (docs)
- docs/design-philosophy.md
- docs/requirements-summary.md
- docs/ci-gates.md
- docs/metrics-format.md
- docs/contributing.md
- docs/glossary.md
- docs/architecture-decisions.md
- docs/docs-coverage-review.md

## 4. English Version (docs/en)
- docs/en/index.md
- docs/en/design-philosophy.md
- docs/en/requirements-summary.md
- docs/en/requirements.md
- docs/en/workspace-architecture.md
- docs/en/rendering-performance-plan.md
- docs/en/replacement-strategy.md
- docs/en/api-mapping.md
- docs/en/api-swapping-quality-plan.md
- docs/en/ci-gates.md
- docs/en/metrics-format.md
- docs/en/github-pages.md
- docs/en/glossary.md
- docs/en/contributing.md
- docs/en/architecture-decisions.md
- docs/en/docs-coverage-review.md
- docs/en/documentation-map.md

## 5. Update Rules
- On requirement changes: update requirements.md and requirements-summary.md together
- On API changes: update api-mapping.md and api-swapping-quality-plan.md together
- On CI threshold changes: update .ci/*-thresholds.env and rendering-performance-plan.md together

## 6. GitHub Pages Publishing
- Config file: `mkdocs.yml`
- Workflow: `.github/workflows/docs-pages.yml`
- Top page: `docs/index.md`
- Publishing guide: `docs/github-pages.md`
- In repository settings, set Pages Build and deployment source to `GitHub Actions`
