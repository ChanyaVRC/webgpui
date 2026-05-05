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

### M3: Component and Accessibility Layer (3-4 weeks)
Scope:
- Build core components: Button, TextInput, List, Panel, Dialog.
- Add accessibility metadata and keyboard-operable paths.
- Define component API stability policy.
Exit Criteria:
- Keyboard-only operation for core component demos.
- Accessibility metadata exported in app-facing structures.

### M4: Migration and Replacement Validation (2-4 weeks)
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

### M5: API Stabilization (2-3 weeks)
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
Risks:
- API surface larger than anticipated. Mitigation: scope to MUST-tier only; SHOULD/LATER deferred.

### M6: Visual Feature Expansion (4-6 weeks)
Scope:
- Image rendering: `NodeKind::Image` backed by PNG/JPEG loading (`image` crate) and GPU texture upload via `webgpui-render`.
- Basic SVG rendering: rasterize SVG to texture via `resvg`/`usvg`; no live SVG node tree at MVP.
- Filter effects: blur and color matrix as post-process pass in `webgpui-render-graph`; gated behind `feature = "filters"`.
Exit Criteria:
- Image nodes render PNG/JPEG correctly in `demo-basic` and `demo-migration`.
- Simple SVG icons (flat paths, no text) render without visual regression (pixel diff <= 1%).
- Filter pass excluded from binary when `filters` feature is disabled.
Crates affected: `webgpui-core` (NodeKind), `webgpui-render` (texture pipeline), `webgpui-render-wgpu` (GPU upload), `webgpui-render-graph` (filter pass), `webgpui-app` (image API).
Risks:
- SVG rasterization is CPU-bound and may cause frame spikes. Mitigation: rasterize off-frame on background thread; cache result.
- Crate version conflicts (`image` vs `resvg`). Mitigation: pin versions in workspace `Cargo.toml`.

### M7: Animation and Transitions (3-5 weeks)
Scope:
- `webgpui-app` exposes an `Animation` builder: target node, target style properties, duration, easing function.
- Easing functions: `linear`, `ease-in`, `ease-out`, `ease-in-out`, cubic bezier.
- Each animation tick calls `mark_dirty_rect` on the animated node, integrating with the P2 dirty-rect system.
- Style transitions: implicit animations triggered by `style_set` when a transition duration is configured on the node.
- Animation timeline managed inside `webgpui-app`; no dependency on external animation crates at MVP.
Exit Criteria:
- `opacity` fade and `position` slide pass visual snapshot tests at 5 keyframe checkpoints.
- No frame-time regression (>5%) on scenes with zero active animations.
- Animation tick always marks dirty; no frame is skipped while an animation is active.
Crates affected: `webgpui-app` (animation API, timeline), `webgpui-core` (dirty integration).
Risks:
- Sub-frame timing precision on Windows (winit event loop granularity). Mitigation: use elapsed-time-based interpolation, not frame-count-based.

### M8: Developer Tools (3-4 weeks)
Scope:
- `webgpui-profiler` gains a rendered overlay mode: draws FPS, avg/p95 frame time, draw-call count onto render output.
- Node inspector overlay (gated behind `dev-tools` feature): shows hovered node's id, kind, computed style, dirty-rect bounds.
- Dirty rect visualization: render dirty regions as colored overlays.
- All dev-tool features gated behind `feature = "dev-tools"` in `webgpui-app` and `webgpui-profiler`; zero runtime cost when disabled.
Exit Criteria:
- Perf overlay renders correctly; no impact on `--release` builds without `dev-tools`.
- Inspector overlay reflects correct computed style for all MUST-tier style properties.
- Binary size delta with `dev-tools` disabled: < 1 KB increase vs no-flag baseline.
Crates affected: `webgpui-profiler` (overlay render), `webgpui-app` (inspector API), `webgpui-core` (data read), `webgpui-render` (overlay pass).
Risks:
- Inspector overlay rendering adds a second render pass. Mitigation: batch with existing profiler overlay pass.

### M9: Performance Deep Dive — P3/P4 (4-6 weeks)
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
- No single frame > 50ms at launch (startup stutter eliminated).
- Per-frame heap allocation count = 0 on steady-state frames (measured via `dhat` or custom hook).
- p95 frame time <= 20ms on a scene with >= 500 nodes.
- Render pass auto-skip verified: zero GPU submissions on frames with no dirty regions.
Crates affected: `webgpui-render-wgpu` (ring buffer, transient pool, prewarm), `webgpui-render-graph` (dependency graph, auto-skip), `webgpui-core` (SoA), `webgpui-app` (prewarm API).
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
