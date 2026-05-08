# Cargo Workspace Structure Proposal (Crate Split)

## 1. Purpose
Split `webgpui` into a Cargo workspace to make responsibilities explicit and development/maintenance easier.

- Separate core engine logic from platform-dependent parts
- Keep structure extensible for future features (text/image/layout)
- Preserve testability with small crate units

## 2. Expected Directory Structure
```text
webgpui/
	Cargo.toml                # workspace root
	crates/
		webgpui/                # facade: public API entrypoint
		webgpui-compat/         # compatibility layer for legacy WebUI engines
		webgpui-core/           # UI tree, state, diff calculation
		webgpui-render/         # renderer abstraction + shared draw data
		webgpui-render-wgpu/    # wgpu implementation
		webgpui-render-graph/   # pass graph and render-order optimization
		webgpui-batching/       # draw command aggregation and instancing
		webgpui-profiler/       # CPU/GPU measurement
		webgpui-platform/       # platform abstraction (window/event)
		webgpui-platform-winit/ # winit implementation
		webgpui-input/          # input events/focus management
		webgpui-geometry/       # coordinates, rects, colors, transforms
		webgpui-layout/         # MVP simple layout (future expansion)
		webgpui-app/            # runtime boot and app integration
	apps/
		demo-basic/             # minimal sample (manual validation)
		demo-migration/         # migration validation sample for legacy engine
```

## 3. Responsibilities by Crate
### 3.1 `webgpui` (Facade)
- External public entrypoint crate
- Re-export minimal MVP public API
- Hide internal crate details

### 3.2 `webgpui-compat`
- Provide legacy-compatible APIs (Node/Style/Event)
- Translate legacy APIs into `webgpui` APIs
- Emit migration warnings (unsupported properties, behavior differences)

### 3.3 `webgpui-core`
- UI node tree management (add/remove/update)
- Diff detection (dirty tracking)
- Build intermediate render representation

### 3.4 `webgpui-render`
- Renderer abstraction trait
- Shared draw-command/batch data
- Backend-agnostic rendering contract

### 3.5 `webgpui-render-wgpu`
- `wgpu` initialization
- Pipeline creation
- Frame rendering, resize, VSync handling

### 3.6 `webgpui-platform`
- Window and event-loop abstraction
- Common interface for OS-dependent behavior

### 3.7 `webgpui-render-graph`
- Manage pass dependencies (clear/ui/overlay)
- Render-order optimization based on sort keys
- Foundation for future multi-pass optimization

### 3.8 `webgpui-batching`
- Aggregate commands to reduce draw calls
- Auto-classify instancing candidates
- Optimize vertex/index buffer packing

### 3.9 `webgpui-profiler`
- CPU frame metrics (update/render/submit)
- GPU timestamp query metrics
- MVP threshold validation logic (for future CI)

### 3.10 `webgpui-platform-winit`
- `winit` implementation
- Window creation and input event intake
- Mouse press/release/scroll events use the latest logical cursor position

### 3.11 `webgpui-input`
- Mouse/keyboard state handling (`InputState`, `InputEvent`)
- `EventPhase` enum (Capture / Target / Bubble) and `dispatch()` for DOM-style three-phase event routing
- `FocusManager`: tab-order registry, Tab/Shift+Tab traversal with wrap-around, `handle_key` integration hook

### 3.12 `webgpui-geometry`
- Shared types such as `Rect`, `Point`, `Size`, `Color`
- Low-dependency base utilities

### 3.13 `webgpui-layout`
- `Direction::Column` (default) and `Direction::Row` stack layout
- `flex_grow` for proportional main-axis space distribution
- `TextMeasure` trait (object-safe) + `DefaultTextMeasure` (pixel-font baseline)
- Text leaf nodes auto-sized from content and `font_size` via `TextMeasure`; wraps to available width
- `LayoutEngine::compute_with` accepts a custom `&dyn TextMeasure`

### 3.14 `webgpui-app`
- Integrate app runtime flow
- Connect `platform` + `render` + `core`

### 3.15 `apps/demo-basic`
- Validate clear render, rectangle render, input display
- M1 keyboard baseline: Tab focus traversal (textbox ↔ button), Enter/Space button activation, focus ring
- Future CI smoke target

### 3.16 `apps/demo-migration`
- Demonstrate migration path from legacy engine implementation
- Validate side-by-side behavior (visual/input/performance)

## 4. Crate Dependency Policy
Dependencies must be one-way; cyclic dependencies are forbidden.

```text
webgpui (facade)
	-> webgpui-app
	-> webgpui-compat
	-> webgpui-core
	-> webgpui-layout

webgpui-compat
	-> webgpui-core
	-> webgpui-layout
	-> webgpui-input
	-> webgpui-geometry

webgpui-app
	-> webgpui-core
	-> webgpui-input
	-> webgpui-render
	-> webgpui-render-graph
	-> webgpui-batching
	-> webgpui-profiler
	-> webgpui-platform
	-> webgpui-geometry

webgpui-render-wgpu
	-> webgpui-render
	-> webgpui-render-graph
	-> webgpui-batching
	-> webgpui-geometry

webgpui-render-graph
	-> webgpui-geometry
	-> webgpui-batching

webgpui-batching
	-> webgpui-render
	-> webgpui-geometry

webgpui-platform-winit
	-> webgpui-platform
	-> webgpui-input

webgpui-core
	-> webgpui-geometry
	-> webgpui-layout (minimum only)
```

## 5. Cargo.toml (Workspace Root) Draft
```toml
[workspace]
members = [
	"crates/webgpui",
	"crates/webgpui-compat",
	"crates/webgpui-core",
	"crates/webgpui-render",
	"crates/webgpui-render-wgpu",
	"crates/webgpui-render-graph",
	"crates/webgpui-batching",
	"crates/webgpui-profiler",
	"crates/webgpui-platform",
	"crates/webgpui-platform-winit",
	"crates/webgpui-input",
	"crates/webgpui-geometry",
	"crates/webgpui-layout",
	"crates/webgpui-app",
	"apps/demo-basic",
	"apps/demo-migration",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT OR Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
wgpu = "0.20"
winit = "0.30"
thiserror = "1"
tracing = "0.1"
smallvec = "1"
glam = "0.28"
```

## 6. Feature Flag Policy
- `default = ["backend-wgpu", "platform-winit"]`
- `backend-wgpu`: enable `webgpui-render-wgpu`
- `platform-winit`: enable `webgpui-platform-winit`
- `compat`: enable `webgpui-compat` (legacy migration)
- Future: add `text`, `image`, `svg`, etc.

## 7. MVP Implementation Order (Performance-First)
1. P0: measurement and rendering hot path
- `webgpui-geometry`
- `webgpui-render`
- `webgpui-render-wgpu`
- `webgpui-profiler`

2. P1: draw-call reduction
- `webgpui-batching`
- `webgpui-render-graph` (minimal sorting/pass optimization)

3. P2: redraw suppression
- `webgpui-core` (dirty-rect integration)
- `webgpui-app` (render-skip control)

4. P3: migration and equivalence validation
- `webgpui-input`
- `webgpui-platform` + `webgpui-platform-winit`
- `webgpui-compat`
- `webgpui` (facade preparation)

5. P4: validation-app completion
- `apps/demo-basic` + `apps/demo-migration`

## 7.1 P0 Minimal Implementation Tasks (Per Crate)
P0 minimum scope is: "measurable FastPath rendering works end-to-end".

1. `webgpui-geometry`
- Define minimal types: `Point`, `Size`, `Rect`, `Color`
- Define `BatchKey` for FastPath (`pipeline/material/z-order`)
- DoD: unit tests pass for struct construction and comparisons

2. `webgpui-render`
- Define `FastPath` trait (`begin_frame_fast`, `submit_batch`, `end_frame_fast`)
- Define `FrameStats` (`cpu_ms`, `gpu_ms`, `draw_calls`)
- DoD: API contract tests pass with mock implementation

3. `webgpui-render-wgpu`
- Implement minimal `FastPath` path in wgpu (clear + rectangle batch)
- Implement minimal command encoder/render pass path
- DoD: one frame can be rendered through FastPath from `apps/demo-basic`

4. `webgpui-profiler`
- CPU segment timing (`update`, `build`, `encode`, `submit`)
- Minimal GPU timestamp query timing (`ui pass`)
- Implement `.ci/p0-metrics.txt` output format
- DoD: one run generates metrics file

5. `webgpui-app`
- Implement `RenderMode::Compat | FastPath` switch
- Implement P0 benchmark hook (fixed scene, fixed frame count)
- DoD: same scene runs in both Compat and FastPath modes

6. `apps/demo-basic`
- Add fixed scene for P0 benchmark (1,000 rectangle elements)
- Accept `--benchmark p0 --output .ci/p0-metrics.txt`
- DoD: metrics can be generated by a single CI command

## 8. Design Rules
- Keep public API aggregated in `webgpui`; minimize lower-crate public surface
- Limit `unsafe` to necessary crates such as `webgpui-render-wgpu`
- Define error types per crate and map errors in upper layers
- Consolidate shared types in `webgpui-geometry` to avoid duplication

## 9. Acceptance Criteria (Structure Stage)
- Entire workspace passes `cargo check --workspace`
- `apps/demo-basic` launches with window + clear rendering
- Input events can be validated via logs
- No cyclic dependencies between crates
- `apps/demo-migration` confirms rendering/input via compatibility APIs

## 10. Next Actions
- Initialize workspace from this structure (`cargo new`)
- Launch `demo-basic` with minimal configuration
- Define minimal traits/APIs in each crate `lib.rs`

## 11. Rendering Optimization Implementation Rules
- Reuse pre-allocated `Vec` capacity and avoid per-frame reallocations
- Use 64-bit sort keys (`pipeline/material/z-order`) to reduce comparison overhead
- Prefer SoA layouts where effective for cache behavior
- Prohibit optimization without measurement; use `webgpui-profiler` metrics for decisions
