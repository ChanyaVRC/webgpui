# API リファレンス

`cargo doc` によって Rust ソースから自動生成されます。
以下のリンクからクレートごとに参照できます。

| クレート | 役割 |
|---|---|
| [webgpui-app](/webgpui/api/webgpui_app/) | アプリケーションエントリポイント・ウィンドウ/イベントループ管理 |
| [webgpui-core](/webgpui/api/webgpui_core/) | ノードツリー・ダーティ追跡・コアランタイム |
| [webgpui-geometry](/webgpui/api/webgpui_geometry/) | 共通ジオメトリ型（`Rect`・`Point`・`Size`・`Color` など） |
| [webgpui-layout](/webgpui/api/webgpui_layout/) | スタック/絶対配置レイアウトエンジン |
| [webgpui-render](/webgpui/api/webgpui_render/) | レンダラートレイト・描画コマンド・DrawList |
| [webgpui-render-graph](/webgpui/api/webgpui_render_graph/) | レンダーパスグラフとトポロジカルソート |
| [webgpui-batching](/webgpui/api/webgpui_batching/) | 描画コールバッチングと頂点パッキング |
| [webgpui-render-wgpu](/webgpui/api/webgpui_render_wgpu/) | wgpu GPU バックエンド |
| [webgpui-render-cpu](/webgpui/api/webgpui_render_cpu/) | CPU ソフトウェアレンダラー（ヘッドレス/テスト用） |
| [webgpui-render-cuda](/webgpui/api/webgpui_render_cuda/) | CUDA バックエンド（オプション機能 `backend-cuda`） |
| [webgpui-input](/webgpui/api/webgpui_input/) | 入力イベント型（キーボード・マウス・フォーカス） |
| [webgpui-compat](/webgpui/api/webgpui_compat/) | レガシー API 互換レイヤー |
| [webgpui-profiler](/webgpui/api/webgpui_profiler/) | フレームタイマーとパフォーマンスメトリクス |
| [webgpui-platform](/webgpui/api/webgpui_platform/) | プラットフォーム抽象化トレイト |
| [webgpui-platform-winit](/webgpui/api/webgpui_platform_winit/) | winit プラットフォーム実装 |
