# sqlite-cms

個人の記事置き場。記事の Markdown を Git で管理し、ビルド時に単一の SQLite へ格納する。
ブラウザは SQLite を sql.js で開き、本文を remark（GFM 対応）で描画する。
仕様は `docs/specifications.md`、設計決定は `docs/adr/` を参照。

## 使い方

Node と Rust（cargo）が要る。

```sh
npm install
npm run dev      # DB 生成 + 開発サーバ
npm test         # cargo test + vitest
npm run build    # DB 生成 + 型検査 + 本番ビルド (dist/)
npm run deploy   # build + wrangler deploy（要 wrangler login）
```

## ビルダー

Markdown を検証して SQLite に格納する処理は、Rust のバイナリ `build-db`（`build-db/`）が行う。
本文は Markdown のまま格納し、HTML への変換はしない。
`npm run build:db` が `cargo run --release` 経由で実行する。
単体で使うときは、リポジトリルートを引数に渡す。

```sh
cargo build --release --manifest-path build-db/Cargo.toml
build-db/target/release/build-db .
```

## コンテンツ

`content/` 以下に Markdown を置く。ファイル名が slug になる。
記法は CommonMark と GFM（表、取り消し線、タスクリスト、自動リンク、脚注）。本文中の生の HTML は表示されない。

- `content/posts/` … ブログ的な軽量の記事（`/posts/:slug`）。frontmatter は `title` と `date`。
- `content/articles/` … 長めの読み物（`/articles/:slug`）。`description`（一覧用の要約）と `updated`（改稿日）を任意で持つ。
- `content/pages/` … 固定ページ。`about.md` が自己紹介（`/about`）。frontmatter は `title` のみ。

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
ビルドのたびに空 DB へファイル名昇順に全件適用され、適用記録は `schema_migrations` に入る。
スキーマを変えるときは既存ファイルを編集せず、次の連番を追加する。

## デプロイ

- 初回はローカルで `npx wrangler login` してから `npm run deploy`。
- CI（`.github/workflows/deploy.yml`）はリポジトリシークレット `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` の設定後、`main` への push で動く。
