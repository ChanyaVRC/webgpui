# webgpui ドキュメント

このサイトは、webgpui の設計思想・要件・高速化計画・移行戦略をまとめたものです。

## クイックリンク
- [設計思想](design-philosophy.md)
- [要件サマリー](requirements-summary.md)
- [要件定義（詳細）](requirements.md)
- [ロードマップ](roadmap.md)
- [構成案（crate分割）](workspace-architecture.md)
- [レンダリング高速化計画](rendering-performance-plan.md)
- [APIマッピング](api-mapping.md)
- [APIスワップ品質保証](api-swapping-quality-plan.md)
- [CIゲート運用](ci-gates.md)
- [メトリクス仕様](metrics-format.md)
- [ドキュメントマップ](documentation-map.md)

## 運用メモ
- 速度最優先で進めるため、P0/P1ゲートをPRで必須化する。
- API互換は `MUST` 行を凍結し、同等性テストとセットで管理する。
