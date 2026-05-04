# CIゲート運用ガイド（P0 / P1）

## 1. 目的
レンダリング最適化の進捗を、PR時点で自動判定する。

- P0: FastPathの最低性能を保証
- P1: バッチング効果を数値で保証

## 2. 対象ワークフロー
- P0: `.github/workflows/p0-gate.yml`
- P1: `.github/workflows/p1-gate.yml`

## 3. 実行タイミング
- `pull_request`
- `workflow_dispatch`

## 4. 入力メトリクス
### 4.1 P0
- メトリクスファイル: `.ci/p0-metrics.txt`
- 閾値ファイル: `.ci/p0-thresholds.env`
- 判定スクリプト: `scripts/ci/check_p0_gate.sh`

### 4.2 P1
- メトリクスファイル: `.ci/p1-metrics.txt`
- 閾値ファイル: `.ci/p1-thresholds.env`
- 判定スクリプト: `scripts/ci/check_p1_gate.sh`

## 5. メトリクス生成方法
### 5.1 デフォルト
ワークフローは以下を実行する。

- P0: `cargo run -p demo-basic -- --benchmark p0 --output .ci/p0-metrics.txt`
- P1: `cargo run -p demo-basic -- --benchmark p1 --output .ci/p1-metrics.txt`

### 5.2 カスタムコマンド
環境変数で上書き可能。

- P0: `P0_METRICS_COMMAND`
- P1: `P1_METRICS_COMMAND`

例:
```bash
P1_METRICS_COMMAND='cargo run -p demo-basic --release -- --benchmark p1 --output .ci/p1-metrics.txt'
```

## 6. ローカルでの事前確認
```bash
scripts/ci/check_p0_gate.sh .ci/p0-metrics.txt .ci/p0-thresholds.env
scripts/ci/check_p1_gate.sh .ci/p1-metrics.txt .ci/p1-thresholds.env
```

## 7. 失敗時の一次対応
1. メトリクスキー欠落を確認
2. 閾値が厳しすぎないか確認
3. 直近変更で draw call / submit call / CPU build が増えていないか確認
4. 必要ならシーン固定条件（要素数、解像度、フレーム数）を統一して再測定

## 8. 運用ルール
- 閾値変更は必ずPRで理由を明記する
- 閾値を緩和する場合は、改善計画と期限を同時に記載する
- ベースライン更新は main の安定コミットを基準にする

## 9. マイルストーン完了判定ゲート（M0-M4）
ロードマップの完了条件をCIで判定するため、マイルストーン専用ゲートを利用する。

- ワークフロー: `.github/workflows/milestone-gate.yml`
- 判定スクリプト: `scripts/ci/check_milestone_gate.sh`
- 実行方法: `workflow_dispatch`（手動実行）

### 9.1 マイルストーン別チェック項目
| マイルストーン | 必須CIチェック |
| --- | --- |
| M0 | `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace --all-targets` |
| M1 | M0 + `cargo test -p webgpui-input --all-targets`, `cargo test -p webgpui-platform-winit --all-targets` |
| M2 | M1 + `cargo test -p webgpui-layout --all-targets`, `cargo test -p webgpui-core --all-targets` |
| M3 | M2 + `cargo test -p webgpui-app --all-targets`, `cargo build -p demo-basic` |
| M4 | M3 + P0/P1 ベンチマークメトリクス生成とゲート評価 |

### 9.2 ローカル実行
```bash
scripts/ci/check_milestone_gate.sh M1
```

## 10. M1 移行ノート

M1 で導入した `webgpui-input` の挙動変更:

| API | 旧挙動 | 新挙動 |
| --- | --- | --- |
| 空 `path` での `dispatch(path, ...)` | パニック（`assert!`） | ノーオペレーション（即座に return） |
| `FocusManager::set_focusable_order(order)` | `order` をそのまま格納 | 重複を除去（先出し優先）し、フォーカス中ノードが新リストに含まれない場合はフォーカスをクリア |

M1 で追加した新 API（既存の呼び出し箇所に破壊的変更なし）:
- `EventPhase` 列挙型（`Capture`、`Target`、`Bubble`）
- `dispatch(path, event, visitor)` フリー関数
- `FocusManager::register_focusable` / `unregister_focusable` / `set_focusable_order` / `focusable_order`
- `FocusManager::move_focus_forward` / `move_focus_backward` / `handle_key`

## 11. M2 移行ノート

M2 で追加した `webgpui-layout` の変更（既存の呼び出し箇所に破壊的変更なし）:

| 変更点 | 詳細 |
| --- | --- |
| `LayoutStyle::direction` | 新フィールド（デフォルト `Direction::Column` — 既存の縦積み挙動を維持） |
| `LayoutStyle::flex_grow` | 新フィールド（デフォルト `0.0` — 指定しないノードは従来と同じ） |
| `LayoutNode::text` / `::font_size` | 新フィールド（デフォルト空文字列 / `14.0` — テキストなしノードに影響なし） |
| `LayoutEngine::compute` | 内部で `DefaultTextMeasure` を使用; `text` を持たないノードの結果は不変 |
| `LayoutEngine::compute_with` | `&dyn TextMeasure` を受け取る新メソッド（カスタムフォントバックエンド注入用） |

M2 で追加した新 API:
- `Direction` 列挙型（`Column`、`Row`）
- `TextMeasure` トレイト（`measure(text, font_size, max_width) -> Size`）
- `DefaultTextMeasure` 構造体（ピクセルフォント基準、`FONT_W=5 FONT_H=7`）
- `LayoutNode::text`、`LayoutNode::font_size`
- `LayoutEngine::compute_with`
