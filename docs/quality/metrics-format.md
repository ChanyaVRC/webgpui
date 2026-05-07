# Metrics Format Specification (P0 / P1)

## 1. Common Rules
- Format: `KEY=VALUE`
- One item per line
- Encoding: UTF-8
- Decimal separator: `.`

## 2. P0 Metrics
File: `.ci/p0-metrics.txt`

Required keys:
- `AVG_FRAME_MS`
- `P95_FRAME_MS`
- `DRAW_CALLS`
- `COMPAT_AVG_FRAME_MS`
- `COMPAT_P95_FRAME_MS`
- `FASTPATH_AVG_FRAME_MS`
- `FASTPATH_P95_FRAME_MS`

Example:
```text
AVG_FRAME_MS=14.8
P95_FRAME_MS=18.9
DRAW_CALLS=160
COMPAT_AVG_FRAME_MS=18.0
COMPAT_P95_FRAME_MS=22.0
FASTPATH_AVG_FRAME_MS=14.8
FASTPATH_P95_FRAME_MS=18.9
```

## 3. P1 Metrics
File: `.ci/p1-metrics.txt`

Required keys:
- `DRAW_CALLS_UNBATCHED`
- `DRAW_CALLS_BATCHED`
- `SUBMIT_CALLS_BATCHED`
- `CPU_BUILD_MS_UNBATCHED`
- `CPU_BUILD_MS_BATCHED`
- `DRAW_CALL_REDUCTION_RATIO`

Example:
```text
DRAW_CALLS_UNBATCHED=320
DRAW_CALLS_BATCHED=96
SUBMIT_CALLS_BATCHED=3
CPU_BUILD_MS_UNBATCHED=6.5
CPU_BUILD_MS_BATCHED=4.9
DRAW_CALL_REDUCTION_RATIO=0.30
```

## 4. Value Definitions
- `DRAW_CALL_REDUCTION_RATIO = DRAW_CALLS_BATCHED / DRAW_CALLS_UNBATCHED`
- `CPU_BUILD_MS_*` means CPU measurements for the build-draw-list section
- `P95_FRAME_MS` is the 95th percentile over measured frames

## 5. Notes
- Missing required keys are treated as gate failures
- Non-numeric values are treated as gate failures
- If duplicate keys exist, the check script uses the last occurrence
