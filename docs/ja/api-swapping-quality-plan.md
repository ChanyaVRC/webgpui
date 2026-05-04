# APIスワップ品質保証計画（Compat <-> FastPath）

## 1. 目的
`webgpui-compat` から `webgpui::FastPath` へ切り替えた場合でも、機能・表示・入力の挙動が同等であることをテストで保証する。

## 2. テスト対象
- 対象経路A: Compat (`RenderMode::Compat`)
- 対象経路B: FastPath (`RenderMode::FastPath`)
- 比較単位: 画面シナリオ、フレーム、イベント列、内部状態

## 3. テストマトリクス
| 区分 | テスト名 | 比較内容 | 判定基準 |
|---|---|---|---|
| 表示 | visual_snapshot_equivalence | 同一フレーム画像 | 差分ピクセル率 <= 0.5% |
| 入力 | event_trace_equivalence | 発火順序/ペイロード | 100%一致 |
| 状態 | state_tree_equivalence | ノード構造/dirty rect | 100%一致 |
| API | must_api_contract_equivalence | MUST API の戻り値/副作用 | 100%一致 |
| 性能 | perf_fastpath_advantage | frame time / draw call | Compat比 10%以上改善 |
| 回復 | fallback_consistency | FastPath -> Compat 切替後 | 機能劣化なし |

## 4. シナリオセット（最低限）
1. Basic Shapes
- container + rectangle + opacity
- resize を 3 回

2. Interactive Panel
- hover / click / key input
- focus 移動を含む

3. Dynamic List
- append/remove/update を連続実行
- dirty rect の多発ケース

4. Stress Batch
- 同一スタイル要素を大量配置
- draw call 削減効果を確認

## 5. 失敗時の切り分け手順
1. まず state 差分を確認
- ノード構造差分
- style 差分

2. 次に event 差分を確認
- 発火順序
- stopPropagation / preventDefault の挙動

3. 最後に visual 差分を確認
- 差分ヒートマップ
- 影響領域（bounding box）

## 6. CI運用ルール
- PRごとに Compat/FastPath 両経路を実行
- どちらか一方が fail ならマージ不可
- 性能ゲートはベンチ専用ランナーで判定
- 比較対象のベースラインは main ブランチ最新を使用

## 7. 記録フォーマット
各テスト実行で以下を保存する。

- git sha
- render mode
- scene 名
- average / p95 frame time
- draw call 数
- snapshot 差分率
- event 差分件数

## 8. 完了条件
- MUST API すべてに同等性テストが存在
- 主要 3 シナリオで Compat/FastPath 同等性が成立
- 2 週間連続で回帰ゼロ
