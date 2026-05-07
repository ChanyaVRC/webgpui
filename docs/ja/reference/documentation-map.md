# ドキュメントマップ

## 1. 使い分け
- 全体方針を知りたい: design-philosophy.md
- 要件を素早く確認したい: requirements-summary.md
- CI 運用を確認したい: ci-gates.md
- メトリクス形式を確認したい: metrics-format.md

## 2. 詳細仕様（日本語）
- requirements.md: 機能/非機能要件、受け入れ条件
- workspace-architecture.md: crate 分割と依存方針
- rendering-performance-plan.md: 速度改善ロードマップ、P0/P1ゲート
- api-mapping.md: 既存 API から新 API への確定表
- replacement-strategy.md: 既存エンジン代替の移行戦略
- api-swapping-quality-plan.md: Compat/FastPath 同等性テスト計画

## 3. 日本語ドキュメント（docs/ja 配下）
- docs/ja/index.md
- docs/ja/github-pages.md
- docs/ja/design-philosophy.md
- docs/ja/requirements-summary.md
- docs/ja/requirements.md
- docs/ja/workspace-architecture.md
- docs/ja/rendering-performance-plan.md
- docs/ja/replacement-strategy.md
- docs/ja/api-mapping.md
- docs/ja/api-swapping-quality-plan.md
- docs/ja/ci-gates.md
- docs/ja/metrics-format.md
- docs/ja/contributing.md
- docs/ja/glossary.md
- docs/ja/architecture-decisions.md
- docs/ja/docs-coverage-review.md
- docs/ja/documentation-map.md

## 4. 英語版（docs ルート）
- docs/index.md
- docs/github-pages.md
- docs/design-philosophy.md
- docs/requirements-summary.md
- docs/requirements.md
- docs/workspace-architecture.md
- docs/rendering-performance-plan.md
- docs/replacement-strategy.md
- docs/api-mapping.md
- docs/api-swapping-quality-plan.md
- docs/ci-gates.md
- docs/metrics-format.md
- docs/contributing.md
- docs/glossary.md
- docs/architecture-decisions.md
- docs/docs-coverage-review.md
- docs/documentation-map.md

## 5. 更新ルール
- 要件変更時: requirements.md と requirements-summary.md を同時更新
- API変更時: api-mapping.md と api-swapping-quality-plan.md を同時更新
- CI閾値変更時: .ci/*-thresholds.env と rendering-performance-plan.md を同時更新

## 6. GitHub Pages 公開
- 設定ファイル: `mkdocs.yml`
- ワークフロー: `.github/workflows/docs-pages.yml`
- 英語トップページ: `docs/index.md`
- 日本語トップページ: `docs/ja/index.md`
- 公開手順書: `docs/github-pages.md`
- リポジトリ設定で Pages の Build and deployment を `GitHub Actions` に設定する
