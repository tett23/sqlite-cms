# 0022 favicon と meta description

## 状態

採用。

## 決定

### favicon

- 記事リポジトリの `content/favicon.svg` を、`/favicon.svg` として配信する。SPA の `index.html` は `<link rel="icon" type="image/svg+xml" href="/favicon.svg">` で参照する。
- `init` は、仮の favicon を `content/favicon.svg` に作る。黒い角丸の四角に、サイト名の最初の一文字（英字は大文字）を白で描いた SVG である。書き手はこれを好きな SVG に置き換える。
- `content/favicon.svg` がなければ、`build`、`serve`、`deploy` が同じ仮の favicon をその場で作って配信する。`init` より前に作った記事リポジトリでも、参照先が 404 にならないようにするためである。
- `init --force` は、`init` が作るほかのファイルと同じく `content/favicon.svg` も作り直す。
- 開発用の `build --data-only` も favicon を書き出す（Vite の開発サーバで表示するため）。

favicon がなかったとき、ブラウザが favicon を取りに行って 404 になり、Lighthouse のベストプラクティスで「コンソールにエラーがある」として減点されていた。

SVG の favicon だけを用意し、`favicon.ico` や `apple-touch-icon` は作らない。

### meta description

- `site.toml` に `description` を足す。省略したときは「`<title>`。記事とブログを置いているサイトです。」とする。どのサイトにも記事とブログの欄があるので、どのサイト名に対しても内容として正しい文言にした。空文字列はエラーにする。
- 既定の文言はビルダーが組み立て、`site` テーブルの `description` 列（マイグレーション 0005）に入れる。SPA は読むだけにする。
- SPA は、ページを表示するたびに `<meta name="description">` を設定する。frontmatter に `description`（要約）を持つ article のページでは、記事の要約を使う。それ以外のページはサイトの説明を使う。
- `init` が作る `site.toml` に、`description` の例をコメントで入れる。

SEO は重視しないが（仕様の目的）、meta description がないことで Lighthouse の SEO が全ページで 90〜91 点だった。

## 実装しないこと

- `favicon.ico`、`apple-touch-icon`、Web アプリのマニフェスト。
- OGP（`og:description` など）。SNS のクローラは JavaScript を実行しないので、ビルド時に HTML へ書き込む仕組みが要る。必要になったら別に決める。
- post と page ごとの説明（frontmatter に説明の項目がない）。

## テスト設計

- favicon：頭文字の描き方（英字の大文字化、日本語、先頭の空白、`<` と `&` のエスケープ、空のサイト名）。`init` が作ること。`content/favicon.svg` があればそれを、なければ仮のものを配信すること。書き出しに入ること。`serve` が `image/svg+xml` で返すこと。`index.html` が参照していること。
- meta description：`site.toml` の読み込み（指定、省略時の既定の文言、空文字列のエラー）。DB に既定の文言が入ること。`getSite` が説明を返すこと。ページの説明とサイトの説明の選び方。
- ブラウザで、favicon が表示され、トップ、article、自己紹介で meta description が切り替わり、要素が重複しないことを確認した。

変更後の Lighthouse は、ベストプラクティスと SEO が全ページで 100 点になった。

## トレードオフ

- meta description は JavaScript で設定するので、JavaScript を実行しないクローラには届かない。SEO を重視しない方針なので受け入れる。
- 仮の favicon のフォントは、表示する環境の sans-serif に任せる。
