# WebUI ロードマップ

## 1. 目的
現在の「GPU UIコア」から、実運用可能な WebUI 基盤まで段階的に到達する。

## 2. 現状（2026-05）
- 実装済み: ウィンドウ/イベントループ、wgpu 描画、基本プリミティブ、入力取得、最小ノードツリー、dirty 管理の基礎、プロファイル基礎。
- 不足: Web 配備経路、本格テキスト基盤、イベント伝播モデル、高度レイアウト、アクセシビリティ、標準コンポーネント層、移行完了の定量指標。

## 3. マイルストーン

### M0: 安定化とCI基盤（完了 ✓）
範囲:
- fmt/test/gate の継続グリーン化。
- warning 回帰の抑止。
- crate と docs の責務オーナー明確化。
完了条件:
- main ブランチで 7 日連続 CI 成功。
- PR チェックで rustfmt 失敗ゼロ。

### M1: 入力とイベントモデル（完了 ✓）
範囲:
- ポインター座標の一貫性を完成。
- compat 側APIに capture/bubble 伝播を導入。
- フォーカス挙動とキーボード遷移の基準化。
完了条件:
- capture -> target -> bubble の順序テストを追加。
- Tab/Shift+Tab のフォーカステストを追加。

### M2: テキストとレイアウト基盤（完了 ✓）
範囲:
- 実運用向けテキスト基盤（フォント読込、shaping対応可能なインターフェース）。
- テキスト計測と折返しの基礎実装。
- 主要画面で Flex 相当に近いレイアウトを実装。
完了条件:
- 混在文字列の安定描画を確認。
- レイアウト fixture の再現性を確保。

### M3: ブラウザ UI コンポーネント層（完了 ✓）

機能的なブラウザ UI を構築するために最低限必要なウィジェット群。
各サブマイルストーンは独立してリリースし、CI をグリーンに保つ。

#### M3-A: コアインタラクティブウィジェット（3-4週間）
範囲:
- **Button**: 5 状態（normal / hover / pressed / focused / disabled）、Enter/Space 活性化。
- **TextInput**: カーソル移動、選択範囲、プレースホルダーテキスト、Backspace/Delete 処理。
- **Label**: 複数行テキスト描画、テキスト揃え（start / center / end）。
完了条件:
- 各ウィジェットの状態遷移ユニットテストが通る。
- `demo-basic` に Label + TextInput + Button のフォームを追加。

#### M3-B: 構造ウィジェット（2-3週間）
範囲:
- **ScrollView**: オーバーフロークリッピング、スクロールオフセット追跡、スクロールバー描画（省略可）。
- **Toolbar**: ギャップ基準の水平レイアウトバー。
- **TabBar + Tab**: 選択状態、矢印キー切替、Home/End。
完了条件:
- ScrollView オーバーフロークリッピングの fixture テストが通る。
- TabBar キーボードトラバーサルテスト（左右矢印・Home/End の折り返し）が通る。

#### M3-C: オーバーレイと Z オーダー（2-3週間）
範囲:
- **Z オーダーシステム**: `LayoutNode` に整数 `layer` フィールドを追加し、layer 順で描画。
- **Dialog**: モーダル背景、フォーカストラップ（Tab はダイアログ内で折り返し）、Escape で閉じる。
- **ContextMenu**: 位置アンカーポップアップ、外部クリックまたは Escape で閉じる。
完了条件:
- Dialog フォーカストラップテスト: 最後のフォーカス可能要素から Tab で先頭に戻る。
- ContextMenu 閉鎖テスト: 外部クリックイベントでメニューが閉じる。

#### M3-D: アクセシビリティとポリッシュ（2-3週間）
範囲:
- **ロールメタデータ**: ARIA 相当の `role` フィールド（`button` / `textbox` / `tab` / `dialog` / `menu`）。
- **フォーカスリング標準化**: 全 M3 ウィジェットで 2 px インセットリングを統一。
- **キーボードオーディット**: 全 M3 ウィジェットをマウスなしで完全操作可能にする。
完了条件:
- `role` フィールドがノードデータ構造に存在し、app 層から参照可能。
- キーボードのみで「フォームデモ」（Label + TextInput + Button + Dialog）を操作できる。
- `cargo test -p webgpui-app --all-targets` が通る。

影響クレート: `webgpui-core`（ウィジェットステートマシン、NodeRole）、`webgpui-app`（DrawContext ウィジェットヘルパー）、`webgpui-batching`（ウィジェットジオメトリのバッチ生成）、`apps/demo-basic`。

### M4: 移行と置換妥当性の検証（進行中 🚧）
前提条件 — 達成済み:
- `webgpui-compat` クレートの MUST ティア全 21 API が実装済み（`NotSupported` スタブを完全実装に置換）。
- `apps/demo-migration` アプリが作成済み（100% MUST API をカバー）。
範囲:
- APIマッピング網羅と MUST 互換チェックの完成。
- 代表レガシー画面を `apps/demo-migration` で再現。
- 移行工数（変更行数・非対応API数）と性能差分の定量化。
完了条件:
- API 置換率 >= 80%。
- 画面再現率 >= 90%。
- 要件サマリーの性能目標を満たす。
- MUST ティア全 API の Compat/FastPath 同等性テストが合格（api-swapping-quality-plan.md §8 参照）。
影響クレート: `webgpui-compat`（新規）、`webgpui-app`、`webgpui-core`、`webgpui-input`、`apps/demo-migration`（新規）。
リスク:
- シェーピングバックエンドの違いにより、compat とレガシーでテキスト位置がずれる。緩和策: ビジュアルスナップショットで ±2px の許容値を設定；既知の差分を文書化。
- capture/bubble 順序のタイミング差。緩和策: イベントトレーステストで厳密な順序を固定；compat 層で差分を吸収。
- `webgpui-compat` API スコープの肥大化。緩和策: §13.4 凍結を強制；追加には明示的な PR を必須。

> **M4 並走トラック — 性能P2（dirty rect）:**
> - `mark_dirty_rect` / `commit_dirty` をレンダーパイプライン（`webgpui-render`）に統合。
> - dirty 領域がない場合のレンダーパススキップを有効化。
> - `P2_GPU_SKIP_RATIO` メトリクスを `.ci/` メトリクス形式に追加。
> - 受入条件: 更新なし画面で GPU 時間が継続低下すること；CIのP2ゲートで検証。
> - このトラックは M4 と並走し、M4 の完了条件はブロックしない。
> - 影響クレート: `webgpui-core`（DirtyTracker）、`webgpui-render`（スキップロジック）、`webgpui-render-wgpu`（シザー）、`webgpui-batching`（バッチカリング）、`webgpui-app`（mark_dirty_rect API）。

### M5: API安定化 — ✓ 完了（2026-05）
範囲:
- `webgpui-app`、`webgpui-core`、`webgpui-input`、`webgpui-compat` の公開 API 全体のドキュメント整備と仕様確定。
- semver ポリシー（v0.x）宣言: patch = バグ修正のみ、minor = 追加のみ、major = MUST ティア型・関数への破壊的変更。
- `docs/semver-policy.md` を公開し、`docs/contributing.md` からリンク。
- M4 で移行候補として特定された API に `#[deprecated]` アノテーションを追加。
完了条件:
- MUST ティア全公開 API に `# Example` ブロック付きの rustdoc が存在する。
- `docs/semver-policy.md` が存在し、contributing ガイドからリンクされている。
- M0〜M5 のエントリを含む `CHANGELOG.md` を作成。
- 影響クレートの公開アイテムに `#[allow(missing_docs)]` 抑止ゼロ。
影響クレート: `webgpui-app`、`webgpui-core`、`webgpui-compat`、`webgpui-input`。

### M6: ビジュアル機能拡張 — ✓ 完了（2026-05）
範囲:
- 画像レンダリング: `image` クレートで PNG/JPEG を読み込み、GPU テクスチャへアップロード；`DrawContext::load_image` / `draw_image` API；パスをキーとした `ImageRegistry` キャッシュ。
- SVG ラスタライズ: `resvg`/`tiny-skia` 経由；`DrawContext::draw_svg` / `load_svg`；`(path, width, height)` でキャッシュ。
- フィルタエフェクト: `webgpui-render-graph` にガウシアンぼかし・5×4 カラー行列ポストプロセスパスを追加；`feature = "filters"` で管理；無効時はバイナリサイズへの影響ゼロ。
完了条件:
- PNG/JPEG 画像ノードが正しく描画される。✓
- シンプルな SVG アイコン（フラットパス）がビジュアルリグレッションなしで描画される。✓
- `filters` フィーチャー無効時、フィルタパスがバイナリから除外される。✓
影響クレート: `webgpui-render-wgpu`（GPU アップロード、フィルタシェーダー）、`webgpui-render-graph`（フィルタパス）、`webgpui-app`（画像/SVG/フィルタ API）。

### M7: アニメーションとトランジション — ✓ 完了（2026-05）
範囲:
- `webgpui-app` で `Animation` ビルダーを公開: `Animation::opacity / translate_x / translate_y(node_id, from, to)` に `.duration_ms()` / `.easing()` チェーン。
- `Easing` 列挙型: `Linear`、`EaseIn`、`EaseOut`、`EaseInOut`、`CubicBezier(x1, y1, x2, y2)` — `Easing::sample(t)` は三次多項式；cubic-bézier は16回二分探索。
- `AnimationTimeline`（内部）: ユーザーコールバック前に毎フレーム進行；補間値を `NodeTree` に書き込み；アクティブなアニメーションがある間 `dirty.mark_all()` を呼び出し。
- スタイルトランジション: `NodeStyle::transition` が `Some(TransitionConfig { duration_ms })` の場合、`DrawContext::set_style` が変更プロパティに対して暗黙的なアニメーションを生成。
- `NodeStyle` に `translate_x: f32`、`translate_y: f32`、`transition: Option<TransitionConfig>` を追加。
- 経過時間ベースの補間（フレームカウントベースではない）；外部アニメーションクレート不使用。
完了条件:
- `opacity` フェードが5点線形キーフレームチェックに合格（`opacity_fade_keyframes_linear`）。✓
- `translate_y` スライドが5点 ease-out 形状チェックに合格（`translate_slide_keyframes_ease_out`）。✓
- アニメーションなしシーンでリグレッションなし: アクティブなアニメーションがない場合 `tick()` は即時リターンし `mark_all()` を呼ばない。✓
- アクティブなアニメーションがある間はティックごとに必ず dirty マーク（`animation_tick_marks_dirty_when_active`）。✓
影響クレート: `webgpui-app`（アニメーション API、タイムライン）、`webgpui-core`（`NodeStyle`、`TransitionConfig`）。

### M8: デベロッパーツール — ✓ 完了（2026-05）
範囲:
- **`dev-tools` フィーチャーフラグ**を `webgpui-app` と `webgpui-profiler` に追加；無効時はバイナリコストゼロ。
- **パフォーマンスオーバーレイ**: FPS、avg/p95 フレーム時間、draw call 数を既存の `DrawList::fill_rect` のみで左上に描画 — 追加 GPU リソース不要。
- **ノードインスペクターオーバーレイ**: `DrawContext::dev_inspect(node_id)` で指定したノードの id、kind、role、opacity、visible、translate x/y、背景色を右上に表示。
- **dirty rect ティント**: `DirtyTracker::effective_area` に半透明の色付き領域をフレームごとに描画。
- `webgpui-app::dev_tools` 内に A–Z、0–9、一般記号をカバーする 3×5 ビットマップフォントを実装。
完了条件:
- ✓ `dev-tools` なし `--release` ビルドへの影響なしでパフォーマンスオーバーレイが正しく描画される。
- ✓ インスペクターが MUST ティア全スタイルプロパティの計算済みスタイルを正確に反映する。
- ✓ `dev-tools` 無効時のバイナリサイズ増加: ゼロ（モジュール全体が `#[cfg]` で除外）。
影響クレート: `webgpui-app`（インスペクター API、オーバーレイモジュール）、`webgpui-profiler`（フィーチャーフラグ）。

### M9: 性能深化 — P3/P4（4-6週間）
範囲:
- **P3 — 転送・キャッシュ最適化:**
  - `webgpui-render-wgpu` の頂点/インデックスアップロードにリングバッファを導入；フレームごとの `create_buffer` 呼び出しを排除。
  - 一時バッファプール: 短命バッファを固定サイズプールで再利用。
  - `prewarm_pipeline(desc)`: 起動時に wgpu パイプラインをコンパイル・キャッシュ。
  - `prewarm_glyph_cache(font, charset)`: 初回フレーム前に指定文字セットをプリラスタライズ。
- **P4 — レンダーグラフと並列化:**
  - `webgpui-render-graph`: 明示的なパス依存関係宣言；dirty 入力がないパスを自動スキップ。
  - UI ツリー更新（メインスレッド）とレンダーコマンドエンコード（`rayon` または `std::thread`）を分離。
  - `webgpui-core` のホットデータに SoA（Struct of Arrays）レイアウトを適用。
完了条件:
- 起動時にフレーム時間 50ms 超なし（スタッタリング解消）。
- 定常フレームのヒープアロケーション回数 = 0（`dhat` またはカスタムフックで計測）。
- 500 ノード以上のシーンで p95 フレーム時間 <= 20ms。
- レンダーパス自動スキップ検証: dirty 領域なしフレームで GPU サブミッションゼロ。
影響クレート: `webgpui-render-wgpu`（リングバッファ、一時プール、prewarm）、`webgpui-render-graph`（依存グラフ、自動スキップ）、`webgpui-core`（SoA）、`webgpui-app`（prewarm API）。
リスク:
- ワーカースレッドでのレンダーエンコード: 一部プラットフォームで wgpu `Surface` が `Send` でない。緩和策: `CommandBuffer`（`Send`）のみワーカーでエンコード；サブミットはメインスレッドで。
- SoA リファクタは大きな構造変更。緩和策: 専用 PR でスナップショット + 性能の before/after を必須化。

### M10: Web / WASM 配備（4-8週間）
範囲:
- `webgpui-platform-winit` 以外の全クレートを `wasm32-unknown-unknown` でコンパイル可能にする。
- プラットフォーム抽象化: `webgpui-platform` が `PlatformBackend` トレイトを定義；`webgpui-platform-winit` がネイティブ実装；新規 `webgpui-platform-web` が `web-sys` を使ってブラウザ実装。
- wgpu バックエンド: 対応ブラウザで `WebGPU` フィーチャー；未対応では wgpu の `webgl` フィーチャーで `WebGL2` にフォールバック。
- イベントブリッジ: `web-sys` DOM イベント（mouse、keyboard、resize、pointer）を `webgpui-input` イベント型にマッピング。
- `apps/demo-web`: ブラウザの `<canvas>` で `demo-basic` シーンを動かす `trunk` 対応バイナリクレート。
- CI: `wasm32` ビルドチェックジョブを追加（`cargo build --target wasm32-unknown-unknown` + `wasm-pack test --headless --chrome`）。
完了条件:
- プラットフォーム非依存の全クレートが `wasm32-unknown-unknown` でエラーゼロでコンパイルされる。
- `demo-web` が代表画面で Chrome/Firefox 上でパニックなしで動作する。
- フレーム時間目標（avg <= 16.6ms、p95 <= 20ms）を Chrome DevTools で達成。
- CI の `wasm32` ビルドチェックがグリーン。
影響クレート: `webgpui-platform`（トレイト）、新規 `webgpui-platform-web`、`webgpui-render-wgpu`（WebGPU/WebGL2）、`webgpui-core`、`webgpui-input`、`webgpui-app`、新規 `apps/demo-web`。
リスク:
- `std::time::Instant` が `wasm32` で使用不可。緩和策: `cfg(target_arch = "wasm32")` でゲート；`web-sys` の `performance.now()` を使用。
- wgpu WebGPU のブラウザサポート状況が異なる。緩和策: CI デフォルトは `webgl` フォールバック；`webgpu` はフィーチャーフラグでオプトイン。
- タッチデバイスでのポインターイベント座標セマンティクスの差異。緩和策: `webgpui-platform-web` で正規化；既知の差分を文書化。

## 4. 横断トラック
- 性能: 対象画面で avg frame <= 16.6ms、p95 <= 20ms を維持。
- 信頼性: panic を最小化し、原因追跡可能なエラーを整備。
- ドキュメント: 各マイルストーンで EN/JA docs を同時更新。

## 5. PR運用方針
- 1マイルストーンを小さなPR単位（review/refactor/docs）に分割。
- 機能PRには必ずテスト、または検証ログを添付。
- 大きな設計変更と挙動変更を1PRに混在させない。
