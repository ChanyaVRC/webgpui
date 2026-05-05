# Rendering Performance Execution Plan

## 1. Goal
Improve perceived speed with rendering-focused optimization and stabilize frame times.

- Average frame time: <= 16.6ms
- P95 frame time: <= 20ms
- Draw calls: <= 200 (major screens)
- Unchanged frames: redraw can be skipped

## 1.1 Implementation Priority (Performance-First)
Fix the order below; do not start lower priority items before upper ones are complete.

1. P0: Measurement and rendering hot path
- Implement CPU/GPU measurement first (no optimization without measurement)
- Implement `begin_frame_fast` / `submit_batch` / `end_frame_fast` first
- Acceptance: bottleneck ranges are numerically visible and FastPath runs

2. P1: Draw-call reduction
- Implement batching and instancing
- Implement sort strategy to minimize pipeline switches
- Acceptance: draw calls <= 200 on representative screens

3. P2: Redraw reduction
- Integrate `mark_dirty_rect` / `commit_dirty`
- Enable render skip on unchanged frames
- Acceptance: GPU time continuously decreases on no-update frames

4. P3: Transfer and cache optimization
- Ring-buffer and transient-buffer reuse
- Introduce `prewarm_pipeline` / `prewarm_glyph_cache`
- Acceptance: reduced startup stuttering

5. P4: Parallelization and structural optimization
- Optimize render graph
- Separate UI updates from render preparation
- Acceptance: further p95 stabilization

## 1.2 Deferred Items (Due to Performance Priority)
- Advanced layout features (full Flex/Grid compatibility)
- Visual feature expansion (filter/transition/SVG)
- Developer UX enhancements (advanced inspector)

## 2. Top-Priority Tasks (Phase 1: Measurement)
1. Add CPU measurement points
- update
- layout
- build draw list
- encode + submit

2. Add GPU measurement
- clear pass
- ui pass
- overlay pass

3. Aggregate results per second
- average
- p95
- max

## 3. Optimization Tasks (Phase 2: High-Impact)
1. Batching
- Sort and aggregate by pipeline / texture / blend state
- Shift small draw calls toward instancing

2. Transfer reduction
- Use ring buffers for vertex data
- Keep static geometry resident on GPU

3. Redraw suppression
- Introduce dirty rect
- Omit render pass when there is no change

## 4. Mid-Term Tasks (Phase 3: Structural Optimization)
1. Introduce render graph
- Explicit pass dependencies
- Auto-skip unnecessary passes

2. Data-structure optimization
- SoA for hot data
- Allocation reduction via pre-reserve

3. Parallelization
- Separate UI update and render prep
- Prepare commands on worker threads

## 5. Validation Method
1. Baseline measurement
- Minimal sample
- Medium-complexity UI sample

2. Per-optimization differential measurement
- Enable one optimization at a time
- Store before/after

3. Regression monitoring
- Warn on significant degradation
- Continuously record key metrics

## 6. Completion Criteria

### P0 Completion
- CPU measurement points (update / layout / draw-list / encode+submit) emit values every frame.
- GPU measurement points (clear / ui / overlay pass) emit values every frame.
- `begin_frame_fast` / `submit_batch` / `end_frame_fast` run without panic on representative scenes.
- CI P0 gate is green (see ci-gates.md).

### P1 Completion
- Draw calls <= 200 on representative screens.
- Pipeline-sort batching verified: draw call count reduces by >= 30% vs unbatched baseline.
- CI P1 gate is green.

### P2 Completion (M4 Parallel Track)
- `mark_dirty_rect` / `commit_dirty` integrated into render pipeline (`webgpui-render`).
- Render pass skipped on frames with no dirty regions; verified by GPU query (submission count = 0).
- `P2_GPU_SKIP_RATIO` metric added to `.ci/` metrics format and reported per frame.
- GPU time on static (no-update) scenes shows measurable and continuous decrease.
- CI P2 gate added and green.

### P3 Completion
- Per-frame heap allocation count = 0 on steady-state frames (measured via `dhat` or custom hook).
- Startup stutter eliminated: no single frame > 50ms at launch.
- `prewarm_pipeline` and `prewarm_glyph_cache` APIs available and used in `demo-basic`.

### P4 Completion
- Render graph auto-skips passes with no dirty inputs; zero GPU submissions on fully-static frames.
- UI update and render command encoding are on separate threads; no measurable increase in frame-time variance.
- p95 frame time <= 20ms on a scene with >= 500 nodes.

### Overall Completion
- p95 is within target (<= 20ms) on all representative screens.
- Frame drops during interaction are visibly reduced (no visible stutter in manual testing).
- All bottlenecks are explainable by measurement logs (no unexplained spikes).

## 7. FastPath-Oriented Optimization Policy
Add performance-focused native APIs separate from compatibility APIs.

1. Low-overhead draw APIs
- `begin_frame_fast(frame_ctx)`
- `submit_batch(batch_key, instances)`
- `end_frame_fast()`

2. Differential-update APIs
- `mark_dirty_rect(rect)` (supersedes `mark_dirty_rect(node_id, rect)` prototype; see §17)
- `commit_dirty()`

3. Buffer-management APIs
- `allocate_transient_buffer(bytes)`
- `write_transient(slice)`
- `recycle_transient(frame_id)`

4. Cache-control APIs
- `prewarm_pipeline(pipeline_desc)`
- `prewarm_glyph_cache(font, charset)`
- `evict_cache(policy)`

## 8. Compatibility API vs Native API Usage
1. Compatibility API (`webgpui-compat`)
- Purpose: minimize migration cost from legacy engines
- Feature: generality first, includes conversion overhead

2. Native API (`webgpui` FastPath)
- Purpose: maximize performance
- Feature: lower-level control and higher caller responsibility

3. Recommended operation
- Start migration with compatibility APIs
- Replace bottleneck-only sections with native APIs in stages

## 9. Introduction Steps
1. Phase A: Add measurement
- Obtain baseline through compatibility path

2. Phase B: Minimal native API introduction
- Implement `begin_frame_fast` / `submit_batch` / `end_frame_fast`
- Verify draw-call and CPU-time differences

3. Phase C: Differential update introduction
- Connect `mark_dirty_rect` with UI diffs
- Enable render skip on unchanged frames

4. Phase D: Cache optimization
- Prewarm pipeline/glyph at startup
- Verify reduced stuttering through measurement

## 10. Acceptance Criteria (Native API)
- Improve average frame time by at least 15% on screens where native APIs are applied
- Maintain visual correctness equivalent to compatibility path
- No feature degradation when fallback to compatibility path

## 11. Test-Based Quality Assurance (API Swap Equivalence)
Guarantee unchanged behavior after switching to native APIs via automated tests.

1. Snapshot equivalence test (visual)
- Run same scenario on `compat` and `fastpath`
- Compare image output at the same frame number with tolerance
- Pass criterion: pixel-difference ratio <= 0.5%, key UI regions <= 0.1%

2. Event trace equivalence test (input)
- Replay fixed click/move/key sequences
- Compare event order and payloads
- Pass criterion: 100% order match, payload difference 0

3. State transition equivalence test (UI tree)
- Validate internal state at mount -> update -> unmount
- Compare node count, parent-child relations, dirty rects
- Pass criterion: 100% structure match

4. Property test (random updates)
- Generate random style/update sequences
- Validate final states of `compat` and `fastpath` match
- Pass criterion: 0 failures (minimum 10,000 sequences)

## 12. Regression Prevention Tests (CI)
1. Required PR jobs
- `equivalence-visual`
- `equivalence-events`
- `equivalence-state`
- `perf-regression`

2. Performance regression gates
- Fail if `fastpath` becomes >10% slower than `compat`
- Fail if p95 frame time worsens by >5% vs baseline

3. Compatibility regression gates
- Block merge if any MUST-API equivalence test fails

## 13. Implementation Rules (Testability)
- Expose path switch via `RenderMode::Compat` / `RenderMode::FastPath`
- Allow fixed random seeds for reproducibility
- Abstract time dependencies behind `Clock` for deterministic tests
- Provide deterministic rendering mode for snapshot comparison

## 14. Reference
- API swap quality details: `api-swapping-quality-plan.md`

## 15. P0 Gate to Introduce in CI First (Minimum Criteria)
P0 completion is first judged mechanically by automated CI gate.

1. Metrics (FastPath standalone)
- `AVG_FRAME_MS <= 16.6`
- `P95_FRAME_MS <= 20.0`
- `DRAW_CALLS <= 200`

2. Metrics (Compared to Compat)
- `FASTPATH_AVG_FRAME_MS <= COMPAT_AVG_FRAME_MS * 0.90`
- `FASTPATH_P95_FRAME_MS <= COMPAT_P95_FRAME_MS * 0.95`

3. Metrics file format
- Output path: `.ci/p0-metrics.txt`
- Format: `KEY=VALUE` (one item per line)
- Required keys:
	- `AVG_FRAME_MS`
	- `P95_FRAME_MS`
	- `DRAW_CALLS`
	- `COMPAT_AVG_FRAME_MS`
	- `COMPAT_P95_FRAME_MS`
	- `FASTPATH_AVG_FRAME_MS`
	- `FASTPATH_P95_FRAME_MS`

4. CI failure conditions
- Missing required key
- Non-numeric value
- Any threshold violation

5. Related files
- Workflow: `.github/workflows/p0-gate.yml`
- Check script: `scripts/ci/check_p0_gate.sh`
- Thresholds: `.ci/p0-thresholds.env`
- Operations guide: `docs/ci-gates.md`
- Metrics spec: `docs/metrics-format.md`

## 16. P1 Gate to Introduce After P0 Completion (Batching Effect)
P1 completion is mechanically judged by CI based on improvement before/after batching.

1. Metrics (after FastPath + batching)
- `DRAW_CALLS_BATCHED <= 120`
- `SUBMIT_CALLS_BATCHED <= 4`

2. Metrics (compared to pre-batching)
- `DRAW_CALL_REDUCTION_RATIO <= 0.60`
- `CPU_BUILD_MS_BATCHED <= CPU_BUILD_MS_UNBATCHED * 0.80`

3. Metrics file format
- Output path: `.ci/p1-metrics.txt`
- Format: `KEY=VALUE` (one item per line)
- Required keys:
	- `DRAW_CALLS_UNBATCHED`
	- `DRAW_CALLS_BATCHED`
	- `SUBMIT_CALLS_BATCHED`
	- `CPU_BUILD_MS_UNBATCHED`
	- `CPU_BUILD_MS_BATCHED`
	- `DRAW_CALL_REDUCTION_RATIO`

4. CI failure conditions
- Missing required key
- Non-numeric value
- Any threshold violation

5. Related files
- Workflow: `.github/workflows/p1-gate.yml`
- Check script: `scripts/ci/check_p1_gate.sh`
- Thresholds: `.ci/p1-thresholds.env`
- Operations guide: `docs/ci-gates.md`
- Metrics spec: `docs/metrics-format.md`

## 17. P2 API Specification — Dirty Rect Optimization

### 17.1 Design Goals
- Two public entry points: `mark_dirty_rect(rect)` and `commit_dirty(viewport) -> DirtyDecision`.
- `DirtyTracker` (already in `webgpui-core`) handles per-frame accumulation.
- `DirtyDecision` (new enum in `webgpui-core`) encodes the render decision for the current frame.
- `RenderGraph` consumes `DirtyDecision` to skip passes or apply a scissor rect.
- The `App::run` frame loop bridges accumulation to the renderer.
- No changes to the public `DrawContext` draw-command API; optimization is fully internal.

### 17.2 New Types

#### `DirtyDecision` (in `webgpui-core`)
```rust
pub enum DirtyDecision {
    /// No dirty regions this frame. Skip all render passes entirely.
    Skip,
    /// Only this screen region changed. Re-render within the scissor rect.
    Partial(Rect),
    /// Full redraw required (resize, first frame, or `mark_all` called).
    Full,
}
```

#### `RenderOutcome` (in `webgpui-render`)
```rust
pub enum RenderOutcome {
    /// GPU command buffer was submitted.
    Submitted,
    /// No GPU work performed; frame was skipped.
    Skipped,
}
```

### 17.3 API Changes per Crate

**`webgpui-core` — `DirtyTracker`**

Existing methods are unchanged. Add:
```rust
/// Ergonomic alias for `mark()`; preferred in frame callbacks and compat layer.
pub fn mark_dirty_rect(&mut self, rect: Rect);

/// Consumes accumulated dirty state and returns the render decision for this frame.
/// Clears internal state. Must be called exactly once per frame, before rendering.
pub fn commit_dirty(&mut self, viewport: Size) -> DirtyDecision;
```
`commit_dirty` logic:
- No rects recorded and `full_invalidate == false` → `Skip`
- `full_invalidate == true` → `Full`, then clear
- Rects recorded → `Partial(dirty_union())`, then clear

**`webgpui-render` — `Renderer` trait**

Add alongside the existing `render()` method:
```rust
/// Re-render only within `area` using a GPU scissor rect.
/// Falls back to full render if the backend does not support scissor.
fn render_partial(&mut self, draw_list: &DrawList, area: Rect) -> RenderResult<RenderOutcome>;
```
The existing `render()` continues to mean a full redraw and returns `RenderOutcome::Submitted`.

**`webgpui-render-graph` — `RenderGraph`**

Add:
```rust
/// Applies the dirty decision to pass configuration before execution.
/// - `Skip`: disables all passes.
/// - `Partial(rect)`: sets a scissor rect on the `ui` pass; clears pass is
///   always full (background must not be clipped).
/// - `Full`: no change (all passes run as configured).
pub fn apply_dirty_decision(&mut self, decision: &DirtyDecision);
```

**`webgpui-app` — Frame loop**

The three currently unused variables become active:
```rust
// Before: let _dirty_tracker = DirtyTracker::new();
// After:  let mut dirty_tracker = DirtyTracker::new();
```
Frame loop pseudocode:
```
on RedrawRequested:
  let viewport = Size::new(sw as f32, sh as f32);
  let decision = dirty_tracker.commit_dirty(viewport);

  if matches!(decision, DirtyDecision::Skip) {
      frame_timer.record_skip();
      p2_skip_counter += 1;
      // No GPU work.
      return;
  }

  draw_list.clear();
  frame_fn(&mut ctx);
  renderer.render_graph_mut().apply_dirty_decision(&decision);

  match decision {
      DirtyDecision::Partial(area) => renderer.render_partial(&draw_list, area),
      _                            => renderer.render(&draw_list),
  };
  p2_total_counter += 1;
```

**`webgpui-app` — `DrawContext`**

Add for use inside frame callbacks:
```rust
/// Marks a screen region as needing redraw this frame.
/// Has no effect when called outside an active frame.
pub fn mark_dirty_rect(&mut self, rect: Rect);
```

### 17.4 Integration with `NodeTree`

When `NodeTree::flush_dirty()` returns dirty `NodeId`s, the frame loop:
1. Looks up each node's computed layout rect (output of the layout pass).
2. Calls `dirty_tracker.mark_dirty_rect(layout_rect)` for each.

This keeps `DirtyTracker` in sync with node-tree change detection.  
Note: this connection is the prerequisite addressed in issue #38.

### 17.5 Metric: `P2_GPU_SKIP_RATIO`

| Key | Definition | CI threshold |
|---|---|---|
| `P2_GPU_SKIP_RATIO` | `skipped_frames / total_frames` (rolling 60-frame window) | >= 0.50 on fully-static scene |
| `P2_GPU_SUBMISSIONS` | raw GPU submission count per 60-frame window | = 0 on fully-static scene |

Output path: `.ci/p2-metrics.txt` (same `KEY=VALUE` format as P0/P1).

### 17.6 Freeze Rule
- `DirtyDecision` variants are frozen at v0.1. Adding a variant is a breaking change.
- `mark_dirty_rect` and `commit_dirty` signatures are frozen at v0.1.
- `RenderOutcome` variants are frozen at v0.1.
- Scissor-rect support in `render_partial` is backend-specific; backends that do not
  support it must fall back to full render and document the limitation.
