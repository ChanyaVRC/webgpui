# GitHub Pages 公開手順

## 1. 前提
- リポジトリに `mkdocs.yml` が存在する
- GitHub Actions が有効
- ワークフロー `.github/workflows/docs-pages.yml` が存在する

## 2. リポジトリ設定
1. GitHub の対象リポジトリを開く
2. Settings -> Pages を開く
3. Build and deployment の Source を `GitHub Actions` に設定

## 3. 初回デプロイ
- `main` ブランチへ push すると `docs-pages` ワークフローが起動
- 成功後、Pages URL が払い出される

## 4. URL設定
`mkdocs.yml` の以下を実リポジトリ値へ更新する。

- `site_url`
- `repo_url`
- `repo_name`

## 5. 運用
- ドキュメント更新後に `main` へマージ
- 自動で再デプロイ
- 失敗時は Actions の `docs-pages` ログを確認

## 6. よくある失敗
- Source が `GitHub Actions` でない
- `mkdocs.yml` のYAML構文エラー
- Markdownリンク切れ（`--strict` で失敗）
