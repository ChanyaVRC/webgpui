# アーキテクチャ意思決定ログ方針

## 1. 目的
重要な設計判断を記録し、将来の変更理由を追跡可能にする。

## 2. 記録対象
- crate 境界の変更
- 公開 API の互換性方針変更
- CI ゲート閾値の大幅変更
- Compat/FastPath の採用範囲変更

## 3. 最小テンプレート
```markdown
# ADR-YYYYMMDD-<title>

## Context
## Decision
## Alternatives
## Consequences
## Metrics Impact
```

## 4. 運用ルール
- 破壊的変更は ADR を必須化
- 関連PRと関連メトリクスを本文に添付
- 変更後に docs の対応ページも更新する
