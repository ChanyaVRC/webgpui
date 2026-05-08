# クレート依存関係マップ

現在の `Cargo.toml` ファイルをもとに作成した実際の依存関係を示します。
`workspace-architecture.md`（設計提案）とは異なり、ワークスペースの実態を反映しています。

## 依存関係グラフ

```
Layer 0（内部依存なし）
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

Layer 5（アプリケーション）
  webgpui-app             → geometry, profiler, input, core, render,
                            render-wgpu, render-graph
                            [render-cpu: オプション機能]
```

## 依存関係一覧

| クレート | 依存先 |
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

\* オプション機能

## フィーチャーフラグ

| クレート | フィーチャー | 有効化される機能 |
|---|---|---|
| `webgpui-render-graph` | `filters` | `PassKind::Filter`、`FilterKind`、`BlurParams`、`ColorMatrixParams` |
| `webgpui-render-wgpu` | `filters` | WGSL フィルタシェーダー、オフスクリーンテクスチャ、`→ render-graph/filters` |
| `webgpui-app` | `filters` | `AppBuilder::enable_filter`、`AppConfig::filters`、`→ render-wgpu/filters` |
| `webgpui-render-wgpu` | `test-gpu` | GPU が必要なテスト（CI では lavapipe を使用） |
| `webgpui-app` | `backend-cpu` | `webgpui-render-cpu` を取り込む |

## 主要な外部依存（M6以降）

| クレート | 使用元 | 用途 |
|---|---|---|
| `image` | `webgpui-render-wgpu`、`webgpui-app` | PNG/JPEG デコードとピクセルレイアウト |
| `resvg` / `tiny-skia` | `webgpui-app` | SVG を RGBA ピクセルへラスタライズ |

## 補足

- このワークスペースに `webgpui` ファサードクレートは存在しません。`webgpui-app` が最上位の統合クレートです。
- `webgpui-render-graph` は `webgpui-geometry`（`Color` → `ClearColor` 変換のため）と `webgpui-batching` に依存しており、`webgpui-render` への直接依存はありません。
- `webgpui-app` は `webgpui-platform` および `webgpui-platform-winit` に依存しません。これらはエンベッダー側で使用されます。
- `webgpui-render-cpu` と `webgpui-render-cuda` は `webgpui-render-wgpu` と同じ `geometry + render` インターフェースを持つ代替バックエンドです。
