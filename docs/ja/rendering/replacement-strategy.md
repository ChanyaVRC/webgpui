# 既存WebUIエンジン代替戦略

## 1. 目的
既存 WebUI エンジンを段階的に `webgpui` へ置き換え、機能・品質・性能の劣化を避ける。

## 2. 代替方針
1. Big-bang 移行を避ける
- 画面単位または機能単位で段階移行する
- 新旧エンジンを一時共存させる

2. 互換レイヤーを先に作る
- 既存 API 呼び出しを `webgpui-compat` で受ける
- 内部で `webgpui` API へ変換する

3. 高速化は移行と同時に測定する
- 既存比で FPS、CPU、GPU を計測
- 改善が確認できる画面から切り替える

## 3. 互換対象（MVP）
- ノード: container, text(簡易), image(プレースホルダ)
- スタイル: position, size, margin, padding, background, border, opacity
- イベント: click, pointer move, key down/up
- ライフサイクル: mount/update/unmount 相当

## 4. 非互換管理
- 未対応 API は明示的に warning を出す
- 動作差がある API は「差分理由」と「代替手段」を migration note に記載
- 互換対象外は feature flag で明示する

## 5. 段階移行プロセス
1. 棚卸し
- 既存画面で使用している API とスタイルを抽出

2. マッピング
- 既存 API -> `webgpui-compat` API の対応表を作成

3. 小規模画面で PoC
- 表示一致、入力一致、性能指標を確認

4. 横展開
- 同種コンポーネントをまとめて移行
- 残件は未対応リストとして管理

5. 置き換え完了判定
- 互換率、画面再現率、速度指標が閾値を満たす

## 6. 判定KPI
- API 代替率: 80%以上
- 画面再現率: 90%以上
- 平均 FPS: 既存以上
- p95 フレーム時間: 既存以下
- 重大不具合: 0

## 7. リスクと対策
- リスク: スタイル互換の差で見た目が崩れる
- 対策: 視覚回帰テストを導入し、主要画面のスクリーンショット比較を行う

- リスク: イベント伝播の差で操作感が変わる
- 対策: capture/bubble の順序をテストで固定し、互換層で吸収する

- リスク: 一部画面で性能改善が出ない
- 対策: profiler でボトルネックを特定し、画面別に最適化対象を分離する

## 8. 直近アクション
- `webgpui-compat` の最小 API セットを定義（→ api-mapping.md §13 で完了）
- `apps/demo-migration` で代表画面を移植
- 既存/新規の比較ベンチを作成

## 9. 確定ドキュメント
- API マッピング表（確定版 v0.1）: `api-mapping.md`

## 10. M4 実行計画

### 10.1 `apps/demo-migration` 構成
```
apps/demo-migration/
  Cargo.toml
  src/
    main.rs          — アプリエントリポイント；CLI 引数でシーン選択
    scenes/
      mod.rs
      screen_a.rs    — 画面A: container + text + event（最小構成）
      screen_b.rs    — 画面B: list + 動的更新 + キーボードナビゲーション
    metrics.rs       — 移行コスト記録（行数、非対応API数）
    compare.rs       — レガシーと新実装を並走させフレーム時間差分を出力
```
`screen_a`・`screen_b` ともに `webgpui-compat` のみに依存する；`webgpui-core` への直接呼び出し禁止。

### 10.2 ビジュアルリグレッションテスト
ツール: `insta`（スナップショットテスト）またはカスタム PNG diff ハーネス。
- 各シーンをオフラインで N フレーム描画し、最初の安定フレームをリファレンススナップショットとして保存。
- CI 上で同シーンを再描画しリファレンスと差分比較（ピクセル差分閾値: <= 1%）。
- スナップショットは `apps/demo-migration/snapshots/` に保存。
- 既知の許容差分（フォントアンチエイリアスなど）を `KNOWN_DIFFS.md` に文書化。

### 10.3 比較ベンチマーク
`--benchmark compare` フラグで実行:
1. レガシーエンジンで画面A・Bを300フレーム描画；avg/p95/draw-calls を記録。
2. `webgpui-compat` で同シーンを300フレーム描画；同じメトリクスを記録。
3. 結果を Markdown テーブルとして標準出力と `migration-report.md` に出力。

受入基準: 新エンジンの avg フレーム時間 <= レガシー、p95 <= レガシー、draw-calls <= レガシー。

### 10.4 移行コスト計測
`metrics.rs` で追跡:
- アプリコードの変更行数（PRの説明欄に手動記入）
- 変換した MUST ティア API 呼び出し箇所数
- 残存 `UNIMPLEMENTED` スタブ数（非対応 API 数）

これらの値は M4 完了 PR の説明に記載する。

### 10.5 同等性テストシナリオ（api-swapping-quality-plan.md §4 参照）
| シナリオ | カバレッジ |
|---|---|
| Basic Shapes | container + rect + opacity + リサイズ3回 |
| Interactive Panel | hover / click / key + フォーカス移動 |
| Dynamic List | append/remove/update + 高頻度 dirty-rect |
| Stress Batch | 同一スタイル大量要素 + draw call 削減の検証 |
