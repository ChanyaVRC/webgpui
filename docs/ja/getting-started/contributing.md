# 開発ガイド

バージョン管理ルールは [セマンティックバージョニングポリシー](../architecture/semver-policy.md) を参照してください。

## 1. ブランチ運用
- 機能追加は feature ブランチで実施
- PR には目的、変更点、計測結果を記載
- P0/P1 関連変更は CI ゲート通過を必須化

## 2. 変更時の必須更新
- 要件変更: `requirements.md` と `docs/requirements-summary.md`
- API 変更: `api-mapping.md` と `api-swapping-quality-plan.md`
- 閾値変更: `.ci/*-thresholds.env` と `rendering-performance-plan.md`

## 3. ローカルチェック
```bash
uvx --with mkdocs-material mkdocs build --strict
scripts/ci/check_p0_gate.sh .ci/p0-metrics.txt .ci/p0-thresholds.env
scripts/ci/check_p1_gate.sh .ci/p1-metrics.txt .ci/p1-thresholds.env
```

## 4. PR テンプレートに含める項目
- 何を速くしたか（対象経路: Compat/FastPath）
- どの指標が改善したか（avg/p95/draw calls）
- 同等性テストに影響があるか
- 閾値変更の有無と根拠
