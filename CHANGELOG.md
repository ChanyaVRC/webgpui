# Changelog

All notable changes are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/) — see [docs/semver-policy.md](docs/semver-policy.md).

---

## [Unreleased] — M5: API Stabilization

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
