# Contributing Guide

See [Semver Policy](semver-policy.md) for version bump rules and the
deprecation process.

## 1. Branch and PR Workflow
- Use feature branches for implementation changes
- Include purpose, technical changes, and measurement outcomes in PRs
- P0/P1 related changes must pass CI gates

## 2. Mandatory Co-Updates
- Requirements changes: update requirements docs and summary docs
- API changes: update mapping and equivalence-test docs
- Threshold changes: update threshold files and performance plan

## 3. Local Validation
```bash
uvx --with mkdocs-material mkdocs build --strict
scripts/ci/check_p0_gate.sh .ci/p0-metrics.txt .ci/p0-thresholds.env
scripts/ci/check_p1_gate.sh .ci/p1-metrics.txt .ci/p1-thresholds.env
```

## 4. Items to Include in PR Template
- What was optimized (target path: Compat/FastPath)
- Which metrics improved (avg/p95/draw calls)
- Whether equivalence tests are impacted
- Whether thresholds changed and why
