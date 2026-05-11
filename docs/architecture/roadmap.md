# WebUI Roadmap

## 1. Objective
Close the gap from "GPU UI core" to a production-ready WebUI stack.

## 2. Current Status (May 2026)
- Done: window/event loop, wgpu rendering path, primitive drawing, basic input capture, minimal node tree, dirty tracking baseline, profiling baseline.
- Missing: web deployment path, mature text stack, event propagation model, advanced layout, accessibility, standard component layer, migration completion metrics.

## 3. Milestones

### M0: Stability and CI Baseline — ✓ Completed
Scope:
- Keep CI green for fmt/test/gates.
- Remove warning regressions.
- Define ownership for crates and docs.
Exit Criteria:
- 7 consecutive days with no red main branch CI.
- Zero rustfmt failures in PR checks.

### M1: Input and Event Model — ✓ Completed
Scope:
- Complete pointer semantics (move/down/up/scroll consistency).
- Add capture/bubble event propagation in compat-facing API.
- Normalize focus behavior and keyboard navigation baseline.
Exit Criteria:
- Event-order tests for capture -> target -> bubble.
- Focus traversal tests for Tab/Shift+Tab.

### M2: Text and Layout Foundation — ✓ Completed
Scope:
- Introduce real text pipeline (font loading, shaping-ready interfaces).
- Implement baseline text metrics and wrapping.
- Expand layout toward Flex-like behavior for key screens.
Exit Criteria:
- Text rendering for mixed strings at stable positions.
- Reproducible layout results for predefined fixtures.

### M3: Browser UI Component Layer — ✓ Completed

The minimum set of widgets needed to build a functional browser UI.
Each sub-milestone ships independently and keeps CI green.

#### M3-A: Core Interactive Widgets (3-4 weeks)
Scope:
- **Button**: 5 interaction states (normal / hover / pressed / focused / disabled), Enter/Space activation.
- **TextInput**: cursor positioning, selection range, placeholder text, Backspace/Delete handling.
- **Label**: multiline text rendering, alignment (start / center / end).
Exit Criteria:
- Unit tests cover each widget state transition.
- `demo-basic` extended with a text form (Label + TextInput + Button).

#### M3-B: Structural Widgets (2-3 weeks)
Scope:
- **ScrollView**: overflow clipping, scroll offset tracking, optional scrollbar rendering.
- **Toolbar**: horizontal strip with gap-based item layout.
- **TabBar + Tab**: selection state, keyboard switching with arrow keys, Home/End.
Exit Criteria:
- ScrollView overflow-clipping fixture tests pass.
- TabBar keyboard traversal tests (left/right arrow, Home/End wrap).

#### M3-C: Overlay and Z-Order (2-3 weeks)
Scope:
- **Z-order system**: integer `layer` field on `LayoutNode`; render order sorted by layer.
- **Dialog**: modal backdrop, focus trap (Tab wraps inside dialog only), close on Escape.
- **ContextMenu**: position-anchored popup, dismiss on outside-click or Escape.
Exit Criteria:
- Dialog focus-trap test: Tab from last focusable item wraps to first.
- ContextMenu dismiss test: outside-click event closes menu.

#### M3-D: Accessibility and Polish (2-3 weeks)
Scope:
- **Role metadata**: ARIA-equivalent `role` field (`button` / `textbox` / `tab` / `dialog` / `menu`).
- **Focus ring standardization**: consistent 2 px inset ring across all M3 widgets.
- **Keyboard audit**: every M3 widget fully operable without mouse.
Exit Criteria:
- `role` field present in node data structures and accessible at app layer.
- Keyboard-only navigation of a "form demo" (Label + TextInput + Button + Dialog).
- `cargo test -p webgpui-app --all-targets` passes.

Crates affected: `webgpui-core` (widget state machines, NodeRole), `webgpui-app` (DrawContext widget helpers), `webgpui-batching` (batch generation for widget geometry), `apps/demo-basic`.

### M4: Migration and Replacement Validation — ✓ Completed
Prerequisites:
- `webgpui-compat` crate must exist with all MUST-tier APIs implemented (see api-mapping.md §13).
- `apps/demo-migration` app must be created as the validation target.
Scope:
- Finish API mapping coverage and MUST compatibility checks.
- Reproduce representative legacy screens in `apps/demo-migration`.
- Quantify migration cost (lines changed, unsupported API count) and performance delta vs legacy.
Exit Criteria:
- API replacement ratio >= 80%.
- Screen reproduction ratio >= 90%.
- Performance target met per requirements summary.
- Compat/FastPath equivalence tests pass for all MUST-tier APIs (see api-swapping-quality-plan.md §8).
Crates affected: `webgpui-compat` (new), `webgpui-app`, `webgpui-core`, `webgpui-input`, `apps/demo-migration` (new).
Risks:
- Text rendering position differs between compat and legacy due to different shaping backends. Mitigation: allow ±2px tolerance in visual snapshots; document known differences.
- Event timing differences (capture/bubble order). Mitigation: event-trace tests lock exact order; compat layer absorbs differences explicitly.
- `webgpui-compat` API scope creep. Mitigation: enforce §13.4 freeze; additions require explicit PR.

> **Parallel track during M4 — Performance P2 (dirty rect):**
> - Integrate `mark_dirty_rect` / `commit_dirty` into the render pipeline (`webgpui-render`).
> - Enable render pass skip when no dirty regions exist.
> - Add `P2_GPU_SKIP_RATIO` metric to `.ci/` metrics format.
> - Acceptance: GPU time continuously decreases on no-update frames; verified via CI P2 gate.
> - This track runs alongside M4 and does not block M4 exit criteria.
> - Responsible PR: `perf/p2-dirty-rect-integration`
> - Crates affected: `webgpui-core` (DirtyTracker), `webgpui-render` (skip logic), `webgpui-render-wgpu` (scissor), `webgpui-batching` (batch culling), `webgpui-app` (mark_dirty_rect API).

### M5: API Stabilization — ✓ Completed (2026-05)
Scope:
- Finalize and document all public-facing API surfaces across `webgpui-app`, `webgpui-core`, `webgpui-input`, `webgpui-compat`.
- Declare semver policy (v0.x): patch = bug fix only, minor = additive, major = any breaking change in MUST-tier type or function.
- Publish `docs/semver-policy.md` and link from `docs/contributing.md`.
- Add `#[deprecated]` annotations for any APIs identified as migration candidates during M4.
Exit Criteria:
- All MUST-tier public APIs have rustdoc with at least one `# Example` block.
- `docs/semver-policy.md` exists and is linked from contributing guide.
- `CHANGELOG.md` created with M0–M5 entries.
- Zero `#[allow(missing_docs)]` suppressions on public items in affected crates.
Crates affected: `webgpui-app`, `webgpui-core`, `webgpui-compat`, `webgpui-input`.

### M6: Visual Feature Expansion — ✓ Completed (2026-05)
Scope:
- Image rendering: PNG/JPEG loading via `image` crate; GPU texture upload; `DrawContext::load_image` / `draw_image` API; path-keyed `ImageRegistry` cache.
- SVG rasterization: via `resvg`/`tiny-skia`; `DrawContext::draw_svg` / `load_svg`; cached by `(path, width, height)`.
- Filter effects: Gaussian blur and 5×4 color matrix as post-process passes in `webgpui-render-graph`; gated behind `feature = "filters"`; zero binary cost when disabled.
Exit Criteria:
- Image nodes render PNG/JPEG correctly. ✓
- Simple SVG icons (flat paths) render without visual regression. ✓
- Filter pass excluded from binary when `filters` feature is disabled. ✓
Crates affected: `webgpui-render-wgpu` (GPU upload, filter shaders), `webgpui-render-graph` (filter pass), `webgpui-app` (image/SVG/filter API).

### M7: Animation and Transitions — ✓ Completed (2026-05)
Scope:
- `webgpui-app` exposes an `Animation` builder: `Animation::opacity / translate_x / translate_y(node_id, from, to)` with `.duration_ms()` / `.easing()` chaining.
- `Easing` enum: `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`, `CubicBezier(x1, y1, x2, y2)` — `Easing::sample(t)` via cubic polynomial; cubic-bézier via 16-iter binary search.
- `AnimationTimeline` (internal): advances active animations each frame before the user callback; writes interpolated values to `NodeTree`; calls `dirty.mark_all()` while any animation is active.
- Style transitions: implicit animations created by `DrawContext::set_style` when `NodeStyle::transition` is `Some(TransitionConfig { duration_ms })`.
- `NodeStyle` gains `translate_x: f32`, `translate_y: f32` and `transition: Option<TransitionConfig>`.
- Elapsed-time-based interpolation (not frame-count-based); no external animation crates.
Exit Criteria:
- `opacity` fade passes 5-point linear keyframe check (`opacity_fade_keyframes_linear`). ✓
- `translate_y` slide passes 5-point ease-out shape check (`translate_slide_keyframes_ease_out`). ✓
- No frame-time regression on zero-animation scenes: `tick()` returns immediately and does not call `mark_all()` when no animations are active. ✓
- Animation tick always marks dirty while active (`animation_tick_marks_dirty_when_active`). ✓
Crates affected: `webgpui-app` (animation API, timeline), `webgpui-core` (`NodeStyle`, `TransitionConfig`).

### M8: Developer Tools — ✓ Completed (2026-05)
Scope:
- **`dev-tools` feature flag** in `webgpui-app` and `webgpui-profiler`; zero binary cost when disabled.
- **Perf overlay**: FPS, avg/p95 frame time, draw-call count rendered in the top-left corner via
  existing `DrawList::fill_rect` calls — no new GPU resources.
- **Node inspector overlay**: id, kind, role, opacity, visible, translate x/y, background color for
  the node passed to `DrawContext::dev_inspect(node_id)` each frame.  Rendered top-right.
- **Dirty-rect tint**: translucent colored region over `DirtyTracker::effective_area` each frame.
- Minimal 3×5 bitmap font in `webgpui-app::dev_tools` covering A–Z, 0–9, and common symbols.
Exit Criteria:
- ✓ Perf overlay renders correctly; no impact on `--release` builds without `dev-tools`.
- ✓ Inspector overlay reflects correct computed style for all MUST-tier style properties.
- ✓ Binary size delta with `dev-tools` disabled: zero (entire module excluded by `#[cfg]`).
Crates affected: `webgpui-app` (inspector API, overlay module), `webgpui-profiler` (feature flag).

### M9: Performance Deep Dive — ✓ Completed (2026-05)
Scope:
- **P3 — Transfer and cache optimization:**
  - Ring-buffer for vertex/index uploads in `webgpui-render-wgpu`; eliminate per-frame `create_buffer` calls.
  - Transient buffer pool: recycle short-lived buffers via a fixed-size pool.
  - `prewarm_pipeline(desc)`: compile and cache wgpu pipeline descriptors at startup.
  - `prewarm_glyph_cache(font, charset)`: pre-rasterize a defined character set before first frame.
- **P4 — Render graph and parallelization:**
  - `webgpui-render-graph`: explicit pass dependency declarations; auto-skip passes with no dirty inputs.
  - Separate UI tree update (main thread) from render command encoding (worker thread via `rayon` or `std::thread`).
  - SoA (Struct of Arrays) layout for hot node data in `webgpui-core`.
Exit Criteria:
- No single frame > 50ms at launch (startup stutter eliminated). ✓
- Per-frame heap allocation count = 0 on steady-state frames (measured via `dhat` or custom hook). ✓
- p95 frame time <= 20ms on a scene with >= 500 nodes. ✓
- Render pass auto-skip verified: zero GPU submissions on frames with no dirty regions. ✓
Crates affected: `webgpui-render-wgpu` (ring buffer, transient pool, prewarm), `webgpui-render-graph` (dependency graph, auto-skip), `webgpui-core` (SoA), `webgpui-app` (prewarm API), `webgpui-profiler` (skip metric).
Risks:
- Worker thread render encoding: wgpu `Surface` is not `Send` on all platforms. Mitigation: encode `CommandBuffer` (which is `Send`) on worker; submit on main thread.
- SoA refactor is a large structural change. Mitigation: dedicated PR with comprehensive snapshot + perf before/after.

### M10: Web / WASM Deployment (4-8 weeks)
Scope:
- All crates except `webgpui-platform-winit` must compile for `wasm32-unknown-unknown`.
- Platform abstraction: `webgpui-platform` defines a `PlatformBackend` trait; `webgpui-platform-winit` implements it for native; new `webgpui-platform-web` implements it for browsers using `web-sys`.
- wgpu backend: `WebGPU` feature on supporting browsers; fall back to `WebGL2` via wgpu's `webgl` feature.
- Event bridge: `web-sys` DOM events (mouse, keyboard, resize, pointer) mapped to `webgpui-input` event types.
- `apps/demo-web`: `trunk`-compatible binary crate running `demo-basic` scenes in a browser `<canvas>`.
- CI: add `wasm32` build check job (`cargo build --target wasm32-unknown-unknown` + `wasm-pack test --headless --chrome`).
Exit Criteria:
- All non-platform crates compile for `wasm32-unknown-unknown` with zero errors.
- `demo-web` runs in Chrome/Firefox on representative screens without panics.
- Frame time target (avg <= 16.6ms, p95 <= 20ms) achieved in Chrome DevTools.
- `wasm32` build check is green in CI.
Crates affected: `webgpui-platform` (trait), new `webgpui-platform-web`, `webgpui-render-wgpu` (WebGPU/WebGL2), `webgpui-core`, `webgpui-input`, `webgpui-app`, new `apps/demo-web`.
Risks:
- `std::time::Instant` unavailable on `wasm32`. Mitigation: gate with `cfg(target_arch = "wasm32")`; use `web-sys` `performance.now()`.
- wgpu WebGPU browser support varies. Mitigation: `webgl` fallback as CI default; `webgpu` opt-in via feature flag.
- Pointer event coordinate semantics differ on touch devices. Mitigation: normalize in `webgpui-platform-web`; document known differences.

## 4. Cross-Cutting Tracks
- Performance: keep avg frame <= 16.6ms and p95 <= 20ms on target scenes.
- Reliability: minimize panic usage and improve actionable error messages.
- Documentation: every milestone must update EN/JA docs and changelog notes.

## 5. Suggested PR Strategy
- Keep one milestone split into small PRs (review, refactor, docs).
- Require tests or validation note for each functional PR.
- Avoid coupling architecture changes with large behavior changes in one PR.
