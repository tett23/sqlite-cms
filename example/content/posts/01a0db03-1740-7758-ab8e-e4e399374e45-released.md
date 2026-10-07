---
title: "sqlite-cms を公開した"
date: 2026-09-26
tags: [日記, sqlite-cms]
---

このサイトを組み立てている sqlite-cms を公開した。
Markdown を書いて `sqlite-cms deploy` すると、Cloudflare に個人サイトができる。

- 記事は全部で一つの SQLite にまとまる。一度読み込んだら、ページを移っても記事を取りに行かない
- 使うのは Rust で書いた `sqlite-cms` コマンドだけで、Node は要らない
- Zenn 風の記法（メッセージ、折りたたみ、数式、図、リンクカード）が書ける
- 見た目は古いウェブサイト風。Lighthouse のユーザー補助とおすすめの方法は 100 点

:::details 手元で試すなら
```sh
curl -fsSL https://github.com/tett23/sqlite-cms/releases/latest/download/sqlite-cms-aarch64-apple-darwin.tar.gz | tar -xz
./sqlite-cms init my-blog
./sqlite-cms serve my-blog
```

詳しくは [sqlite-cms の使い方](/articles/getting-started) に書いた。
:::

https://github.com/tett23/sqlite-cms
