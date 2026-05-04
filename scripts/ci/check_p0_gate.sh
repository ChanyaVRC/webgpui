#!/usr/bin/env bash
set -euo pipefail

METRICS_FILE="${1:-.ci/p0-metrics.txt}"
THRESHOLDS_FILE="${2:-.ci/p0-thresholds.env}"

if [[ ! -f "$METRICS_FILE" ]]; then
  echo "[p0-gate] metrics file not found: $METRICS_FILE"
  exit 1
fi

if [[ ! -f "$THRESHOLDS_FILE" ]]; then
  echo "[p0-gate] thresholds file not found: $THRESHOLDS_FILE"
  exit 1
fi

# shellcheck disable=SC1090
source "$THRESHOLDS_FILE"

required_keys=(
  "AVG_FRAME_MS"
  "P95_FRAME_MS"
  "DRAW_CALLS"
  "COMPAT_AVG_FRAME_MS"
  "COMPAT_P95_FRAME_MS"
  "FASTPATH_AVG_FRAME_MS"
  "FASTPATH_P95_FRAME_MS"
)

for key in "${required_keys[@]}"; do
  if ! grep -q "^${key}=" "$METRICS_FILE"; then
    echo "[p0-gate] missing key in metrics file: $key"
    exit 1
  fi
done

get_metric() {
  local key="$1"
  grep "^${key}=" "$METRICS_FILE" | tail -n 1 | cut -d'=' -f2
}

AVG_FRAME_MS="$(get_metric AVG_FRAME_MS)"
P95_FRAME_MS="$(get_metric P95_FRAME_MS)"
DRAW_CALLS="$(get_metric DRAW_CALLS)"
COMPAT_AVG_FRAME_MS="$(get_metric COMPAT_AVG_FRAME_MS)"
COMPAT_P95_FRAME_MS="$(get_metric COMPAT_P95_FRAME_MS)"
FASTPATH_AVG_FRAME_MS="$(get_metric FASTPATH_AVG_FRAME_MS)"
FASTPATH_P95_FRAME_MS="$(get_metric FASTPATH_P95_FRAME_MS)"

awk -v avg="$AVG_FRAME_MS" \
    -v p95="$P95_FRAME_MS" \
    -v dc="$DRAW_CALLS" \
    -v cavg="$COMPAT_AVG_FRAME_MS" \
    -v cp95="$COMPAT_P95_FRAME_MS" \
    -v favg="$FASTPATH_AVG_FRAME_MS" \
    -v fp95="$FASTPATH_P95_FRAME_MS" \
    -v avg_max="$AVG_FRAME_MS_MAX" \
    -v p95_max="$P95_FRAME_MS_MAX" \
    -v dc_max="$DRAW_CALLS_MAX" \
    -v avg_ratio="$FASTPATH_AVG_IMPROVEMENT_RATIO" \
    -v p95_ratio="$FASTPATH_P95_IMPROVEMENT_RATIO" '
BEGIN {
  ok = 1

  if (avg + 0 > avg_max + 0) {
    printf("[p0-gate] FAIL AVG_FRAME_MS: %.3f > %.3f\n", avg + 0, avg_max + 0)
    ok = 0
  }
  if (p95 + 0 > p95_max + 0) {
    printf("[p0-gate] FAIL P95_FRAME_MS: %.3f > %.3f\n", p95 + 0, p95_max + 0)
    ok = 0
  }
  if (dc + 0 > dc_max + 0) {
    printf("[p0-gate] FAIL DRAW_CALLS: %.0f > %.0f\n", dc + 0, dc_max + 0)
    ok = 0
  }

  if (favg + 0 > (cavg + 0) * (avg_ratio + 0)) {
    printf("[p0-gate] FAIL FASTPATH_AVG_FRAME_MS: %.3f > %.3f (compat %.3f * ratio %.2f)\n", favg + 0, (cavg + 0) * (avg_ratio + 0), cavg + 0, avg_ratio + 0)
    ok = 0
  }
  if (fp95 + 0 > (cp95 + 0) * (p95_ratio + 0)) {
    printf("[p0-gate] FAIL FASTPATH_P95_FRAME_MS: %.3f > %.3f (compat %.3f * ratio %.2f)\n", fp95 + 0, (cp95 + 0) * (p95_ratio + 0), cp95 + 0, p95_ratio + 0)
    ok = 0
  }

  if (ok == 0) {
    exit 1
  }

  printf("[p0-gate] PASS avg=%.3f p95=%.3f draw_calls=%.0f\n", avg + 0, p95 + 0, dc + 0)
}
'
