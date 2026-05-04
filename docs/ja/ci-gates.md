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
