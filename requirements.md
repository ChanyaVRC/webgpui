# WebUI GPU Engine Requirements (Rust)

## 1. Purpose
Develop a WebUI engine in Rust with GPU rendering support.

- High-speed rendering (GPU utilization)
- Cross-platform support (Linux / Windows first)
- Long-term extensibility (UI components, animation, text, input)

## 2. Scope
### 2.1 MVP (First Release)
- Window creation
- GPU context initialization
- Basic primitive rendering (rectangles, lines)
- Color and opacity settings
- Resize handling
- Input event capture (mouse, keyboard)
- Minimal UI node tree (simple layout)

### 2.2 Future Extensions (Out of MVP)
- Advanced layout (Flex/Grid-compatible)
- Optimized text rendering
- Image, SVG, filters
- Animation and transitions
- Accessibility support
- Developer tools (inspector, profiler)

## 3. Baseline Technologies
- Language: Rust (stable)
- GPU abstraction: wgpu (default) or CUDA (optional)
  - **wgpu (default backend)**: cross-platform GPU API abstraction
    - Works on any hardware with WebGPU support (NVIDIA, AMD, Intel, Apple Metal)
    - Simplified API, easier portability
  - **CUDA (alternative backend)**: NVIDIA-specific GPU compute
    - Higher performance potential via fine-grained GPU control
    - Requires NVIDIA GPU + CUDA Toolkit 11.8 or newer
    - Compute Capability 3.5+ (Maxwell generation or newer)
    - Platforms: Linux (x86_64), Windows (x86_64)
    - Compile-time selection via Cargo feature: `backend-cuda`
    - Must be functionally equivalent to wgpu backend (verified by equivalence tests)
- Window/events: winit
- Math: glam (as needed)
- Serialization: serde (as needed)

## 4. Functional Requirements
### 4.1 Rendering
- Provide a per-frame rendering loop
- Use a structure that supports draw command batching
- Support DPI scaling
- Allow VSync on/off configuration
- Include sorting strategy to minimize pipeline switches
- Support instanced rendering by design
- Reuse static geometry without re-uploading
- Manage render boundaries for partial redraw (dirty rect)

### 4.2 UI Tree
- Support node add/remove/update operations
- Preserve parent-child relationships
- Design for redraw-range detection on changes (future optimization)

### 4.3 Event Handling
- Capture mouse move/click/scroll
- Capture keyboard input
- Provide basic focus management behavior

### 4.4 API Design
- Provide a declarative API for app-side UI construction
- Hide internal implementation details and prioritize public API stability
- Return errors via Result and preserve traceable root causes

### 4.5 Legacy Engine Compatibility and Migration
- Provide a compatibility layer for major concepts (Node, Style, Event)
- Provide adapter APIs for direct replacement of existing APIs
- Target CSS compatibility; in MVP, prioritize frequent properties (position, size, margin, padding, background, border, opacity)
- Provide basic behavior equivalent to legacy event models (capture/bubble)
- Allow staged migration by supporting old/new engine coexistence in the same app (screen-level or component-level)
- Provide migration guide and API mapping table

## 5. Non-Functional Requirements
### 5.1 Performance
- Target frame rate: 60 FPS (standard environment)
- Suppress unnecessary redraw when UI is unchanged
- Keep startup time low (initial target: under 1 second)
- Frame-time target: average <= 16.6ms, p95 <= 20ms
- CPU main-thread budget: <= 4ms per frame (MVP target)
- GPU rendering budget: <= 8ms per frame (MVP target)
- Draw-call target: <= 200 on representative screens (MVP target)
- Memory allocation target: keep per-frame heap allocation close to zero

### 5.2 Reliability
- Provide clear error messages when GPU initialization fails
- Minimize panic usage and handle recoverable failures

### 5.3 Maintainability
- Modular separation (renderer, scene, input, platform)
- Responsibility split that supports unit testing
- Public API documentation coverage

### 5.4 Compatibility and Operations
- Define public API compatibility policy (semver)
- Quantify migration cost from existing engines (lines changed, replacement points)
- Require migration notes for backward-incompatible changes

## 6. Architecture Policy
- `platform`: window, events, OS-specific handling
- `renderer`: GPU abstraction and backend implementations (wgpu or CUDA)
  - Backend selection via Cargo feature flag (`backend-wgpu` or `backend-cuda`)
  - Dynamic runtime selection via `BackendSelector` enum in `webgpui-render`
  - Both backends implement the same `Renderer` trait contract
  - Query available backends at runtime: `BackendSelector::available()`
  - All backends must pass identical equivalence tests (visual, event, perf)
- `scene`: UI node management and diff updates
- `input`: input state and event dispatch
- `app`: application-facing API layer

## 7. Development Milestones
1. Project initialization (Cargo workspace and base modules)
2. Window + wgpu initialization
3. Clear-color rendering
4. Basic primitive rendering
5. Input event handling
6. Minimal UI tree integration
7. Sample app creation
8. Minimal testing/measurement

## 8. Acceptance Criteria (MVP)
- Launch and window display on target environments (wgpu default, CUDA optional)
- Render basic primitives at 60 FPS on both backends (when enabled)
- No display breakage on resize
- Input events are available in app layer
- UI node updates are reflected in sample rendering
- Reproduce at least one representative legacy UI screen via compatibility layer
- Provide at least one migration sample (Before/After)
- Visual output is pixel-identical between backends (wgpu and CUDA, when both available)

## 9. Risks and Mitigations
- GPU/driver variance: prepare backend selection and fallback policy
- API bloat: limit MVP to minimal public API surface
- Premature optimization: complete functionality first, then optimize by measurement

## 10. Next Actions
- Finalize project structure (crate split)
- Draft MVP API sketch
- Start minimal sample implementation (window + clear render)

## 12. Replacement Viability Criteria
- API replacement ratio: cover >= 80% of major APIs in the target legacy engine
- Screen reproduction ratio: visually reproduce >= 90% of target UI
- Performance requirement: at least 1.2x average FPS over legacy on same screen, or equal FPS with lower CPU/GPU usage
- Migration cost: migrate one representative screen within two engineer-days

## 11. Rendering Optimization Policy (Priority Order)
1. Measurement infrastructure first
- CPU: separately measure update/render times per frame
- GPU: pass-level measurement via timestamp query

2. Batching optimization
- Aggregate commands by same pipeline and same texture
- Send instancing-friendly primitives together

3. Data transfer optimization
- Use reusable buffers to reduce map/unmap frequency
- Manage dynamic update regions via ring buffer to reduce copy count

4. Redraw optimization
- Generate dirty rects from UI diffs and redraw only changed regions
- Provide mode to skip rendering on no-update frames and redraw on input

5. Pipeline optimization
- Create frequent pipelines at initialization
- Reduce shader branching and minimize vertex-format variants

6. Parallelization
- Separate UI update (CPU) and GPU submit preparation; allow future worker-thread offloading
