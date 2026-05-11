# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/) — see [docs/semver-policy.md](docs/semver-policy.md).

---

## [Unreleased]

---

## [0.10.0] — M9: Performance Deep Dive (2026-05)

### Added
- **P3 — Staging Vec pre-allocation** in `webgpui-render-wgpu`: `staging_vertices` (4 096),
  `staging_indices` (6 144), `staging_batch_ranges` (64), and both image staging Vecs are
  now initialized with `Vec::with_capacity()` instead of `Vec::new()`.  Steady-state frames
  no longer trigger heap reallocation, satisfying the P3 exit criterion
  "per-frame heap allocation count = 0".
- **`WgpuRenderer::prewarm()`**: submits a no-op command buffer before the event loop starts,
  flushing deferred driver-level pipeline compilation and eliminating first-frame stutter.
- **`AppBuilder::prewarm_pipeline()`**: fluent API that calls `prewarm()` after
  `WgpuRenderer::new()` and before the event loop.
- **`AppBuilder::prewarm_glyph_cache(charset)`**: no-op stub establishing the API surface for
  when a real glyph atlas is introduced.
- **P4 — Render pass auto-skip** in `App::run()`: image upload, side-renderer, and
  `WgpuRenderer::render()` are gated on `dirty_tracker.is_dirty()`.  The animation timeline
  tick already calls `mark_all()` while animations are active, so the guard covers both idle
  and animating cases.  Zero GPU submissions are issued on truly idle frames, satisfying the
  P4 exit criterion "render pass auto-skip verified".
- **`FrameTimer::record_skip()`** in `webgpui-profiler`: records a frame where GPU submission
  was skipped.  Called automatically by `App::run()` on skipped frames.
- **`FrameStats::skip_ratio`**: fraction of frames where GPU submission was skipped
  (`frames_skipped / frames_total`) — the **P4_GPU_SKIP_RATIO** CI gate metric.
- **`FrameTimer::frames_total()` / `frames_skipped()`**: accessors exposing the raw counters.
- **`FrameStats` `Display`** updated to include `skip=X.X%`.
- Seven new profiler tests covering `skip_ratio` edge cases; six new `m9_tests` in
  `webgpui-app` covering P3/P4 preconditions.

---

## [0.9.0] — M8: Developer Tools

### Added
- **`dev-tools` feature flag** in `webgpui-app` and `webgpui-profiler`; zero binary cost and zero
  runtime overhead when the feature is disabled — all overlay code lives inside
  `#[cfg(feature = "dev-tools")]` blocks.
- **Perf overlay** (top-left corner): FPS derived from the rolling-window avg frame time,
  avg and p95 frame times in ms, and user draw-call count for the current frame.
  Rendered via existing `DrawList::fill_rect` primitives — no new GPU resources required.
- **Node inspector overlay** (top-right corner): shows id, kind, role, opacity, visible flag,
  `translate_x/y`, and background color (RGB) for the node selected by
  `DrawContext::dev_inspect(node_id)` each frame.
- **Dirty-rect tint overlay**: translucent red region drawn over `DirtyTracker::effective_area`
  each frame, making redrawn regions visually obvious.
- **Minimal 3×5 bitmap font** in `webgpui-app::dev_tools`: uppercase A–Z, digits 0–9, and common
  symbols rendered as SCALE×SCALE `fill_rect` calls — no font loading, no extra crate dependencies.
- **`DrawContext::dev_inspect(node_id)`** (`dev-tools` feature only): sets which node is reflected
  by the inspector overlay for the current frame.
- `demo-basic`: `dev-tools` feature threaded through; calls `ctx.dev_inspect(anim_fade_node)`
  to demonstrate the inspector alongside the M7 animation demo.
- Seven new tests in `dev_tools_tests` covering all M8 exit criteria: overlay draw-command
  emission, correct reflection of opacity/visible/translate fields, and dirty-rect tint behavior.

---

## [0.8.0] — M7: Animation and Transitions

### Added
- **`Easing`** enum in `webgpui-app`: `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`,
  `CubicBezier(x1, y1, x2, y2)` (CSS `cubic-bezier()` compatible). `Easing::sample(t)`
  evaluates the curve; cubic-bézier uses 16-iteration binary search — sufficient for 60 fps.
- **`Animation`** builder: `Animation::opacity / translate_x / translate_y(node_id, from, to)`
  with fluent `.duration_ms(ms)` and `.easing(Easing)` chaining. Default: 300 ms, linear.
- **`AnimatedProperty`** enum (`Opacity`, `TranslateX`, `TranslateY`).
- **`AnimationTimeline`** (internal to `webgpui-app`): owns the list of running animations;
  each frame `tick()` interpolates values, writes them into `NodeTree` via `set_style`, and
  calls `dirty.mark_all()` while any animation is active. Completed animations are removed.
- **`DrawContext::start_animation(Animation)`**: enqueues a one-shot animation starting on
  the current frame.
- **`DrawContext::set_style(NodeId, NodeStyle)`**: applies a style change and automatically
  creates transition animations for changed animatable properties when
  `NodeStyle::transition` is `Some(TransitionConfig { duration_ms })`.
- `AnimationTimeline::tick` is called before the user frame callback each frame, so
  animated values are already applied when `frame_fn` reads `node.style`.
- **`NodeStyle`** gains two new fields in `webgpui-core`:
  - `translate_x: f32` — X-axis render-time offset (default `0.0`).
  - `translate_y: f32` — Y-axis render-time offset (default `0.0`).
- **`TransitionConfig { duration_ms: f64 }`** and `NodeStyle::transition:
  Option<TransitionConfig>` for implicit style animations.
- `NodeId` re-exported from `webgpui-app` for convenient single-crate imports.
- `apps/demo-basic` extended with a startup animation strip: opacity fade-in (0 → 1,
  900 ms, EaseInOut) and translate-Y slide-in (−32 → 0 px, 600 ms, EaseOut).
- 15 new unit tests: easing endpoint/shape/symmetry/clamp, 5-point opacity keyframe check,
  5-point translate shape check, `AnimationTimeline` dirty/no-dirty/zero-duration/transition.

---

## [0.7.0] — M6: Visual Feature Expansion

### Added
- **Image rendering**: PNG/JPEG loading via the `image` crate; `ImageRegistry` caches decoded
  images by path and avoids re-decoding across frames; `DrawContext::load_image` / `draw_image`
  API; `WgpuRenderer::upload_images` uploads pixel data to GPU textures on demand.
- **SVG rasterization**: `DrawContext::draw_svg` / `load_svg` via `resvg` + `tiny-skia`; results
  are cached by `(path, width, height)` so repeated draws within the same or future frames are
  zero-cost after the first rasterization.
- **Filter effects** (`filters` feature, zero cost when disabled):
  - `PassKind::Filter`, `FilterKind::Blur` / `FilterKind::ColorMatrix` in `webgpui-render-graph`.
  - `BlurParams` (Gaussian blur, configurable pixel radius) and `ColorMatrixParams` (5×4 RGBA
    matrix with built-in `IDENTITY` and `GRAYSCALE` constants).
  - `RenderGraph::add_blur_pass` / `add_color_matrix_pass` helpers.
  - `AppBuilder::enable_filter(FilterKind)` fluent API in `webgpui-app`.
  - WGSL fullscreen-triangle shaders; offscreen intermediate texture used as UI render target
    when filters are active; texture invalidated on resize.
- **CI**: `--features filters` check + test steps; `--features test-gpu,filters` WGSL shader
  compilation tests via lavapipe.

---

## [0.6.0] — M5: API Stabilization

### Added
- `#![warn(missing_docs)]` enabled in `webgpui-core`, `webgpui-compat`, `webgpui-input`, `webgpui-app`; all public items now carry rustdoc comments.
- `docs/semver-policy.md`: formal v0.x semver rules and MUST-tier stability guarantee.
- `CHANGELOG.md` (this file).

### Fixed
- `TabBar::label` panicking silently on out-of-bounds index — `# Panics` doc added; `get_label` non-panicking alternative available.

---

## [0.5.0] — M4: Migration and Replacement Validation

### Added
- `webgpui-compat` crate: full MUST-tier compat layer mapping legacy WebUI string API to typed `webgpui-core` API.
  - `node_create`, `node_append`, `node_remove`, `node_update`
  - `style_background`, `style_size`, `style_position`, `style_padding`, `style_border`, `style_margin`, `style_opacity`, `style_visible`, `style_set`, `style_set_many`
  - `event_on`, `event_stop_propagation`, `focus_set`
  - `app_mount`, `app_unmount`, `render_request`, `render_vsync`, `viewport_resize`
- `apps/demo-migration`: validation app reproducing a representative legacy screen using only the compat API.
- 49 equivalence tests covering all MUST-tier compat functions (`crates/webgpui-compat/src/tests.rs`).
- `docs/m4-metrics.md` and `docs/ja/m4-metrics.md`: API replacement rate (20/20 MUST = 100 %) and screen reproduction rate (8/8 checks = 100 %).
- Multiple performance improvements (O(1) `NodeTree::len`, O(1) FocusManager step, batch-sort skip, layout cache, profiler cache, render-graph O(1) `pass_by_name`).

### Changed
- `NodeTree::add_node` returns `Option<NodeId>` instead of panicking on invalid parent.
- `NodeTree::flush_dirty` returns `Vec<NodeId>` (previously no return value).
- `NodeTree::len` is now O(1) via an incrementally-maintained `live_count`.

### Fixed
- `NodeTree::len` was counting tombstone slots as live nodes.
- `NodeTree::set_role` was not marking the node dirty.
- `DirtyTracker` not reset after backend switch causing stale dirty state.
- `FrameTimer` not reset after backend switch causing stale stats.
- `DrawBatch.key` made non-optional; `None`-key panics in batch sort eliminated.
- `add_dependency` in render-graph now returns `bool` and only marks `topo_dirty` on actual change.
- Layout engine: negative `gap` clamped to zero; `LayoutNode::Default` field duplication resolved.
- Platform-winit: cursor position used consistently for all mouse events.

---

## [0.4.0] — M3: Browser UI Component Layer

### Added
- **Button**: 5 interaction states (normal / hover / pressed / focused / disabled), Enter/Space keyboard activation.
- **TextInput**: cursor positioning, selection range, placeholder text, Backspace/Delete, Shift+Arrow selection highlight.
- **Label**: multiline text rendering, alignment (start / center / end).
- **ScrollView**: overflow clipping, scroll offset tracking.
- **Toolbar**: horizontal strip with gap-based item layout.
- **TabBar + Tab**: selection state, left/right arrow keyboard switching, Home/End wrap.
- **Dialog**: modal backdrop, focus trap (Tab wraps inside dialog), close on Escape.
- **ContextMenu**: position-anchored popup, dismiss on outside-click or Escape.
- `NodeRole` enum (`Button`, `TextBox`, `Tab`, `Dialog`, `Menu`) for accessibility metadata.
- `FOCUS_RING_WIDTH` and `FOCUS_RING_COLOR` constants (2 px blue focus ring).
- `BackendSwitcher` for runtime GPU backend switching (`webgpui-app`).
- Z-order via integer `layer` field on `LayoutNode`; render order sorted by layer.
- `Dialog::set_focusable_count` and ContextMenu keyboard navigation.

### Fixed
- Widget state machine correctness (hover/pressed/focused transitions).
- `TabBar::select_last` panicked on empty tab list — guarded with empty-list check.
- TextInput max-length used byte count instead of character count.
- `DirtyTracker` docs clarified for dual-`None` semantics of `dirty_union`.

---

## [0.3.0] — M2: Text and Layout Foundation

### Added
- `webgpui-layout` crate: `LayoutEngine` with Row/Column direction, `flex_grow`, gap, margin, padding.
- `DefaultTextMeasure`: text metrics stub for shaping-ready text sizing.
- `LayoutResult` with `layer` field populated from `LayoutNode`.
- `sorted_indices` on `LayoutResult` for layer-sorted render order.
- `Direction::sel()` helper to eliminate match boilerplate.
- Workspace architecture docs updated for M2.

### Fixed
- `DefaultTextMeasure` returned non-zero size for `font_size <= 0`.
- `LayoutEngine` child-index validation and absolute-height assertion.
- `LayoutStyle::PartialEq` derived; `set_layout` skips no-op updates.

---

## [0.2.0] — M1: Input and Event Model

### Added
- `webgpui-input` crate: `MouseButton`, `KeyCode`, `InputState`, `KeyState`, `FocusManager`, `Modifiers`.
- Capture → target → bubble event propagation in compat-facing API.
- `FocusManager`: Tab/Shift+Tab traversal, registration, step, set-order.
- Keyboard navigation baseline: focus ring rendering, Tab traversal, Enter/Space activation.
- `webgpui-platform-winit`: key-lookup table replacing 36-arm match.

### Fixed
- `step_focus` sentinel handling; missing focus traversal tests added.
- Platform-winit key char match replaced with efficient lookup table.

---

## [0.1.0] — M0: Stability and CI Baseline

### Added
- Initial workspace with crates: `webgpui-core`, `webgpui-geometry`, `webgpui-render`, `webgpui-render-wgpu`, `webgpui-render-cuda`, `webgpui-render-cpu`, `webgpui-render-graph`, `webgpui-batching`, `webgpui-profiler`, `webgpui-platform`, `webgpui-platform-winit`, `webgpui-app`.
- `NodeTree`: arena-backed node tree with `NodeId`, `NodeStyle`, `NodeKind`.
- `DirtyTracker`: dirty-rect accumulation for incremental rendering.
- `webgpui-render-wgpu`: wgpu-based GPU renderer.
- `webgpui-render-cuda`: CUDA renderer scaffold.
- `webgpui-render-cpu`: CPU software renderer.
- `webgpui-render-graph`: topological render pass graph.
- `webgpui-batching`: draw-call batcher with `DrawBatch` keying.
- `webgpui-profiler`: `FrameTimer` with P0/P1 gate metrics.
- `apps/demo-basic`: reference application with headless benchmark mode.
- CI: workspace test workflow, rustfmt, P0/P1 gate checks.

### Fixed
- Initial rustfmt pass across the workspace.
- Headless benchmark mode added to `demo-basic` for CI gate compatibility.
