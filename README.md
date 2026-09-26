# sqlite-cms

個人の記事置き場のためのツール。記事の Markdown をビルド時に単一の SQLite へ格納し、Cloudflare に配信する。
ブラウザは SQLite を sql.js で開き、本文を remark（GFM 対応）で描画する。
仕様は `docs/specifications.md`、設計決定は `docs/adr/` を参照。

このリポジトリはツール（スキーマ、ビルダー、SPA、デプロイの設定）だけを持つ。
記事は別のリポジトリ（コンテンツリポジトリ）で管理する。
`example/` は動作確認用のサンプル。

## 使い方

Node と Rust（cargo）が要る。

```sh
npm install
npm run dev      # DB 生成 + 開発サーバ
npm test         # cargo test + vitest
npm run build    # DB 生成 + 型検査 + 本番ビルド (dist/)
npm run deploy   # build + wrangler deploy（要 wrangler login、SITE_DIR 必須）
```

コンテンツリポジトリの場所は環境変数 `SITE_DIR` で指定する。
未指定なら `example/` を使う（`deploy` だけは指定が必須）。

```sh
SITE_DIR=../blog-content npm run dev
SITE_DIR=../blog-content npm run deploy
```

## ビルダー

Markdown を検証して SQLite に格納する処理は、Rust のバイナリ `sqlite-cms`（`cli/`）が行う。
本文は Markdown のまま格納し、HTML への変換はしない。
マイグレーションはバイナリに埋め込まれているので、バイナリ単体でどこからでも実行できる。

```sh
cargo build --release --manifest-path cli/Cargo.toml
cli/target/release/sqlite-cms [SITE_DIR] [--public <DIR>]   # SITE_DIR の既定は .、--public の既定は public
```

記事リポジトリの中で実行するなら、引数は要らない。
`sqlite-cms --help` で、記事リポジトリの構成と書き出すものの一覧を確認できる。
npm スクリプトは `SITE_DIR` 環境変数（未指定なら `example/`）を常に引数として渡す。

DB を `<public>/db/` に、`content/media/` を `<public>/media/` に書き出す。

## コンテンツリポジトリの構成

```
site.toml          -- サイトのメタデータ（必須）
content/
  index.md         -- トップページの本文（任意。frontmatter なしの Markdown）
  posts/           -- ブログ的な軽量の記事（/posts/:slug）
  articles/        -- 長めの読み物（/articles/:slug）
  pages/           -- 固定ページ。about.md が自己紹介（/about）
  media/           -- 画像など（任意。/media/ で配信）
```

`example/` が同じ構成のサンプルになっている。

### site.toml

```toml
title = "tett23の記事置き場"   # 必須
author = "tett23"              # 任意。フッターに表示

[license]                      # 任意。フッターに表示
name = "CC0 1.0"               # [license] を書くなら必須
url = "https://creativecommons.org/publicdomain/zero/1.0/"  # 任意。書けばリンクになる
```

知らないキーはエラーになる（綴りの誤りを見逃さないため）。

### 記事

Markdown のファイル名が slug になる。
記法は CommonMark と GFM（表、取り消し線、タスクリスト、自動リンク、脚注）。本文中の生の HTML は表示されない。
画像は `content/media/` に置き、`![説明](/media/foo.png)` のように参照する。

- `posts/` の frontmatter は `title` と `date`。
- `articles/` は加えて `description`（一覧用の要約）と `updated`（改稿日）を任意で持つ。
- `pages/` の frontmatter は `title` のみ。

```markdown
---
title: 記事タイトル
date: 2026-09-17
description: 一覧に出す要約（article のみ、任意）
updated: 2026-09-18
---

本文。
```

## DB マイグレーション

スキーマは `migrations/NNNN_名前.sql` の適用列で定義する。
ファイルはビルド時にバイナリへ埋め込まれ、DB の生成のたびに空 DB へファイル名昇順に全件適用される。適用記録は `schema_migrations` に入る。
スキーマを変えるときは既存ファイルを編集せず、次の連番を追加する。

## デプロイ

ローカルからは `npx wrangler login` のあと、`SITE_DIR` を指定して `npm run deploy`。

CI（`.github/workflows/deploy.yml`）を使うには、このリポジトリに次を設定する。

| 種類 | 名前 | 内容 |
|---|---|---|
| 変数 | `CONTENT_REPOSITORY` | コンテンツリポジトリ（`owner/repo`）。未設定ならデプロイは止まる |
| シークレット | `CLOUDFLARE_API_TOKEN` | Cloudflare の API トークン |
| シークレット | `CLOUDFLARE_ACCOUNT_ID` | Cloudflare のアカウント ID |
| シークレット | `CONTENT_REPOSITORY_TOKEN` | コンテンツリポジトリが非公開の場合のみ。読み取り権限のあるトークン |

ワークフローは `main` への push、手動実行、`repository_dispatch`（種別 `content-updated`）で走る。
コンテンツの更新でデプロイしたいときは、コンテンツリポジトリの push から `content-updated` を送る。

```sh
gh api repos/<owner>/sqlite-cms/dispatches -f event_type=content-updated
```
