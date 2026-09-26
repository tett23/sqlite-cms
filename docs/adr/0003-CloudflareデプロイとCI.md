# 0003 Cloudflare デプロイと CI

## 状態

採用。

## 決定

Cloudflare Workers の静的アセット配信（assets のみ、Worker スクリプトなし）にデプロイする。

### wrangler 設定

`modules/blog/wrangler.jsonc` に置く。

- `assets.directory` は `./dist`。
- `assets.not_found_handling` は `single-page-application`。BrowserRouter のフォールバックをこれで賄う。

### キャッシュ制御

`public/_headers` で与える。

```
/db/*.sqlite
  Cache-Control: public, max-age=31536000, immutable
/db/manifest.json
  Cache-Control: no-cache
```

### CI

GitHub Actions（`.github/workflows/deploy-blog.yml`）。

- `main` への push のうち `modules/blog/**` に変更があるものを契機とする。
- 手順は install → test → build（DB 生成を含む）→ `wrangler deploy`。
- 認証はリポジトリシークレット `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID`。

## 実装しないこと

- プレビュー環境（ブランチごとのデプロイ）。
- 独自ドメインの設定。workers.dev のサブドメインで始め、必要になったら Cloudflare 側で付ける。

## テスト設計

- CI のデプロイ前に vitest と `tsc --noEmit` を必須で通す。
- デプロイ自体の自動検証は持たない。デプロイ後の確認は目視とする。

## トレードオフ

- Cloudflare Pages ではなく Workers 静的アセットを選ぶ。機能は同等だが、Pages は新規機能の追加が止まっており、Cloudflare 自身が Workers への集約を案内しているためである。
- シークレットが未設定の間、CI のデプロイは失敗する。ローカルからの `wrangler deploy` は `wrangler login` 済みなら動く。
