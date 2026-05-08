# Crate Dependency Map

Accurate as of the current Cargo.toml files. This reflects the real workspace state,
not the design proposal in `workspace-architecture.md`.

## Dependency Graph

```
Layer 0 (no internal deps)
  webgpui-geometry
  webgpui-profiler

Layer 1
  webgpui-render          → geometry
  webgpui-layout          → geometry

Layer 2
  webgpui-batching        → geometry, render
  webgpui-core            → geometry, layout
  webgpui-render-cpu      → geometry, render
  webgpui-render-cuda     → geometry, render

Layer 3
  webgpui-render-graph    → geometry, batching
  webgpui-input           → geometry, core
  webgpui-platform        → geometry, input

Layer 4
  webgpui-render-wgpu     → geometry, render, batching, render-graph
  webgpui-platform-winit  → geometry, input, platform
  webgpui-compat          → core, geometry, layout, input

Layer 5 (application)
  webgpui-app             → geometry, profiler, input, core, render,
                            render-wgpu, render-graph
                            [render-cpu: optional feature]
```

## Full Table

| Crate | Depends on |
|---|---|
| `webgpui-geometry` | — |
| `webgpui-profiler` | — |
| `webgpui-render` | geometry |
| `webgpui-layout` | geometry |
| `webgpui-batching` | geometry, render |
| `webgpui-core` | geometry, layout |
| `webgpui-render-cpu` | geometry, render |
| `webgpui-render-cuda` | geometry, render |
| `webgpui-render-graph` | geometry, batching |
| `webgpui-input` | geometry, core |
| `webgpui-platform` | geometry, input |
| `webgpui-render-wgpu` | geometry, render, batching, render-graph |
| `webgpui-platform-winit` | geometry, input, platform |
| `webgpui-compat` | core, geometry, layout, input |
| `webgpui-app` | geometry, profiler, input, core, render, render-wgpu, render-graph, render-cpu* |

\* optional feature

## Notes

- There is no `webgpui` facade crate in this workspace. `webgpui-app` is the top-level
  integration crate.
- `webgpui-render-graph` depends on `webgpui-geometry` (for `Color` → `ClearColor` conversion)
  and `webgpui-batching`, but not on `webgpui-render` directly.
- `webgpui-app` does not depend on `webgpui-platform` or `webgpui-platform-winit`; those
  are used by the embedder, not the app library itself.
- `webgpui-render-cpu` and `webgpui-render-cuda` are alternative backends that mirror the
  same `geometry + render` surface as `webgpui-render-wgpu`.
