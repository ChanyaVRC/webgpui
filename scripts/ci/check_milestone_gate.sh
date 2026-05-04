#!/usr/bin/env bash
set -euo pipefail

MILESTONE="${1:-}"

if [[ -z "$MILESTONE" ]]; then
  echo "usage: $0 <M0|M1|M2|M3|M4>"
  exit 2
fi

run_common() {
  cargo fmt --all -- --check
  cargo check --workspace
  cargo test --workspace --all-targets
}

run_m1() {
  cargo test -p webgpui-input --all-targets
  cargo test -p webgpui-platform-winit --all-targets
}

run_m2() {
  cargo test -p webgpui-layout --all-targets
  cargo test -p webgpui-core --all-targets
}

run_m3() {
  cargo test -p webgpui-app --all-targets
  cargo build -p demo-basic
}

run_m4() {
  mkdir -p .ci
  cargo run -p demo-basic -- --benchmark p0 --output .ci/p0-metrics.txt
  cargo run -p demo-basic -- --benchmark p1 --output .ci/p1-metrics.txt
  scripts/ci/check_p0_gate.sh .ci/p0-metrics.txt .ci/p0-thresholds.env
  scripts/ci/check_p1_gate.sh .ci/p1-metrics.txt .ci/p1-thresholds.env
}

echo "[milestone-gate] evaluating $MILESTONE"
run_common

case "$MILESTONE" in
  M0)
    ;;
  M1)
    run_m1
    ;;
  M2)
    run_m1
    run_m2
    ;;
  M3)
    run_m1
    run_m2
    run_m3
    ;;
  M4)
    run_m1
    run_m2
    run_m3
    run_m4
    ;;
  *)
    echo "unknown milestone: $MILESTONE"
    exit 2
    ;;
esac

echo "[milestone-gate] $MILESTONE passed"
