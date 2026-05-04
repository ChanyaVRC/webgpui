# GitHub Pages Deployment

## 1. Prerequisites
- `mkdocs.yml` exists
- GitHub Actions enabled
- `.github/workflows/docs-pages.yml` exists

## 2. Repository Settings
1. Open repository settings
2. Go to Pages
3. Set Build and deployment source to `GitHub Actions`

## 3. First Deployment
- Push to `main`
- `docs-pages` workflow builds and deploys
- Pages URL is generated automatically

## 4. Required Config
Update in `mkdocs.yml`:
- `site_url`
- `repo_url`
- `repo_name`

## 5. Operations
- Merge documentation updates into `main`
- Redeploy is triggered automatically
- On failure, inspect logs in Actions `docs-pages` workflow

## 6. Common Failure Cases
- Pages source is not set to `GitHub Actions`
- YAML syntax errors in `mkdocs.yml`
- Broken Markdown links (fails with `--strict`)
