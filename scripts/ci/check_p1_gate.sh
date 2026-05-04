#!/usr/bin/env bash
set -euo pipefail

METRICS_FILE="${1:-.ci/p1-metrics.txt}"
THRESHOLDS_FILE="${2:-.ci/p1-thresholds.env}"

if [[ ! -f "$METRICS_FILE" ]]; then
  echo "[p1-gate] metrics file not found: $METRICS_FILE"
  exit 1
fi

if [[ ! -f "$THRESHOLDS_FILE" ]]; then
  echo "[p1-gate] thresholds file not found: $THRESHOLDS_FILE"
  exit 1
fi

# shellcheck disable=SC1090
source "$THRESHOLDS_FILE"

required_keys=(
  "DRAW_CALLS_UNBATCHED"
  "DRAW_CALLS_BATCHED"
  "SUBMIT_CALLS_BATCHED"
  "CPU_BUILD_MS_UNBATCHED"
  "CPU_BUILD_MS_BATCHED"
  "DRAW_CALL_REDUCTION_RATIO"
)

for key in "${required_keys[@]}"; do
  if ! grep -q "^${key}=" "$METRICS_FILE"; then
    echo "[p1-gate] missing key in metrics file: $key"
    exit 1
  fi
done

get_metric() {
  local key="$1"
  grep "^${key}=" "$METRICS_FILE" | tail -n 1 | cut -d'=' -f2
}

DRAW_CALLS_UNBATCHED="$(get_metric DRAW_CALLS_UNBATCHED)"
DRAW_CALLS_BATCHED="$(get_metric DRAW_CALLS_BATCHED)"
SUBMIT_CALLS_BATCHED="$(get_metric SUBMIT_CALLS_BATCHED)"
CPU_BUILD_MS_UNBATCHED="$(get_metric CPU_BUILD_MS_UNBATCHED)"
CPU_BUILD_MS_BATCHED="$(get_metric CPU_BUILD_MS_BATCHED)"
DRAW_CALL_REDUCTION_RATIO="$(get_metric DRAW_CALL_REDUCTION_RATIO)"

awk -v dcu="$DRAW_CALLS_UNBATCHED" \
    -v dcb="$DRAW_CALLS_BATCHED" \
    -v scb="$SUBMIT_CALLS_BATCHED" \
    -v cpuu="$CPU_BUILD_MS_UNBATCHED" \
    -v cpub="$CPU_BUILD_MS_BATCHED" \
    -v dratio="$DRAW_CALL_REDUCTION_RATIO" \
    -v dcb_max="$DRAW_CALLS_BATCHED_MAX" \
    -v scb_max="$SUBMIT_CALLS_BATCHED_MAX" \
    -v dratio_max="$DRAW_CALL_REDUCTION_RATIO_MAX" \
    -v cpu_ratio="$CPU_BUILD_IMPROVEMENT_RATIO" '
BEGIN {
  ok = 1

  if (dcb + 0 > dcb_max + 0) {
    printf("[p1-gate] FAIL DRAW_CALLS_BATCHED: %.0f > %.0f\n", dcb + 0, dcb_max + 0)
    ok = 0
  }

  if (scb + 0 > scb_max + 0) {
    printf("[p1-gate] FAIL SUBMIT_CALLS_BATCHED: %.0f > %.0f\n", scb + 0, scb_max + 0)
    ok = 0
  }

  if (dratio + 0 > dratio_max + 0) {
    printf("[p1-gate] FAIL DRAW_CALL_REDUCTION_RATIO: %.3f > %.3f\n", dratio + 0, dratio_max + 0)
    ok = 0
  }

  if (cpub + 0 > (cpuu + 0) * (cpu_ratio + 0)) {
    printf("[p1-gate] FAIL CPU_BUILD_MS_BATCHED: %.3f > %.3f (unbatched %.3f * ratio %.2f)\n", cpub + 0, (cpuu + 0) * (cpu_ratio + 0), cpuu + 0, cpu_ratio + 0)
    ok = 0
  }

  if (dcb + 0 > dcu + 0) {
    printf("[p1-gate] FAIL DRAW_CALLS did not improve: batched %.0f > unbatched %.0f\n", dcb + 0, dcu + 0)
    ok = 0
  }

  if (ok == 0) {
    exit 1
  }

  printf("[p1-gate] PASS draw_calls %.0f -> %.0f ratio=%.3f cpu_build %.3f -> %.3f\n", dcu + 0, dcb + 0, dratio + 0, cpuu + 0, cpub + 0)
}
'