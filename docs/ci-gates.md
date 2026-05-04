# CI Gate Operations Guide (P0 / P1)

## 1. Purpose
Automatically judge rendering optimization progress at PR time.

- P0: guarantees minimum FastPath performance
- P1: guarantees batching effect numerically

## 2. Target Workflows
- P0: `.github/workflows/p0-gate.yml`
- P1: `.github/workflows/p1-gate.yml`

## 3. Execution Timing
- `pull_request`
- `workflow_dispatch`

## 4. Input Metrics
### 4.1 P0
- Metrics file: `.ci/p0-metrics.txt`
- Threshold file: `.ci/p0-thresholds.env`
- Check script: `scripts/ci/check_p0_gate.sh`

### 4.2 P1
- Metrics file: `.ci/p1-metrics.txt`
- Threshold file: `.ci/p1-thresholds.env`
- Check script: `scripts/ci/check_p1_gate.sh`

## 5. Metrics Generation
### 5.1 Default
Workflows run the following commands.

- P0: `cargo run -p demo-basic -- --benchmark p0 --output .ci/p0-metrics.txt`
- P1: `cargo run -p demo-basic -- --benchmark p1 --output .ci/p1-metrics.txt`

### 5.2 Custom Commands
Can be overridden with environment variables.

- P0: `P0_METRICS_COMMAND`
- P1: `P1_METRICS_COMMAND`

Example:
```bash
P1_METRICS_COMMAND='cargo run -p demo-basic --release -- --benchmark p1 --output .ci/p1-metrics.txt'
```

## 6. Local Pre-Check
```bash
scripts/ci/check_p0_gate.sh .ci/p0-metrics.txt .ci/p0-thresholds.env
scripts/ci/check_p1_gate.sh .ci/p1-metrics.txt .ci/p1-thresholds.env
```

## 7. First Response on Failure
1. Check for missing metrics keys
2. Check whether thresholds are too strict
3. Check if recent changes increased draw calls / submit calls / CPU build time
4. If needed, re-measure with fixed scene conditions (element count, resolution, frame count)

## 8. Operation Rules
- Every threshold change must include reasons in PR description
- Threshold relaxation must include improvement plan and target date
- Baseline updates should use stable main-branch commits
