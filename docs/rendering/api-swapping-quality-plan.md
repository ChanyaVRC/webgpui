# API Swapping Quality Assurance Plan (Compat <-> FastPath)

## 1. Purpose
Guarantee through tests that behavior is equivalent when switching from `webgpui-compat` to `webgpui::FastPath`.

## 2. Test Targets
- Path A: Compat (`RenderMode::Compat`)
- Path B: FastPath (`RenderMode::FastPath`)
- Comparison unit: screen scenario, frame, event sequence, internal state

## 3. Test Matrix
| Category | Test Name | Comparison | Pass Criteria |
|---|---|---|---|
| Visual | visual_snapshot_equivalence | Same-frame image output | Pixel diff ratio <= 0.5% |
| Input | event_trace_equivalence | Event order/payload | 100% match |
| State | state_tree_equivalence | Node structure/dirty rect | 100% match |
| API | must_api_contract_equivalence | Return value/side effects of MUST APIs | 100% match |
| Performance | perf_fastpath_advantage | Frame time / draw calls | >= 10% better than Compat |
| Recovery | fallback_consistency | After FastPath -> Compat switch | No functional degradation |

## 4. Scenario Set (Minimum)
1. Basic Shapes
- container + rectangle + opacity
- resize 3 times

2. Interactive Panel
- hover / click / key input
- includes focus movement

3. Dynamic List
- repeated append/remove/update
- frequent dirty-rect cases

4. Stress Batch
- large number of same-style elements
- validate draw-call reduction effect

## 5. Failure Triage Procedure
1. Check state diffs first
- node structure diff
- style diff

2. Check event diffs next
- event order
- stopPropagation / preventDefault behavior

3. Check visual diffs last
- diff heatmap
- impacted region (bounding box)

## 6. CI Operation Rules
- Run both Compat/FastPath paths on every PR
- If either path fails, merge is blocked
- Performance gates are judged on dedicated benchmark runners
- Use latest stable main-branch baseline for comparisons

## 7. Recording Format
Store the following for each test run:

- git sha
- render mode
- scene name
- average / p95 frame time
- draw call count
- snapshot diff ratio
- event diff count

## 8. Completion Criteria
- Equivalence tests exist for all MUST APIs
- Compat/FastPath equivalence holds on all 4 baseline scenarios (§4)
- Zero regressions for two consecutive weeks
- All criteria below must be met before M4 is closed:

| Check | Acceptance |
|---|---|
| MUST-tier API coverage | Equivalence test exists for every MUST row in api-mapping.md §3–§6 |
| Visual equivalence | Pixel diff ratio <= 0.5% on Basic Shapes and Interactive Panel scenarios |
| Event trace equivalence | 100% match on capture/bubble order for Interactive Panel scenario |
| State tree equivalence | 100% match on node structure and dirty-rect state after Dynamic List updates |
| Performance gate | FastPath avg frame time >= 10% better than Compat on Stress Batch scenario |
| Fallback consistency | No functional degradation after FastPath -> Compat switch in Interactive Panel |
| CI gate | `ci/compat-gate` job is green; merge blocked on any failure |

## 9. Milestone Linkage
- M4 entry condition: §8 CI gate must exist and be green before M4 is declared complete.
- M4 parallel P2 track: equivalence tests must also cover dirty-rect skip behavior (state_tree_equivalence for zero-dirty frames).
- M5: any API deprecation identified during M4 must have corresponding equivalence test removal or update in the same PR.
