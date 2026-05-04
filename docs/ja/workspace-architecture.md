# Cargo Workspace 構成案（crate分割）

## 1. 目的
`webgpui` を Cargo workspace として分割し、責務を明確化して開発・保守をしやすくする。

- エンジン中核とプラットフォーム依存を分離
- 将来機能（text/image/layout）を追加しやすい構造
- テストしやすい小さな crate 単位を維持

## 2. 想定ディレクトリ構成
```text
webgpui/
  Cargo.toml                # workspace root
  crates/
    webgpui/                # facade: 利用者向け公開API
    webgpui-compat/         # 既存WebUIエンジン互換レイヤー
    webgpui-core/           # UIツリー、状態、差分計算
    webgpui-render/         # レンダラ抽象 + 共通描画データ
    webgpui-render-wgpu/    # wgpu実装
    webgpui-render-graph/   # パス構築・描画順最適化
    webgpui-batching/       # 描画コマンド集約・インスタンシング
    webgpui-profiler/       # CPU/GPU計測
    webgpui-platform/       # プラットフォーム抽象（window/event）
    webgpui-platform-winit/ # winit実装
    webgpui-input/          # 入力イベント/フォーカス管理
    webgpui-geometry/       # 座標系、矩形、色、変換など
    webgpui-layout/         # MVP簡易レイアウト（将来拡張）
    webgpui-app/            # ランタイム起動・アプリ統合
  apps/
    demo-basic/             # 最小サンプル（手動確認用）
    demo-migration/         # 既存エンジン移行検証サンプル
```

## 3. 各 crate の責務
### 3.1 `webgpui`（Facade）
- 外部公開用の入口 crate
- MVP で公開する最小 API を再エクスポート
- 内部 crate の詳細を隠蔽

### 3.2 `webgpui-compat`
- 既存 WebUI エンジン互換 API（Node/Style/Event）を提供
- 既存 API から `webgpui` API への変換を担当
- 移行時の差分警告（未対応プロパティ、動作差）を出力

### 3.3 `webgpui-core`
- UI ノードツリー管理（追加/削除/更新）
- 差分検出（dirty 管理）
- 描画に必要な中間表現生成

### 3.4 `webgpui-render`
- レンダラ抽象トレイト
- 描画コマンド/バッチ共通データ
- バックエンド非依存の描画契約

### 3.5 `webgpui-render-wgpu`
- `wgpu` の初期化
- パイプライン作成
- フレーム描画、リサイズ、VSync設定

### 3.6 `webgpui-platform`
- ウィンドウ・イベントループ抽象
- OS 依存機能の共通インターフェース

### 3.7 `webgpui-render-graph`
- 描画パス（clear/ui/overlay など）の依存関係管理
- ソートキーに基づく描画順最適化
- 将来的なマルチパス最適化の受け皿

### 3.8 `webgpui-batching`
- draw call 削減のためのコマンド集約
- インスタンシング対象の自動分類
- 頂点/インデックスバッファへの詰め替え最適化

### 3.9 `webgpui-profiler`
- CPU フレーム計測（update/render/submit）
- GPU timestamp query 計測
- MVPの性能閾値判定ロジック（将来CI利用）

### 3.10 `webgpui-platform-winit`
- `winit` 実装
- ウィンドウ生成、入力イベント受け取り
- マウス押下/離上/スクロールイベントは最新の論理カーソル座標を使用

### 3.11 `webgpui-input`
- マウス/キーボード状態管理（`InputState`、`InputEvent`）
- `EventPhase` 列挙型（Capture / Target / Bubble）と `dispatch()` によるDOM式三フェーズイベントルーティング
- `FocusManager`: タブ順レジストリ、Tab/Shift+Tab の折り返しトラバーサル、`handle_key` 統合フック

### 3.12 `webgpui-geometry`
- `Rect`, `Point`, `Size`, `Color` など共通型
- 依存の少ない基礎ユーティリティ

### 3.13 `webgpui-layout`
- MVP の簡易レイアウト（縦積み/絶対配置など）
- 将来的に Flex/Grid 相当へ拡張

### 3.14 `webgpui-app`
- アプリ実行フロー統合
- `platform` + `render` + `core` の接続

### 3.15 `apps/demo-basic`
- クリア描画、矩形描画、入力表示の確認
- M1 キーボードベースライン: Tab フォーカス移動（テキストボックス↔ボタン）、Enter/Space ボタン活性化、フォーカスリング
- CI のスモーク確認対象（将来的に）

### 3.16 `apps/demo-migration`
- 既存エンジン実装からの移行手順を実証するサンプル
- 同一 UI の新旧比較（見た目/入力/性能）を検証

## 4. crate 依存方針
依存は一方向にし、循環依存を禁止する。

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
  -> webgpui-render
  -> webgpui-geometry

webgpui-batching
  -> webgpui-render
  -> webgpui-geometry

webgpui-platform-winit
  -> webgpui-platform
  -> webgpui-input

webgpui-core
  -> webgpui-geometry
  -> webgpui-layout (必要最小限)
```

## 5. Cargo.toml（workspace root）草案
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

## 6. Feature フラグ方針
- `default = ["backend-wgpu", "platform-winit"]`
- `backend-wgpu`: `webgpui-render-wgpu` を有効化
- `platform-winit`: `webgpui-platform-winit` を有効化
- `compat`: `webgpui-compat` を有効化（既存エンジン移行向け）
- 将来: `text`, `image`, `svg` などを追加

## 7. MVP 実装順（速度最優先）
1. P0: 計測と描画ホットパス
- `webgpui-geometry`
- `webgpui-render`
- `webgpui-render-wgpu`
- `webgpui-profiler`

2. P1: draw call 削減
- `webgpui-batching`
- `webgpui-render-graph`（ソートとパス最適化の最小機能）

3. P2: 再描画抑制
- `webgpui-core`（dirty rect 連携）
- `webgpui-app`（render skip 制御）

4. P3: 移行と同等性検証
- `webgpui-input`
- `webgpui-platform` + `webgpui-platform-winit`
- `webgpui-compat`
- `webgpui`（Facade整備）

5. P4: 検証アプリ仕上げ
- `apps/demo-basic` + `apps/demo-migration`

## 7.1 P0 最小実装タスク（crate単位）
P0 は「計測可能な FastPath 描画が動く」までを最小スコープとする。

1. `webgpui-geometry`
- `Point`, `Size`, `Rect`, `Color` の最小型を定義
- FastPath で使う `BatchKey`（pipeline/material/z-order）を定義
- DoD: 単体テストで構造体の生成と比較が通る

2. `webgpui-render`
- `FastPath` トレイトを定義（`begin_frame_fast`, `submit_batch`, `end_frame_fast`）
- `FrameStats`（cpu_ms, gpu_ms, draw_calls）を定義
- DoD: モック実装で API 契約テストが通る

3. `webgpui-render-wgpu`
- `FastPath` の wgpu 最小実装（clear + rectangle batch）
- command encoder / render pass の最小経路を実装
- DoD: `apps/demo-basic` から FastPath で 1 フレーム描画できる

4. `webgpui-profiler`
- CPU 区間計測（update, build, encode, submit）
- GPU timestamp query の最小計測（ui pass）
- `.ci/p0-metrics.txt` 出力フォーマットを実装
- DoD: 1 実行でメトリクスファイルが生成される

5. `webgpui-app`
- `RenderMode::Compat | FastPath` の切替フラグを実装
- P0 ベンチ実行フック（固定シーン、固定フレーム数）を実装
- DoD: 同一シーンを Compat/FastPath 両方で実行できる

6. `apps/demo-basic`
- P0 ベンチ用固定シーン（矩形 1,000 要素）を追加
- `--benchmark p0 --output .ci/p0-metrics.txt` を受け付ける
- DoD: CI から単一コマンドでメトリクス生成できる

## 8. 設計ルール
- 公開 API は `webgpui` に集約し、下位 crate の public surface を最小化
- `unsafe` は `webgpui-render-wgpu` など必要箇所に限定
- エラー型は crateごとに定義し、上位層で変換して返却
- 型共有は `webgpui-geometry` に寄せ、重複定義を避ける

## 9. 受け入れ条件（構成段階）
- workspace 全体が `cargo check --workspace` を通る
- `apps/demo-basic` が起動し、ウィンドウ + クリア描画が動作する
- 入力イベントをログ出力して確認できる
- crate 間に循環依存がない
- `apps/demo-migration` で互換 API 経由の表示と入力が確認できる

## 10. 次アクション
- この構成案で workspace を初期化（`cargo new`）
- `demo-basic` を最小構成で起動
- 各 crate の `lib.rs` に最小トレイト/API を定義

## 11. レンダリング高速化の実装ルール
- フレーム中の `Vec` 再確保を避けるため、capacity を先に確保して再利用する
- ソートキーは 64bit 整数（pipeline/material/z-order）で比較回数を削減する
- 可能な範囲で SoA レイアウトを使い、キャッシュ効率を上げる
- 計測のない最適化は禁止し、`webgpui-profiler` の数値で意思決定する
