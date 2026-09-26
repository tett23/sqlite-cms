# 0037 robots.txt とサイトマップ

## 状態

採用。

## 決定

### robots.txt

サイトに `/robots.txt` を置き、クロールを許可する。

- 記事リポジトリの `content/robots.txt` があれば、それを `/robots.txt` として配信する。なければ、既定の内容を配信する。favicon（ADR 0022）と同じ扱いである。
- 既定の内容は、すべてのクローラにすべてのページのクロールを許可する（`User-agent: *`、`Allow: /`）。特定のクローラを断る書き方の例を注釈に書く。
- `init` は、既定の内容の `content/robots.txt` を作る。書き手がすぐに編集できるようにするためである。`--force` では作り直す。
- 見本（`example/content/robots.txt`）にも、同じ内容を置く。
- `build`、`serve`、`deploy` のどれでも配信し、`--data-only`（Vite の開発サーバー向け）でも書き出す。

`robots.txt` はクローラがドメインの直下でしか読まないので、`base_path`（ADR 0030）でドメインの直下でない場所に置くサイトでは効かない。README に書く。

### サイトマップ

`site.toml` に `url` を書いたときは、`/sitemap.xml` を作る。

- サイトは SPA で、ページの一覧を HTML のリンクからたどれない（DB を読んだ後に描く）。検索エンジンにページの一覧を知らせる手段として作る。
- URL は絶対 URL である必要があるので、`url` がなければ作らない（RSS と同じ、ADR 0033）。
- 載せるのは、トップページ、一覧（`/archive`）、自己紹介（page の `about` があるとき）、すべての article と post。検索のページ（結果が言葉で変わる）と、URL のないページは載せない。
- 最終更新日（`lastmod`）は、article は改稿日（なければ公開日）、post は公開日、トップページと一覧は最も新しい記事の日付にする。自己紹介には日付を持たないので付けない。
- `changefreq` と `priority` は付けない。Google は読まないと公表している。

`robots.txt` に、サイトマップの場所（`Sitemap: <url>sitemap.xml`）を足す。

- 書き手の `content/robots.txt` にも足す。`init` が作る `content/robots.txt` に、サイトの URL を書かせずに済むようにするためである。
- すでに `Sitemap:` の行（大文字と小文字を区別しない）があれば、足さない。書き手の指定を優先する。

### 断らないもの

- **AI の学習用のクローラ**（GPTBot、ClaudeBot、CCBot、Google-Extended など）は、既定では断らない。見本の記事は CC0 で公開しており、学習に使われることを拒む理由がない。断りたい書き手は、`content/robots.txt` に書き足せる（書き方の例を既定の内容の注釈に置く）。
- **DB（`/db/`）** は断らない。Google などは JavaScript を実行してページを読むので、DB を断ると本文が見えなくなる。

## 実装しないこと

- `site.toml` でのクローラごとの許可の指定。`content/robots.txt` を直接書けば足りる。
- 画像のサイトマップ、ニュースのサイトマップ。
- `url` がないときのサイトマップ（相対 URL のサイトマップは仕様にない）。

## テスト設計

- robots.txt：既定の内容の指示がすべて許可の二行だけであること。`content/robots.txt` があればそれを、なければ既定の内容を配信すること。`init` が作り、`--force` で作り直すこと。
- サイトマップ：載せるページと並び、最終更新日（改稿日を優先すること、トップページと一覧は最新の日付）、URL のない page を載せないこと、記事がないときの形。
- robots.txt への `Sitemap:` の行：一度だけ足し、すでにある（大文字でも）ときは足さず、末尾に改行がなくても足せること。
- 組み立て：`url` があるときだけサイトマップを作り、`robots.txt` に行を足すこと。
- 結合テスト：見本をビルドすると `robots.txt` と `sitemap.xml` ができ、`robots.txt` にサイトマップの場所が入ること。

見本をビルドした `sitemap.xml` を XML として読めることを確かめた。
`serve` が `robots.txt` を `text/plain` で返し、Lighthouse の robots.txt の監査に合格することを確かめた。
