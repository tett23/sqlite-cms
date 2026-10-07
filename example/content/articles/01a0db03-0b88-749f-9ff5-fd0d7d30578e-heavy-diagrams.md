---
title: "重いページの見本：図の多い記事"
slug: heavy-diagrams
date: 2026-09-26
category: samples
description: 表示の重さを確かめるための見本。sqlite-cms の仕組みを、フローチャート、シーケンス図、ER 図など 10 種類の mermaid の図で描く。
tags: [見本, 計測]
---

:::message
このページは、表示の重さを確かめるための見本です。
図が多い記事で、mermaid の読み込みと図の描画が、本文の表示や入力への応答をどれだけ遅らせるかを Lighthouse で計測しています。
:::

sqlite-cms の仕組みを、いろいろな種類の図で描きます。

## 組み立ての流れ（フローチャート）

```mermaid
flowchart TD
  A[site.toml] --> C{sqlite-cms build}
  B[content/*.md] --> C
  M[content/media] --> C
  C --> D[(articles-ハッシュ.sqlite)]
  C --> E[index.html と JS]
  C --> F[media と link-cards]
  C --> G[rss.xml と sitemap.xml]
  D --> H{公開先}
  E --> H
  F --> H
  G --> H
  H -->|deploy| I[Cloudflare]
  H -->|build| J[GitHub Pages]
  H -->|build| K[rsync で任意のサーバー]
```

## ページを開いたときの通信（シーケンス図）

```mermaid
sequenceDiagram
  participant B as ブラウザ
  participant S as サーバー
  B->>S: GET /articles/heavy-diagrams
  S-->>B: index.html（読み込み中の表示と CSS）
  par 並行して取得
    B->>S: GET /assets/index.js
    B->>S: GET /db/articles-ハッシュ.sqlite
  end
  S-->>B: JS、DB
  B->>B: 自前の読み手で DB を読む
  B->>B: 本文の Markdown を描く
  Note over B: 本文が画面に出る
  B->>S: GET /assets/mermaid.js（図が画面の近くに来たら）
  S-->>B: mermaid
  B->>B: 図を描く
```

## 記事の種類（クラス図）

```mermaid
classDiagram
  class Post {
    +String slug
    +String title
    +Date published_at
    +String body_md
  }
  class Article {
    +String slug
    +String title
    +Date published_at
    +Date updated_at
    +String description
    +String body_md
  }
  class Page {
    +String slug
    +String title
    +String body_md
  }
  class Site {
    +String title
    +String author
    +String description
    +String home_md
  }
  Site "1" o-- "*" Post
  Site "1" o-- "*" Article
  Site "1" o-- "*" Page
```

## DB のテーブル（ER 図）

```mermaid
erDiagram
  posts {
    TEXT slug PK
    TEXT title
    TEXT published_at
    TEXT body_md
  }
  articles {
    TEXT slug PK
    TEXT title
    TEXT published_at
    TEXT updated_at
    TEXT description
    TEXT body_md
  }
  pages {
    TEXT slug PK
    TEXT title
    TEXT body_md
  }
  site {
    INTEGER id PK
    TEXT title
    TEXT author
    TEXT license_name
    TEXT license_url
    TEXT home_md
    TEXT description
  }
  link_cards {
    TEXT url PK
    TEXT image_path
  }
```

## ライブラリの読み込み（状態遷移図）

```mermaid
stateDiagram-v2
  [*] --> 未読み込み
  未読み込み --> 待機: 部品が画面の近くに来た
  待機 --> 読み込み中: 本文を描いた後、手が空いた
  読み込み中 --> 読み込み済み: 成功
  読み込み中 --> 未読み込み: 失敗（次に表示したときに読み直す）
  読み込み済み --> [*]
```

## 最初に読み込むもの（円グラフ）

gzip 後の大きさ（KB）の目安です。

```mermaid
pie showData
  title 最初に読み込むもの（gzip 後、KB）
  "本体の JS" : 72
  "DB" : 31
  "index.html" : 4
```

## 作業の流れ（Git のグラフ）

```mermaid
gitGraph
  commit id: "robots.txt"
  branch feat/search
  checkout feat/search
  commit id: "全文検索"
  checkout main
  branch feat/feed
  checkout feat/feed
  commit id: "統合一覧と RSS"
  checkout main
  branch integration/all
  checkout integration/all
  merge feat/search
  merge feat/feed
  checkout main
  merge integration/all
  commit id: "表示の速さの改善"
```

## 機能の一覧（マインドマップ）

```mermaid
mindmap
  root((sqlite-cms))
    書く
      Markdown
      Zenn の拡張
      数式
      図
    組み立てる
      SQLite 一つ
      RSS
      サイトマップ
      リンクカード
    公開する
      Cloudflare
      GitHub Pages
      rsync
    読む
      一度の読み込み
      全文検索
      色分け
```

## 版の歴史（年表）

```mermaid
timeline
  title sqlite-cms の版
  v0.0.1 : 最初の公開 : Lighthouse の計測
  v0.0.2 : new の slug の省略 : リリースでの計測
  v0.0.3 : Zenn の拡張 : serve の gzip : リンクカードの画像 : MIT ライセンス
```

## モジュールの関係（elk のレイアウト）

`layout: elk` を指定すると、大きな図を elk で並べます。点線は、必要になったときに読み込むもの（動的 import）です。

```mermaid
---
config:
  layout: elk
---
flowchart LR
  main[main.tsx] --> App[App.tsx]
  App --> router[router.tsx]
  App --> db[db.ts]
  App --> search[search.ts]
  App --> MarkdownBody[MarkdownBody.tsx]
  db --> sqlite[sqlite.ts]
  search --> sqlite
  MarkdownBody --> remark[remarkExtensions.ts]
  MarkdownBody --> raw[rawHtml.ts]
  MarkdownBody --> sanitize[sanitize.ts]
  MarkdownBody --> hl[highlightLoader.ts]
  MarkdownBody --> near[nearViewport.ts]
  MarkdownBody --> Diagram[Diagram.tsx]
  MarkdownBody --> Math[Math.tsx]
  hl --> lazy[lazyLoader.ts]
  hl -.-> worker[highlight.worker.ts]
  worker --> highlight[highlight.ts]
  highlight --> shiki[(Shiki)]
  Diagram --> loaders[loaders.ts]
  Math --> loaders
  loaders --> lazy
  loaders -.-> mermaidTs[mermaid.ts]
  loaders -.-> katexTs[katex.ts]
  mermaidTs --> mermaid[(mermaid)]
  katexTs --> katex[(KaTeX)]
  remark --> transforms[transforms.ts]
  remark --> container[container.ts]
  remark --> fence[fence.ts]
  remark --> mathSyntax[math.ts]
```

## 書き誤りのある図

mermaid が読めない図は、図の文字列のまま表示します。

```mermaid
flowchart TD
  A --> 
```
