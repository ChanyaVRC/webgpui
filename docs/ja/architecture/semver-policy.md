# セマンティックバージョニングポリシー

webgpui は [Semantic Versioning 2.0.0](https://semver.org/lang/ja/) に従い、
v0.x 期間中は以下のルールを適用します。

## v0.x のルール

| バージョン | 適用タイミング |
|---|---|
| **patch** (0.x.**y**) | バグ修正のみ — API の変更なし、正しい呼び出し元から見た動作変更なし |
| **minor** (0.**x**.0) | 後方互換の追加 — 既存コードを壊さない新しい型・関数・トレイト実装の追加 |
| **major** (**1**.0.0) | 最初の安定リリース用。v0.x では使用しない |

> **注意:** v0.x では、MUST 外の API に破壊的変更が伴う場合でも、
> `CHANGELOG.md` に `# Migration` セクションを設けることで **minor** バンプで
> 対応できます。MUST 層 API（`docs/ja/api-mapping.md §13.4` 参照）は、
> 追加的変更であっても minor バンプが必要です。

## MUST 層の安定保証

`docs/ja/api-mapping.md §13.4` で MUST 層と指定されている API には、
最も強い安定保証が与えられます。

- **削除・リネーム禁止** — minor バンプ と少なくとも 1 リリース期間の `#[deprecated]` アノテーションなしに行わない
- **シグネチャ変更禁止** — パラメータ型・戻り値型・エラーバリアントの変更は minor バンプが必要
- **動作変更** — 正しい呼び出し元を壊す変更は minor バンプと `CHANGELOG.md` の `### Changed` エントリが必要

## 非推奨化プロセス

1. 対象アイテムに `#[deprecated(since = "0.x.0", note = "...")]` を付与する
2. `note` 文字列と `CHANGELOG.md` に代替手段を明記する
3. 非推奨アイテムは少なくとも 1 minor リリース後に削除する
4. 削除時は minor バンプ（MUST 外であれば patch バンプ）を伴う

## 変更履歴の規律

公開 API に影響するすべての変更は、[Keep a Changelog](https://keepachangelog.com/ja/1.1.0/)
形式に従い `CHANGELOG.md` の適切な見出し
（`Added`・`Changed`・`Deprecated`・`Removed`・`Fixed`・`Security`）に記録する。

参照: [docs/ja/contributing.md](../getting-started/contributing.md)
