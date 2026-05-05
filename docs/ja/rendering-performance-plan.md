# レンダリング高速化 実行計画

## 1. 目標
レンダリングを中心に体感速度を向上させ、フレーム安定性を確保する。

- 平均フレーム時間: 16.6ms 以下
- p95 フレーム時間: 20ms 以下
- ドローコール: 200 以下（主要画面）
- 更新なしフレーム: 再描画スキップ可能

## 1.1 実装優先順位（速度最優先）
以下を固定優先順位とし、上位が完了するまで下位には着手しない。

1. P0: 計測と描画ホットパス
- CPU/GPU 計測を最初に実装（計測なし最適化を禁止）
- `begin_frame_fast` / `submit_batch` / `end_frame_fast` を先行実装
- 受け入れ条件: ボトルネック区間が数値で可視化され、FastPath が動作する

2. P1: ドローコール削減
- バッチングとインスタンシングを実装
- パイプライン切り替え回数を最小化するソートを実装
- 受け入れ条件: 主要画面で draw call <= 200

3. P2: 再描画削減
- `mark_dirty_rect` / `commit_dirty` を接続
- 更新なしフレームで render skip を有効化
- 受け入れ条件: 無更新時のGPU処理時間が継続的に低下

4. P3: 転送とキャッシュ最適化
- リングバッファ化と transient buffer 再利用
- `prewarm_pipeline` / `prewarm_glyph_cache` を導入
- 受け入れ条件: 初回描画時スタッタリングが減少

5. P4: 並列化と構造最適化
- render graph 最適化
- UI更新と描画準備の分離
- 受け入れ条件: p95 のさらなる安定化

## 1.2 後回しにする項目（速度優先のため）
- 高度レイアウト機能（Flex/Grid 完全互換）
- 見た目機能の拡張（filter/transition/SVG）
- 開発者向けUX改善（インスペクタ高度化）

## 2. 最優先タスク（Phase 1: 計測）
1. CPU 計測ポイントを追加
- update
- layout
- build draw list
- encode + submit

2. GPU 計測を追加
- clear pass
- ui pass
- overlay pass

3. 結果を 1 秒単位で集計
- average
- p95
- max

## 3. 高速化タスク（Phase 2: 即効性の高い最適化）
1. バッチング
- pipeline / texture / blend state ごとにソートして集約
- 小粒な draw call をインスタンシングへ寄せる

2. 転送量削減
- 頂点バッファをリングバッファ化
- 静的ジオメトリを GPU 常駐

3. 再描画抑制
- dirty rect を導入
- 変更がない場合は render pass を省略

## 4. 中期タスク（Phase 3: 構造最適化）
1. render graph 導入
- パス依存を明示化
- 不要パスを自動スキップ

2. データ構造最適化
- 頻出データを SoA 化
- アロケーション削減（事前 reserve）

3. 並列化
- UI更新と描画準備を分離
- ワーカースレッドでコマンド準備

## 5. 検証方法
1. ベースライン計測
- 最小サンプル
- 中規模 UI サンプル

2. 施策ごとの差分計測
- 1施策ずつ有効化
- before/after を保存

3. 回帰監視
- 一定以上の劣化で警告
- 主要指標を継続記録

## 6. 完了条件

### P0 完了
- CPU 計測ポイント（update / layout / draw-list / encode+submit）が毎フレーム値を出力。
- GPU 計測ポイント（clear / ui / overlay パス）が毎フレーム値を出力。
- `begin_frame_fast` / `submit_batch` / `end_frame_fast` が代表シーンでパニックなしで動作。
- CI P0 ゲートがグリーン（ci-gates.md 参照）。

### P1 完了
- 代表画面で draw call <= 200。
- パイプラインソートバッチング検証: バッチなしベースライン比で draw call 数が >= 30% 減少。
- CI P1 ゲートがグリーン。

### P2 完了（M4 並走トラック）
- `mark_dirty_rect` / `commit_dirty` をレンダーパイプライン（`webgpui-render`）に統合。
- dirty 領域なしフレームでレンダーパスをスキップ；GPU クエリで確認（サブミッション数 = 0）。
- `P2_GPU_SKIP_RATIO` メトリクスを `.ci/` メトリクス形式に追加し毎フレーム出力。
- 静的（更新なし）シーンで GPU 時間が継続的に低下することを計測で確認。
- CI P2 ゲートを追加してグリーン。

### P3 完了
- 定常フレームのヒープアロケーション回数 = 0（`dhat` またはカスタムフックで計測）。
- 起動スタッタリング解消: 起動時にフレーム時間 50ms 超なし。
- `prewarm_pipeline` と `prewarm_glyph_cache` の API が `demo-basic` で使用可能。

### P4 完了
- レンダーグラフが dirty 入力なしのパスを自動スキップ；完全静的フレームで GPU サブミッションゼロ。
- UI 更新とレンダーコマンドエンコードが別スレッドで動作；フレーム時間分散の顕著な増加なし。
- 500 ノード以上のシーンで p95 フレーム時間 <= 20ms。

### 総合完了
- 代表画面全体で p95 が目標内（<= 20ms）に収まる。
- 操作時のフレーム落ちが目視で大幅改善（手動確認でスタッタリングなし）。
- 全ボトルネックが計測ログで説明可能（説明不能なスパイクなし）。

## 7. 独自APIによる高速化方針
既存互換 API とは別に、性能重視のネイティブ API を追加して高速化を狙う。

1. 低オーバーヘッド描画 API
- `begin_frame_fast(frame_ctx)`
- `submit_batch(batch_key, instances)`
- `end_frame_fast()`

2. 差分更新 API
- `mark_dirty_rect(node_id, rect)`
- `commit_dirty()`

3. バッファ管理 API
- `allocate_transient_buffer(bytes)`
- `write_transient(slice)`
- `recycle_transient(frame_id)`

4. キャッシュ制御 API
- `prewarm_pipeline(pipeline_desc)`
- `prewarm_glyph_cache(font, charset)`
- `evict_cache(policy)`

## 8. 互換APIと独自APIの使い分け
1. 互換 API（`webgpui-compat`）
- 目的: 既存エンジンからの移行コスト最小化
- 特徴: 汎用性優先、変換処理コストあり

2. 独自 API（`webgpui` ネイティブ）
- 目的: 最大性能の引き出し
- 特徴: 低レベル制御、責務を呼び出し側にも要求

3. 推奨運用
- 初期移行は互換 API を使う
- ボトルネック箇所だけ独自 API に段階置換する

## 9. 導入ステップ
1. Phase A: 計測追加
- 互換 API 経由のベースラインを取得

2. Phase B: 独自API最小導入
- `begin_frame_fast` / `submit_batch` / `end_frame_fast` を実装
- draw call と CPU 時間の差分を確認

3. Phase C: 差分更新導入
- `mark_dirty_rect` を UI 差分と接続
- 更新なしフレームで render skip を有効化

4. Phase D: キャッシュ最適化
- pipeline/glyph の prewarm を起動時に実行
- スタッタリング低減を計測で確認

## 10. 受け入れ条件（独自API）
- 独自APIを適用した画面で平均フレーム時間を 15%以上改善
- 独自API適用時も互換APIと同等の表示正しさを維持
- fallback 経路（互換API）へ切り替えても機能劣化しない

## 11. テストによる品質保証（APIスワップ同等性）
独自APIへ切り替えても動作が変わらないことを、以下の自動テストで保証する。

1. スナップショット同等性テスト（表示）
- 同一シナリオを `compat` 経路と `fastpath` 経路で実行
- 同一フレーム番号で画像出力し、許容誤差付きで比較
- 合格基準: 差分ピクセル率 <= 0.5%、主要UI領域は <= 0.1%

2. イベントトレース同等性テスト（入力）
- click / move / key の入力列を固定リプレイ
- 発火イベント順序と payload を比較
- 合格基準: 順序一致 100%、payload 差分 0

3. 状態遷移同等性テスト（UIツリー）
- mount -> update -> unmount の各段階で内部状態を検証
- ノード数、親子関係、dirty rect を比較
- 合格基準: 構造一致 100%

4. プロパティテスト（ランダム更新）
- ランダムな style/update シーケンスを生成
- `compat` と `fastpath` の終状態が一致することを検証
- 合格基準: 失敗ケース 0（最小 10,000 シーケンス）

## 12. 回帰防止テスト（CI）
1. PR必須ジョブ
- `equivalence-visual`
- `equivalence-events`
- `equivalence-state`
- `perf-regression`

2. 性能回帰ゲート
- `fastpath` が `compat` より 10%以上遅くなったら失敗
- p95 フレーム時間が基準から 5%以上悪化したら失敗

3. 互換性回帰ゲート
- MUST API に関する同等性テストが 1 件でも失敗したらマージ不可

## 13. 実装ルール（テスト容易性）
- 実行経路を `RenderMode::Compat` / `RenderMode::FastPath` で明示切替可能にする
- 乱数を使う処理は seed 固定を可能にし、再現性を担保する
- 時刻依存処理は `Clock` 抽象を介してテストで固定する
- スナップショット比較用に deterministic 描画モードを用意する

## 14. 参照
- APIスワップ品質保証詳細: `api-swapping-quality-plan.md`

## 15. CIに先行導入する P0ゲート（最低基準）
P0 の完了判定は、まず CI の自動ゲートで機械的に判定する。

1. 判定指標（FastPath 単体）
- `AVG_FRAME_MS <= 16.6`
- `P95_FRAME_MS <= 20.0`
- `DRAW_CALLS <= 200`

2. 判定指標（Compat 比較）
- `FASTPATH_AVG_FRAME_MS <= COMPAT_AVG_FRAME_MS * 0.90`
- `FASTPATH_P95_FRAME_MS <= COMPAT_P95_FRAME_MS * 0.95`

3. メトリクスファイル形式
- 出力先: `.ci/p0-metrics.txt`
- 形式: `KEY=VALUE`（1行1項目）
- 必須キー:
	- `AVG_FRAME_MS`
	- `P95_FRAME_MS`
	- `DRAW_CALLS`
	- `COMPAT_AVG_FRAME_MS`
	- `COMPAT_P95_FRAME_MS`
	- `FASTPATH_AVG_FRAME_MS`
	- `FASTPATH_P95_FRAME_MS`

4. CI 失敗条件
- 必須キー欠落
- 数値変換不可
- いずれかの閾値違反

5. 関連ファイル
- ワークフロー: `.github/workflows/p0-gate.yml`
- 判定スクリプト: `scripts/ci/check_p0_gate.sh`
- 閾値定義: `.ci/p0-thresholds.env`
- 運用ガイド: `docs/ci-gates.md`
- メトリクス仕様: `docs/metrics-format.md`

## 16. P0完了後に導入する P1ゲート（バッチング効果）
P1 の完了判定は、バッチング適用前後の改善量を CI で機械判定する。

1. 判定指標（FastPath + バッチング適用後）
- `DRAW_CALLS_BATCHED <= 120`
- `SUBMIT_CALLS_BATCHED <= 4`

2. 判定指標（バッチング適用前との比較）
- `DRAW_CALL_REDUCTION_RATIO <= 0.60`
- `CPU_BUILD_MS_BATCHED <= CPU_BUILD_MS_UNBATCHED * 0.80`

3. メトリクスファイル形式
- 出力先: `.ci/p1-metrics.txt`
- 形式: `KEY=VALUE`（1行1項目）
- 必須キー:
	- `DRAW_CALLS_UNBATCHED`
	- `DRAW_CALLS_BATCHED`
	- `SUBMIT_CALLS_BATCHED`
	- `CPU_BUILD_MS_UNBATCHED`
	- `CPU_BUILD_MS_BATCHED`
	- `DRAW_CALL_REDUCTION_RATIO`

4. CI 失敗条件
- 必須キー欠落
- 数値変換不可
- いずれかの閾値違反

5. 関連ファイル
- ワークフロー: `.github/workflows/p1-gate.yml`
- 判定スクリプト: `scripts/ci/check_p1_gate.sh`
- 閾値定義: `.ci/p1-thresholds.env`
- 運用ガイド: `docs/ci-gates.md`
- メトリクス仕様: `docs/metrics-format.md`

## 17. P2 API仕様 — dirty rect 最適化

### 17.1 設計目標
- 公開エントリポイントは2つ: `mark_dirty_rect(rect)` と `commit_dirty(viewport) -> DirtyDecision`。
- `DirtyTracker`（既存、`webgpui-core`）がフレームごとの蓄積を担う。
- `DirtyDecision`（新規 enum、`webgpui-core`）が当該フレームのレンダリング決定を表す。
- `RenderGraph` が `DirtyDecision` を受け取り、パスのスキップや scissor rect を設定する。
- `App::run` のフレームループが蓄積とレンダラーを橋渡しする。
- 公開 `DrawContext` の描画コマンド API には変更なし；最適化は完全に内部で完結する。

### 17.2 新規型定義

#### `DirtyDecision`（`webgpui-core` に追加）
```rust
pub enum DirtyDecision {
    /// dirty 領域なし。全レンダーパスをスキップする。
    Skip,
    /// この画面領域のみ変化。scissor rect 内のみ再描画する。
    Partial(Rect),
    /// 全画面再描画が必要（リサイズ・初回フレーム・`mark_all` 呼び出し時）。
    Full,
}
```

#### `RenderOutcome`（`webgpui-render` に追加）
```rust
pub enum RenderOutcome {
    /// GPU コマンドバッファを投入した。
    Submitted,
    /// GPU 処理なし；フレームをスキップした。
    Skipped,
}
```

### 17.3 クレートごとの API 変更

**`webgpui-core` — `DirtyTracker`**

既存メソッドは変更なし。以下を追加:
```rust
/// `mark()` の利便性エイリアス。フレームコールバックや compat 層での使用を想定。
pub fn mark_dirty_rect(&mut self, rect: Rect);

/// 蓄積した dirty 状態を消費し、このフレームのレンダリング決定を返す。
/// 内部状態をクリアする。レンダリング前に1フレームにつき1回だけ呼び出す。
pub fn commit_dirty(&mut self, viewport: Size) -> DirtyDecision;
```
`commit_dirty` のロジック:
- rect 未記録かつ `full_invalidate == false` → `Skip`
- `full_invalidate == true` → `Full`、その後クリア
- rect 記録あり → `Partial(dirty_union())`、その後クリア

**`webgpui-render` — `Renderer` トレイト**

既存 `render()` に加えて追加:
```rust
/// GPU scissor rect を使い `area` 内のみ再描画する。
/// バックエンドが scissor をサポートしない場合は全画面フォールバック。
fn render_partial(&mut self, draw_list: &DrawList, area: Rect) -> RenderResult<RenderOutcome>;
```
既存の `render()` は全画面再描画のままで `RenderOutcome::Submitted` を返す。

**`webgpui-render-graph` — `RenderGraph`**

追加:
```rust
/// 実行前にパス設定へ dirty 決定を反映する。
/// - `Skip`: 全パスを無効化。
/// - `Partial(rect)`: `ui` パスに scissor rect を設定。clear パスは常に全画面。
/// - `Full`: 変更なし（全パス設定どおり実行）。
pub fn apply_dirty_decision(&mut self, decision: &DirtyDecision);
```

**`webgpui-app` — フレームループ**

現在 `_` 変数の3つをアクティブ化:
```rust
// Before: let _dirty_tracker = DirtyTracker::new();
// After:  let mut dirty_tracker = DirtyTracker::new();
```
フレームループ疑似コード:
```
on RedrawRequested:
  let viewport = Size::new(sw as f32, sh as f32);
  let decision = dirty_tracker.commit_dirty(viewport);

  if matches!(decision, DirtyDecision::Skip) {
      frame_timer.record_skip();
      p2_skip_counter += 1;
      // GPU 処理なし。
      return;
  }

  draw_list.clear();
  frame_fn(&mut ctx);
  renderer.render_graph_mut().apply_dirty_decision(&decision);

  match decision {
      DirtyDecision::Partial(area) => renderer.render_partial(&draw_list, area),
      _                            => renderer.render(&draw_list),
  };
  p2_total_counter += 1;
```

**`webgpui-app` — `DrawContext`**

フレームコールバック内から呼び出すために追加:
```rust
/// このフレームで再描画が必要な画面領域を登録する。
/// アクティブなフレーム外で呼び出しても効果なし。
pub fn mark_dirty_rect(&mut self, rect: Rect);
```

### 17.4 `NodeTree` との統合

`NodeTree::flush_dirty()` が dirty な `NodeId` セットを返したとき、フレームループは:
1. 各ノードの計算済みレイアウト rect（レイアウトパスの出力）を参照する。
2. 各ノードについて `dirty_tracker.mark_dirty_rect(layout_rect)` を呼び出す。

これにより `DirtyTracker` とノードツリーの変更検知が同期される。  
注記: この接続は issue #38 で追跡している前提作業である。

### 17.5 メトリクス: `P2_GPU_SKIP_RATIO`

| キー | 定義 | CI 閾値 |
|---|---|---|
| `P2_GPU_SKIP_RATIO` | `skipped_frames / total_frames`（直近60フレームのローリング平均） | 完全静止シーンで >= 0.50 |
| `P2_GPU_SUBMISSIONS` | 60フレーム単位の GPU サブミット回数 | 完全静止シーンで = 0 |

出力先: `.ci/p2-metrics.txt`（P0/P1 と同じ `KEY=VALUE` 形式）。

### 17.6 凍結ルール
- `DirtyDecision` のバリアントは v0.1 で凍結。バリアント追加は破壊的変更。
- `mark_dirty_rect` と `commit_dirty` のシグネチャは v0.1 で凍結。
- `RenderOutcome` のバリアントは v0.1 で凍結。
- `render_partial` の scissor サポートはバックエンド固有。非サポートのバックエンドは
  全画面フォールバックし、その旨を文書化しなければならない。
