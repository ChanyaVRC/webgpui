# API Mapping Table (Legacy API -> New API)

## 1. Position of This Document
This document is the frozen mapping (v0.1) to replace existing WebUI engines with `webgpui`.

- Compatibility layer: `webgpui-compat`
- New implementation target: `webgpui` / `webgpui-core` / `webgpui-app`
- Scope: MVP (basic container/text/image, key styles, key events)

## 2. Notation
- Legacy APIs use commonly seen WebUI naming
- New APIs use Rust-style type-safe naming
- Status:
	- `MUST`: mandatory in MVP
	- `SHOULD`: recommended in late MVP
	- `LATER`: out of MVP

## 3. Node/Tree APIs
| Legacy API | New API (Compat Layer) | New API (Core) | Status | Migration Note |
|---|---|---|---|---|
| `createNode(type)` | `compat::node_create(kind)` | `webgpui::Node::new(kind)` | MUST | Convert `type` to `NodeKind` |
| `appendChild(parent, child)` | `compat::node_append(parent, child)` | `webgpui::Tree::append(parent, child)` | MUST | Change no-return API to `Result<()>` |
| `insertBefore(parent, child, before)` | `compat::node_insert_before(...)` | `webgpui::Tree::insert_before(...)` | SHOULD | Error when `before` is invalid |
| `removeChild(parent, child)` | `compat::node_remove(parent, child)` | `webgpui::Tree::remove(parent, child)` | MUST | Invalidate `NodeId` after detach |
| `setText(node, text)` | `compat::text_set(node, text)` | `webgpui::Node::set_text(text)` | SHOULD | MVP supports only simple text |
| `setImage(node, src)` | `compat::image_set(node, src)` | `webgpui::Node::set_image(source)` | SHOULD | Placeholder rendering allowed in MVP |

## 4. Style APIs
| Legacy API | New API (Compat Layer) | New API (Core) | Status | Migration Note |
|---|---|---|---|---|
| `setStyle(node, key, value)` | `compat::style_set(node, key, value)` | `webgpui::Style::set(prop, value)` | MUST | Convert string keys to `StyleProp` enum |
| `setStyles(node, object)` | `compat::style_set_many(node, styles)` | `webgpui::Node::set_style(style)` | MUST | Update only diffs (mark dirty) |
| `getStyle(node, key)` | `compat::style_get(node, key)` | `webgpui::Style::get(prop)` | SHOULD | Computed values are future scope |
| `setPosition(node, x, y)` | `compat::style_position(node, x, y)` | `webgpui::Style::position(x, y)` | MUST | Units unified to logical px |
| `setSize(node, w, h)` | `compat::style_size(node, w, h)` | `webgpui::Style::size(w, h)` | MUST | Represent auto via `Option<f32>` |
| `setMargin(node, l, t, r, b)` | `compat::style_margin(node, ...)` | `webgpui::Style::margin(...)` | MUST | Expand shorthand in compat layer |
| `setPadding(node, l, t, r, b)` | `compat::style_padding(node, ...)` | `webgpui::Style::padding(...)` | MUST | Expand shorthand in compat layer |
| `setBackground(node, color)` | `compat::style_background(node, color)` | `webgpui::Style::background(color)` | MUST | Convert color strings to RGBA |
| `setBorder(node, width, color)` | `compat::style_border(node, width, color)` | `webgpui::Style::border(width, color)` | MUST | MVP has no border radius |
| `setOpacity(node, alpha)` | `compat::style_opacity(node, alpha)` | `webgpui::Style::opacity(alpha)` | MUST | Clamp out-of-range values |

## 5. Event APIs
| Legacy API | New API (Compat Layer) | New API (Core) | Status | Migration Note |
|---|---|---|---|---|
| `addEventListener(node, type, handler)` | `compat::event_on(node, ty, cb)` | `webgpui::Events::on(node, ty, cb)` | MUST | Handlers must be `Send + Sync + 'static` |
| `removeEventListener(node, type, handler)` | `compat::event_off(node, ty, id)` | `webgpui::Events::off(node, ty, id)` | SHOULD | Move from function-pointer matching to ID management |
| `dispatchEvent(node, event)` | `compat::event_dispatch(node, evt)` | `webgpui::Events::dispatch(node, evt)` | SHOULD | capture/bubble is MVP-basic only |
| `stopPropagation()` | `compat::event_stop_propagation(ctx)` | `EventContext::stop_propagation()` | MUST | Explicit propagation stop |
| `preventDefault()` | `compat::event_prevent_default(ctx)` | `EventContext::prevent_default()` | SHOULD | Default handling depends on input kind |
| `setFocus(node)` | `compat::focus_set(node)` | `webgpui::Input::focus(node)` | MUST | Emit focus-loss events |

## 6. Lifecycle/Execution APIs
| Legacy API | New API (Compat Layer) | New API (Core) | Status | Migration Note |
|---|---|---|---|---|
| `mount(root)` | `compat::app_mount(root)` | `webgpui::App::mount(root)` | MUST | Execute initial layout + first render |
| `unmount()` | `compat::app_unmount()` | `webgpui::App::unmount()` | SHOULD | Fix resource release order |
| `update(node, patch)` | `compat::node_update(node, patch)` | `webgpui::Tree::update(node, patch)` | MUST | Return dirty range by design |
| `requestRender()` | `compat::render_request()` | `webgpui::Renderer::request_frame()` | MUST | Coalesce when no updates |
| `setVSync(enabled)` | `compat::render_vsync(enabled)` | `webgpui::Renderer::set_vsync(enabled)` | MUST | Environment-dependent application delay |
| `resize(width, height)` | `compat::viewport_resize(w, h)` | `webgpui::Renderer::resize(size)` | MUST | Handle DPI changes together |

## 7. Measurement/Debug APIs
| Legacy API | New API (Compat Layer) | New API (Core) | Status | Migration Note |
|---|---|---|---|---|
| `getFPS()` | `compat::metrics_fps()` | `webgpui::Profiler::fps()` | SHOULD | Standardize moving-average window |
| `getFrameTime()` | `compat::metrics_frame_time()` | `webgpui::Profiler::frame_time()` | SHOULD | Include p95 by default |
| `enableOverlay(flag)` | `compat::debug_overlay(flag)` | `webgpui::Profiler::set_overlay(flag)` | LATER | MVP prioritizes log output |

## 8. Unsupported/Difference Items (MVP)
| Legacy API | Policy | Alternative |
|---|---|---|
| `setFilter(node, cssFilter)` | LATER | Temporarily use image pre-processing |
| `setTransition(node, ...)` | LATER | Use app-side timeline management |
| `setGridLayout(node, ...)` | LATER | MVP supports simple layout only |

## 9. Migration Template
```rust
// Before (legacy)
// let root = createNode("container");
// setStyle(root, "background", "#20242a");
// addEventListener(root, "click", on_click);

// After (webgpui-compat)
let root = compat::node_create(NodeKind::Container)?;
compat::style_set(root, "background", "#20242a")?;
let _listener_id = compat::event_on(root, EventType::Click, on_click)?;
compat::app_mount(root)?;
```

## 10. Freeze Rule (Confirmed Operation)
- In v0.1, `MUST` rows are frozen scope
- Breaking changes for `MUST` rows are allowed only in major releases
- Any mapping change must update migration notes in the same change

## 11. Performance-Oriented Native API Mapping (No Compat Layer)
This section defines a path that prioritizes performance over compatibility.

| Purpose | Legacy API (Representative) | Native API (New) | Effect | Caution |
|---|---|---|---|---|
| Frame begin/end | `requestRender()` + internal automation | `webgpui::FastPath::begin_frame_fast(ctx)` / `end_frame_fast()` | Reduce extra frame-boundary overhead | Call order must be strict |
| Batch submission | Layered `appendChild` + `setStyle` calls | `webgpui::FastPath::submit_batch(batch_key, instances)` | Reduce draw calls | More responsibility than high-level API |
| Differential update | `update(node, patch)` | `webgpui::FastPath::mark_dirty_rect(node, rect)` *(not yet implemented)* | Minimize redraw area | Dirty management must be accurate |
| Transient buffer | Allocate per use internally | `webgpui::FastPath::allocate_transient_buffer(size)` | Reduce allocation cost | Requires reuse rules |
| Pipeline prewarm | Lazy creation at first draw | `webgpui::FastPath::prewarm_pipeline(desc)` | Suppress initial stutter | Increases startup cost |
| Text prewarm | Create on first glyph use | `webgpui::FastPath::prewarm_glyph_cache(font, charset)` | Reduce typing-time stutter | Requires charset design |

### 11.1 Adoption Rules
- Do not use native APIs in initial migration of legacy screens
- Replace only measured bottleneck points with native APIs
- Make before/after measurement logs mandatory for native API adoption

## 12. Test Guarantee Rules (Confirmed)
- Every `MUST` row API must have Compat/FastPath equivalence tests
- Equivalence tests must compare at least return value, side effects, event order, and render output
- API specification changes must update corresponding equivalence tests in the same PR
- Merges for `MUST` APIs are blocked when equivalence tests fail

### 12.1 Reference
- API swap quality plan: `api-swapping-quality-plan.md`

## 13. `webgpui-compat` Crate — Minimum API Definition

This section defines the minimum required API for the `webgpui-compat` crate, which is a prerequisite for M4.

### 13.1 Crate Role
`webgpui-compat` is a thin translation layer. It accepts legacy-style function calls and delegates to the internal `webgpui-core` / `webgpui-app` / `webgpui-input` crates. It does **not** contain rendering logic.

### 13.2 Module Structure
```
webgpui-compat/src/
  lib.rs       — re-exports all public modules
  types.rs     — shared types (NodeId, NodeKind, StyleProp, EventType, EventContext, ListenerId, CompatError)
  node.rs      — node_create, node_append, node_remove, node_insert_before, node_update
  style.rs     — style_set, style_set_many, style_position, style_size, style_margin,
                  style_padding, style_background, style_border, style_opacity
  event.rs     — event_on, event_off, event_dispatch, event_stop_propagation,
                  event_prevent_default, focus_set
  app.rs       — app_mount, app_unmount, render_request, render_vsync, viewport_resize
```

### 13.3 Public Types (MVP-Minimum)
| Type | Kind | Description |
|---|---|---|
| `NodeId` | newtype (`u64`) | Opaque handle for a node. Invalid after `node_remove`. |
| `NodeKind` | enum | `Container`, `Text`, `Image` |
| `StyleProp` | enum | One variant per MUST-tier style key |
| `EventType` | enum | `Click`, `PointerMove`, `PointerDown`, `PointerUp`, `Scroll`, `KeyDown`, `KeyUp`, `Focus`, `FocusLost` |
| `EventContext` | struct | Carries event payload; exposes `stop_propagation()` and `prevent_default()` |
| `ListenerId` | newtype (`u64`) | Handle returned by `event_on`; passed to `event_off` |
| `CompatError` | enum | `InvalidNode`, `InvalidListener`, `StyleParseError(String)`, `InternalError(String)` |

All public functions return `Result<T, CompatError>`.

### 13.4 MUST-Tier API Scope (Initial Implementation Target)
Only the MUST-flagged rows from §3–§6 are in scope for the initial `webgpui-compat` implementation. SHOULD and LATER rows are deferred.

| Module | Functions |
|---|---|
| `node` | `node_create`, `node_append`, `node_remove`, `node_update` |
| `style` | `style_set`, `style_set_many`, `style_position`, `style_size`, `style_margin`, `style_padding`, `style_background`, `style_border`, `style_opacity` |
| `event` | `event_on`, `event_stop_propagation`, `focus_set` |
| `app` | `app_mount`, `node_update`, `render_request`, `render_vsync`, `viewport_resize` |

### 13.5 Crate Dependencies
```toml
[dependencies]
webgpui-core   = { path = "../webgpui-core" }
webgpui-app    = { path = "../webgpui-app" }
webgpui-input  = { path = "../webgpui-input" }
```
No direct dependency on `webgpui-render` or `webgpui-render-wgpu`.

### 13.6 Feature Flags
| Flag | Default | Description |
|---|---|---|
| (none) | — | MUST-tier APIs are always compiled in. No feature flags at MVP. |

SHOULD/LATER APIs will be gated behind `compat-full` in a later milestone.

### 13.7 Freeze Rule
All types and functions listed in §13.3 and §13.4 are frozen at v0.1. Changes require a major-version bump per the semver policy defined in M5.

## 14. `webgpui-batching` Crate — Responsibilities

`webgpui-batching` is responsible for merging draw commands into GPU-efficient batches.
It is distinct from `webgpui-render` (which owns the GPU submission pipeline) and is used
by the renderer backends but carries no platform-specific code.

### 14.1 Crate Role
- Accepts a `DrawList` of high-level draw commands.
- Groups commands by `BatchKey` (blend mode, texture, pipeline, z-order).
- Produces a flat `Vec<DrawBatch>` ready for upload and submission.

### 14.2 Milestone Involvement

| Milestone | Involvement |
|---|---|
| M3 | Generates batches for widget geometry (Button, TextInput, Label). |
| P2 (dirty rect) | Culls batches that fall entirely outside the dirty rect, reducing GPU submissions. |
| M9 (optimization) | Candidate for SoA vertex layout and ring-buffer batch recycling. |

### 14.3 Crate Dependencies
- `webgpui-geometry` (Rect, Color, Point)
- `webgpui-render` (DrawCommand, DrawList)
