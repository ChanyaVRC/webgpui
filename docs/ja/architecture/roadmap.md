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

### M5: API安定化（2-3週間）
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
リスク:
- API 表面が想定より広い。緩和策: MUST ティアのみに絞る；SHOULD/LATER は後回し。

### M6: ビジュアル機能拡張（4-6週間）
範囲:
- 画像レンダリング: `image` クレートで PNG/JPEG を読み込み、`webgpui-render` 経由で GPU テクスチャへアップロード。
- 基本的な SVG レンダリング: `resvg`/`usvg` でテクスチャへラスタライズ；MVP ではライブ SVG ノードツリーなし。
- フィルタエフェクト: `webgpui-render-graph` にぼかし・カラー行列ポストプロセスパスを追加；`feature = "filters"` で管理。
完了条件:
- `demo-basic` と `demo-migration` で PNG/JPEG 画像ノードが正しく描画される。
- シンプルな SVG アイコン（フラットパス、テキストなし）がビジュアルリグレッションなしで描画される（ピクセル差分 <= 1%）。
- `filters` フィーチャー無効時、フィルタパスがバイナリから除外される。
影響クレート: `webgpui-core`（NodeKind）、`webgpui-render`（テクスチャパイプライン）、`webgpui-render-wgpu`（GPUアップロード）、`webgpui-render-graph`（フィルタパス）、`webgpui-app`（画像API）。
リスク:
- SVG ラスタライズは CPU バウンドでフレームスパイクを引き起こす可能性。緩和策: フレーム外でバックグラウンドスレッドに分離；結果をキャッシュ。
- クレートバージョン競合（`image` vs `resvg`）。緩和策: ワークスペース `Cargo.toml` でバージョンを固定。

### M7: アニメーションとトランジション（3-5週間）
範囲:
- `webgpui-app` で `Animation` ビルダーを公開: ターゲットノード、スタイルプロパティ、継続時間、イージング関数。
- イージング関数: `linear`、`ease-in`、`ease-out`、`ease-in-out`、三次ベジェ。
- アニメーションの各ティックで対象ノードの `mark_dirty_rect` を呼び出し、P2 dirty rect システムと統合。
- スタイルトランジション: ノードにトランジション継続時間が設定されている場合、`style_set` でアニメーションを暗黙的に発火。
- アニメーションタイムラインは `webgpui-app` 内で管理；MVP では外部アニメーションクレートに依存しない。
完了条件:
- `opacity` フェードと `position` スライドが5キーフレームチェックポイントでビジュアルスナップショットテストに合格。
- アクティブなアニメーションがないシーンでフレーム時間リグレッション（5%超）なし。
- アニメーション中はティックごとに必ず dirty マーク；フレームスキップなし。
影響クレート: `webgpui-app`（アニメーション API、タイムライン）、`webgpui-core`（dirty 統合）。
リスク:
- Windows で winit イベントループ粒度によるサブフレームタイミング精度の問題。緩和策: フレームカウントではなく経過時間ベースの補間を使用。

### M8: デベロッパーツール（3-4週間）
範囲:
- `webgpui-profiler` にレンダードオーバーレイモードを追加: FPS、avg/p95 フレーム時間、draw call 数を描画出力上に表示。
- ノードインスペクターオーバーレイ（`dev-tools` フィーチャーで管理）: ホバー中ノードの id、kind、計算済みスタイル、dirty rect 境界を表示。
- dirty rect 可視化: dirty 領域を色付きオーバーレイで描画。
- すべての開発ツール機能を `webgpui-app` と `webgpui-profiler` の `feature = "dev-tools"` でゲート；無効時はランタイムコストゼロ。
完了条件:
- `dev-tools` なし `--release` ビルドへの影響なしでペルフオーバーレイが正しく描画される。
- インスペクターが MUST ティア全スタイルプロパティの計算済みスタイルを正確に反映する。
- `dev-tools` 無効時のバイナリサイズ増加 < 1 KB（フラグなしベースライン比）。
影響クレート: `webgpui-profiler`（オーバーレイ描画）、`webgpui-app`（インスペクター API）、`webgpui-core`（データ参照）、`webgpui-render`（オーバーレイパス）。
リスク:
- インスペクターオーバーレイが第2レンダーパスを追加。緩和策: 既存のプロファイラーオーバーレイパスと統合してバッチ処理。

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
