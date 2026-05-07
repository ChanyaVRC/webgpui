# 設計思想

## 1. プロダクトの軸
本エンジンは「既存WebUIエンジンの置換可能性」と「レンダリング速度」を両立する。

- 互換性: 既存 API から段階移行できる
- 速度: FastPath と計測主導で継続改善する
- 保守性: crate 分割と責務分離を徹底する

## 2. 設計原則
### 2.1 計測先行
- 計測なしの最適化は禁止
- CPU/GPU の数値で意思決定する
- 改善は CI ゲートで継続監視する

### 2.2 段階移行
- Compat 経路を先に整備し、FastPath へ段階置換する
- 新旧エンジン共存を許容し、画面単位で移行する

### 2.3 API 安定性
- 公開 API は facade 層に集約
- 互換性は semver と migration note で管理
- MUST API は同等性テストを必須化する

### 2.4 速度最優先の実装順
- P0: 計測 + ホットパス
- P1: バッチング
- P2: 再描画抑制
- P3: 転送/キャッシュ最適化
- P4: 並列化/構造最適化

## 3. アーキテクチャ原則
- 一方向依存を維持し循環依存を禁止
- 低レベル最適化は必要領域に限定
- 共通型は geometry 層に寄せて重複を避ける

## 4. 品質保証原則
- Compat と FastPath の同等性をテストで保証
- 表示、入力、状態、性能の4軸で回帰防止する
- CI で P0/P1 ゲートを先行運用し、閾値を明文化する

## 5. 参照
- 要件定義: architecture/requirements.md
- 構成案: architecture/workspace-architecture.md
- 高速化計画: rendering/rendering-performance-plan.md
- API マッピング: rendering/api-mapping.md
- API スワップ品質: rendering/api-swapping-quality-plan.md
