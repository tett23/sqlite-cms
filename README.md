# sqlite-cms

[![ci](https://github.com/tett23/sqlite-cms/actions/workflows/ci.yml/badge.svg)](https://github.com/tett23/sqlite-cms/actions/workflows/ci.yml)

Markdown で書いた記事から、軽い個人サイトを作って Cloudflare に公開するツール。
記事は全部まとめて一つの SQLite に入り、ブラウザはそれを一度読み込むだけで、あとはページを移るたびにサーバへ記事を取りに行かない（画像は除く）。
見た目は白背景に黒文字、青い下線のリンクだけの、古いウェブサイトのようなものになる。

使うのは `sqlite-cms` コマンドだけ。記事は自分のリポジトリ（以下、記事リポジトリ）で管理する。

## インストール

[Releases](https://github.com/tett23/sqlite-cms/releases) から自分の環境のバイナリ（`sqlite-cms-<target>.tar.gz`）を取ってきて展開し、PATH の通った場所に置く。

```sh
tar -xzf sqlite-cms-aarch64-apple-darwin.tar.gz
mv sqlite-cms ~/.local/bin/
```

ソースからビルドする方法は `DEVELOPMENT.md` にある。

## はじめる

```sh
sqlite-cms init my-blog --title "私の記事置き場"   # my-blog/ に記事リポジトリを作る
cd my-blog
sqlite-cms new post hello --title はじめまして     # 最初の記事の雛形
sqlite-cms serve                                  # http://127.0.0.1:8080/ で確認
```

`init` は、ビルドに必要な `site.toml` と `content/`（トップページ、自己紹介のページ、記事用の空のディレクトリ）を作る。
すでに `site.toml` か `content/` があるときはエラーになる。
`--force` を付けると作り直す。このとき `init` が作るファイル（`site.toml`、`content/index.md`、`content/pages/about.md`）は上書きされるが、書いた記事や画像は消えない。

## 記事リポジトリの構成

```
site.toml          -- サイトの設定（必須）
content/
  index.md         -- トップページの本文（任意。frontmatter なしの Markdown）
  posts/           -- ブログ的な軽い記事（/posts/:slug）
  articles/        -- 長めの読み物（/articles/:slug）
  pages/           -- 固定ページ。about.md が自己紹介（/about）
  media/           -- 画像など（任意。/media/ で配信）
```

このリポジトリの `example/` が、記事や画像を入れた同じ構成のサンプルになっている。

### site.toml

```toml
title = "tett23の記事置き場"   # 必須。ヘッダとページタイトル
author = "tett23"              # 任意。フッターに表示
timezone = "Asia/Tokyo"        # 任意。new が入れる日付のタイムゾーン（"+09:00" の形も可）。省略すると環境のタイムゾーン

[license]                      # 任意。フッターに表示
name = "CC0 1.0"               # [license] を書くなら必須
url = "https://creativecommons.org/publicdomain/zero/1.0/"  # 任意。書けばリンクになる

[deploy]                       # sqlite-cms deploy を使うなら必須
worker = "my-blog"             # 公開先の Cloudflare Worker 名（英小文字、数字、ハイフン）
```

知らないキーはエラーになる（綴りの誤りを見逃さないため）。

### 記事

Markdown のファイル名が URL の一部（slug）になる。

```markdown
---
title: 記事タイトル
date: 2026-09-17
description: 一覧に出す要約（articles/ のみ、任意）
updated: 2026-09-18
---

本文。
```

- `posts/` の frontmatter は `title` と `date`。
- `articles/` は加えて `description`（一覧用の要約）と `updated`（改稿日）を任意で持つ。
- `pages/` の frontmatter は `title` だけ。

frontmatter は YAML のうち、1 行に 1 つの `キー: 値` だけを書ける。
値はそのまま書くか、`"..."`（`\"` や `\n` のエスケープが使える）か `'...'` で囲む。
複数行の値（`|` や `>`）、リスト、入れ子は使えず、書くと何行目が問題かを示すエラーになる。
値が `[` や `{` などの記号で始まるときは、クォートで囲む。

記法は CommonMark と GFM（表、取り消し線、タスクリスト、自動リンク、脚注）。

Markdown では書けない表現のために、次の HTML のタグだけを本文に書ける。

| タグ | 用途 |
|---|---|
| `<details>`、`<summary>` | 折りたたみ（`<details open>` で最初から開く） |
| `<kbd>` | キー操作（<kbd>Ctrl</kbd> + <kbd>C</kbd>） |
| `<sub>`、`<sup>` | 下付き、上付き（H<sub>2</sub>O、x<sup>2</sup>） |
| `<ruby>`、`<rt>`、`<rp>` | ルビ |
| `<mark>` | 蛍光ペンのような強調 |
| `<abbr title="...">` | 略語と正式名 |
| `<dl>`、`<dt>`、`<dd>` | 定義リスト |
| `<br>` | 表のセルの中の改行 |

これ以外のタグ（`div`、`span` など）はタグだけが外れて中身の文字が残る。`<script>`、`<iframe>`、`<style>` とコメントは中身ごと表示されない。
属性は `details` の `open` と `abbr` の `title` などに限られ、`class`、`style`、`id`、`onclick` などは取り除かれる。

`<details>` の中で Markdown を使うときは、`<summary>` の後と `</details>` の前に空行を入れる。
空行がないと、Markdown の記法がそのまま文字として出る。
`<summary>` の中は HTML なので、強調やコードには `<strong>` や `<code>` を使う。

```markdown
<details>
<summary>補足</summary>

ここは **Markdown** で書ける。

</details>
```

コードブロックに言語名を書くと色が付く（Shiki による）。
対応する言語名は次のとおり。これ以外の言語名と、言語名のないブロックは色なしで表示される。

| 言語 | 書ける言語名 |
|---|---|
| CSS | `css` |
| diff | `diff` |
| Haskell | `haskell`、`hs` |
| HTML | `html` |
| JavaScript | `javascript`、`js`、`cjs`、`mjs` |
| JSON | `json` |
| JSX、TSX | `jsx`、`tsx` |
| Markdown | `markdown`、`md` |
| Python | `python`、`py` |
| Rust | `rust`、`rs` |
| シェル | `shellscript`、`bash`、`sh`、`shell`、`zsh` |
| SQL | `sql` |
| TOML | `toml` |
| TypeScript | `typescript`、`ts`、`cts`、`mts` |
| YAML | `yaml`、`yml` |
画像は `content/media/` に置き、`![説明](/media/foo.png)` のように参照する。

## 使い方

記事リポジトリの中で実行する（別の場所から使うときは、記事リポジトリのパスを引数に渡す）。

```sh
sqlite-cms new post hello --title はじめまして   # content/posts/hello.md の雛形を作る
sqlite-cms new post                             # content/posts/<今日の日付>.md の雛形を作る
sqlite-cms serve    # http://127.0.0.1:8080/ でプレビュー（記事を変えたら再起動）
sqlite-cms build    # dist/ に配信用の一式を書き出す
sqlite-cms deploy   # Cloudflare に公開する
```

`new` の種別は `post`、`article`、`page` のどれか。
日付は今日が入り、`--date 2026-09-26` で変えられる。
今日の日付は `site.toml` の `timezone` のタイムゾーンで決まる。省略すると、実行した環境のタイムゾーンになる（CI など、手元と違うタイムゾーンで動かすときは指定しておくとよい）。
slug（ファイル名と URL になる名前）を省くと、記事の日付（`2026-09-26` など）が slug になる。
同じ日付の記事がすでにあれば、`2026-09-26-2`、`2026-09-26-3` と枝番が付く。
page は日付を持たないので、slug を省けない。
slug を指定して、同じ名前のファイルがすでにあるときは、上書きせずにエラーになる。

slug を省いて別の場所の記事リポジトリを指すときは、`./blog` や `../blog` のようにパスとわかる形で書く（`/` を含むか `.` で始まる引数は、slug ではなく記事リポジトリとみなす）。

詳しいオプションは `sqlite-cms --help` で確認できる。

## Cloudflare への公開

1. Cloudflare の API トークンを、ダッシュボードのテンプレート「Edit Cloudflare Workers」から作る。
2. `site.toml` の `[deploy]` に公開先の Worker 名を書く。
3. 環境変数を設定して `sqlite-cms deploy` を実行する。

```sh
export CLOUDFLARE_API_TOKEN=...
export CLOUDFLARE_ACCOUNT_ID=...
sqlite-cms deploy
```

公開が終わると `https://<worker>.<サブドメイン>.workers.dev` の URL が表示される。
独自ドメインは Cloudflare のダッシュボードで設定する。

### GitHub Actions で自動公開する

`example/.github/workflows/deploy.yml` を記事リポジトリの `.github/workflows/` にコピーし、記事リポジトリのシークレットに `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` を設定する。
`main` に push するたびに、最新の `sqlite-cms` を取ってきて公開する。
