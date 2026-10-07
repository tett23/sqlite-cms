---
title: "sqlite-cms の使い方"
slug: getting-started
date: 2026-09-26
category: howto
description: 記事リポジトリを作ってから、記事を書き、手元で確かめ、Cloudflare や GitHub Pages に公開するまでの手順。
tags: [sqlite-cms, 使い方]
---

sqlite-cms は、Markdown で書いた記事から個人サイトを組み立てるコマンドです。
使うのは `sqlite-cms` コマンドだけで、Node や TypeScript は要りません。

## インストールする

[Releases](https://github.com/tett23/sqlite-cms/releases) に、macOS（Apple Silicon）と Linux（x86_64）のバイナリがあります。
macOS なら次のとおりです。

```sh
curl -fsSL https://github.com/tett23/sqlite-cms/releases/latest/download/sqlite-cms-aarch64-apple-darwin.tar.gz | tar -xz
mv sqlite-cms ~/.local/bin/
```

:::message
macOS 用のバイナリは Apple の署名を受けていません。
ブラウザで取ってきたときは、`xattr -d com.apple.quarantine sqlite-cms` で隔離の属性を外してから使います。
curl や gh で取ってきたときは要りません。
:::

## 記事リポジトリを作る

```sh
sqlite-cms init my-blog --title "私の記事置き場"
cd my-blog
```

次のファイルができます。

```
site.toml          サイトの設定
content/
  index.md         トップページの本文
  favicon.svg      ファビコン（サイト名の頭文字を描いた仮のもの）
  posts/           ブログ的な軽い記事
  articles/        長めの読み物
  pages/about.md   自己紹介
  media/           画像など
.env.example       公開に使う認証情報の見本
.gitignore         .env などを Git に入れないための設定
```

サイト名やページの説明（meta description）は `site.toml` で変えられます。

## 記事を書く

```sh
sqlite-cms new post first-post --title "最初の記事"
```

`content/posts/<UUIDv7>-first-post.md` に雛形ができるので、本文を書き足します。
先頭の UUIDv7 は作った時刻から作る名前で、記事の URL（`/posts/<UUIDv7>`）になります。同じ日付の記事も、作った順に並びます。
日付は今日が入ります。タイムゾーンは `site.toml` の `timezone` で決められます。

日々のメモなら、名前を考えずに作れます。

```sh
sqlite-cms new post
```

名前を省くと、ファイル名は `content/posts/<UUIDv7>.md` に、タイトルは今日の日付になります。

長めの読み物（article）は、タイトルを付けて作ります。ファイル名にタイトルが入ります。

```sh
sqlite-cms new article --title "長い読み物"
sqlite-cms new article long-read --title "長い読み物"
```

名前（`long-read`）を付けると、frontmatter に `slug: long-read` が入り、URL が `/articles/long-read` になります。
付けなければ、URL は UUIDv7 です。

| 種別 | 置き場所 | URL | 向いているもの |
|---|---|---|---|
| `post` | `content/posts/` | `/posts/<UUIDv7>` | 日々のメモ |
| `article` | `content/articles/` | `/articles/<slug>`（slug がなければ `/articles/<UUIDv7>`） | 要約付きの読み物 |
| `page` | `content/pages/` | `/about` のみ | 自己紹介などの固定ページ |

## 手元で確かめる

```sh
sqlite-cms serve
```

`http://127.0.0.1:8080/` で表示を確かめられます。
止めるときは <kbd>Ctrl</kbd> + <kbd>C</kbd> です。
記事を書き換えると、自動で組み立て直して、ブラウザも読み込み直します。

## 公開する

`site.toml` に公開先の Worker 名を書きます。

```toml
[deploy]
worker = "my-blog"
```

Cloudflare の API トークンとアカウント ID を `.env` に書いて、公開します。
トークンに要る権限は、アカウントの「Workers スクリプト：編集」だけです。

```sh
cp .env.example .env   # CLOUDFLARE_API_TOKEN と CLOUDFLARE_ACCOUNT_ID を書く
sqlite-cms deploy
```

公開が終わると、`https://<Worker 名>.<サブドメイン>.workers.dev` の URL が表示されます。
`.env` は `.gitignore` に書いてあるので、Git には入りません。

:::details Cloudflare 以外に公開する
`site.toml` の `[deploy]` に `target` を書くと、GitHub Pages か、rsync で送る任意のサーバーに公開できます。

```toml
[deploy]
target = "github-pages"   # 記事リポジトリの gh-pages ブランチに push する
```

```toml
[deploy]
target = "rsync"
destination = "user@example.com:/var/www/blog/"
```

`https://<user>.github.io/my-blog/` のように、ドメインの直下でない場所に置くときは、`site.toml` の先頭に `base_path = "/my-blog/"` も書きます。
:::

<details>
<summary>GitHub Actions で自動で公開する</summary>

記事リポジトリの `.github/workflows/deploy.yml` に置くと、`main` に push するたびに公開されます。
リポジトリのシークレットに `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` を設定しておきます。

```yaml
name: deploy
on:
  push:
    branches: [main]
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - run: gh release download --repo tett23/sqlite-cms --pattern 'sqlite-cms-x86_64-unknown-linux-gnu.tar.gz' --output - | tar -xz
        env:
          GH_TOKEN: ${{ github.token }}
      - run: ./sqlite-cms deploy
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
```

</details>

書ける記法は [記法の一覧](/articles/syntax) と [拡張の記法](/articles/extensions) にまとめてあります。
