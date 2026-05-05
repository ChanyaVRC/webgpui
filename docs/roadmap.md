# WebUI Roadmap

## 1. Objective
Close the gap from "GPU UI core" to a production-ready WebUI stack.

## 2. Current Status (May 2026)
- Done: window/event loop, wgpu rendering path, primitive drawing, basic input capture, minimal node tree, dirty tracking baseline, profiling baseline.
- Missing: web deployment path, mature text stack, event propagation model, advanced layout, accessibility, standard component layer, migration completion metrics.

## 3. Milestones

### M0: Stability and CI Baseline (1-2 weeks)
Scope:
- Keep CI green for fmt/test/gates.
- Remove warning regressions.
- Define ownership for crates and docs.
Exit Criteria:
- 7 consecutive days with no red main branch CI.
- Zero rustfmt failures in PR checks.

### M1: Input and Event Model (2-3 weeks)
Scope:
- Complete pointer semantics (move/down/up/scroll consistency).
- Add capture/bubble event propagation in compat-facing API.
- Normalize focus behavior and keyboard navigation baseline.
Exit Criteria:
- Event-order tests for capture -> target -> bubble.
- Focus traversal tests for Tab/Shift+Tab.

### M2: Text and Layout Foundation (3-5 weeks)
Scope:
- Introduce real text pipeline (font loading, shaping-ready interfaces).
- Implement baseline text metrics and wrapping.
- Expand layout toward Flex-like behavior for key screens.
Exit Criteria:
- Text rendering for mixed strings at stable positions.
- Reproducible layout results for predefined fixtures.

### M3: Browser UI Component Layer (10-14 weeks total, 4 sub-milestones)

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

### M4: Migration and Replacement Validation (2-4 weeks)
Prerequisites:
- `webgpui-compat` crate must exist with all MUST-tier APIs implemented (see api-mapping.md §13).
- `apps/demo-migration` app must be created as the validation target.
Scope:
- Finish API mapping coverage and MUST compatibility checks.
- Reproduce representative legacy screens in `apps/demo-migration`.
- Quantify migration cost and performance delta.
Exit Criteria:
- API replacement ratio >= 80%.
- Screen reproduction ratio >= 90%.
- Performance target met per requirements summary.

> **Parallel track during M4 — Performance P2 (dirty rect):**
> - Integrate `mark_dirty_rect` / `commit_dirty` into the render pipeline.
> - Enable render skip on unchanged frames.
> - Acceptance: GPU time continuously decreases on no-update frames.
> - This track runs alongside M4 and does not block M4 exit criteria.

### M5: API Stabilization (2-3 weeks)
Scope:
- Finalize and document all public-facing API surfaces.
- Declare semver policy (v0.x) and define backward-compatibility rules.
- Ensure documentation coverage for all MUST-tier APIs in EN/JA.
Exit Criteria:
- All MUST-tier public APIs have rustdoc with examples.
- semver policy document published and linked from contributing guide.
- No undocumented breaking changes since M4.

### M6: Visual Feature Expansion (4-6 weeks)
Scope:
- Image rendering (PNG/JPEG loading, GPU upload).
- Basic SVG rendering (paths, fill, stroke).
- Filter effects (blur, color matrix) as optional layer.
Exit Criteria:
- Image nodes render correctly in `demo-basic` and `demo-migration`.
- Simple SVG icons render without visual regression.

### M7: Animation and Transitions (3-5 weeks)
Scope:
- Frame-based animation timeline (easing, keyframe).
- Style transition support for opacity, position, size.
- Animation API surface defined in `webgpui-app`.
Exit Criteria:
- Opacity fade and position slide transitions pass visual snapshot tests.
- No frame-time regression on non-animated scenes.

### M8: Developer Tools (3-4 weeks)
Scope:
- In-process node inspector overlay (node tree, style, dirty rect visualization).
- Performance overlay (FPS, frame time, draw-call count).
- Inspector gated behind `dev-tools` feature flag.
Exit Criteria:
- Inspector overlay renders correctly without impacting production builds.
- All profiler fields surfaced in overlay match `webgpui-profiler` internal values.

### M9: Performance Deep Dive — P3/P4 (4-6 weeks)
Scope:
- P3: Ring-buffer and transient-buffer reuse; `prewarm_pipeline` / `prewarm_glyph_cache`.
- P4: Render graph optimization (explicit pass dependencies, auto-skip).
- Separate UI update from render preparation (worker threads).
Exit Criteria:
- Startup stutter eliminated (no single frame > 50ms at launch).
- p95 frame time stable at <= 20ms on representative screen with 500+ nodes.

### M10: Web / WASM Deployment (4-8 weeks)
Scope:
- `wasm32-unknown-unknown` target support for all non-platform crates.
- winit web backend integration (`web_sys` event bridge).
- wgpu WebGPU / WebGL2 backend selection at compile time.
- `apps/demo-web` targeting a browser canvas.
Exit Criteria:
- `demo-basic` runs in a modern browser via `wasm-pack` / `trunk`.
- Frame time target (avg <= 16.6ms, p95 <= 20ms) measured in Chrome DevTools.
- No WASM-specific panics on representative screens.

## 4. Cross-Cutting Tracks
- Performance: keep avg frame <= 16.6ms and p95 <= 20ms on target scenes.
- Reliability: minimize panic usage and improve actionable error messages.
- Documentation: every milestone must update EN/JA docs and changelog notes.

## 5. Suggested PR Strategy
- Keep one milestone split into small PRs (review, refactor, docs).
- Require tests or validation note for each functional PR.
- Avoid coupling architecture changes with large behavior changes in one PR.
