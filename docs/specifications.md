# sqlite-cms 仕様書

sqlite-cms は個人の記事置き場である。
組版システム golem とは独立したリポジトリであり、将来 golem のプレーンテキスト出力（横書き変種）を記事ソースとして受け入れる余地だけを残す。

この文書は sqlite-cms の目的、アーキテクチャ、意図的に採用しなかったもの、ディレクトリ構成を定める。
設計決定と実装ステップは `adr/` の連番 ADR に置く。

## 目的

自分の記事を置き、読める状態に保つ。
それ以上の機能（コメント、解析、購読）は持たない。
SEO は考慮しない。
読者は少数であり、検索エンジン経由の流入を設計目標にしないためである。

## コンテンツ種別

コンテンツは三種別に分ける（ADR 0004）。

- **post**：ブログ的な軽量の記事。日付とタイトルだけを持つ。
- **article**：独立した読み物。一覧用の要約と改稿日を任意で持つ。
- **page**：自己紹介などの固定ページ。日付を持たない。

post と article の境界は書き手の判断に委ねる。

## リポジトリの分担

このリポジトリはツールだけを持つ。
記事などのコンテンツは別のリポジトリ（以下、コンテンツリポジトリ）で Markdown ファイルとして管理する（ADR 0009）。

- **このリポジトリ**：スキーマ（マイグレーション）、CLI `sqlite-cms`、SPA。
- **コンテンツリポジトリ**：サイトのメタデータと公開先（`site.toml`）、記事の Markdown、トップページの本文、画像。

コンテンツの更新でツールのリポジトリにコミットが生じず、ツールの変更で記事の履歴が汚れない。
このリポジトリの `example/` は動作確認用のサンプルであり、公開する記事ではない。

## 利用者と開発者

サイトの管理（プレビュー、書き出し、公開）は、すべて CLI `sqlite-cms` で行う（ADR 0011）。
利用者（コンテンツリポジトリの持ち主）が扱うのは、CLI のバイナリとコンテンツリポジトリだけである。
SPA はバイナリに埋め込まれて配布されるので、利用者は Node も TS のコードも意識しない。

TS（`web/`）は SPA を開発する人だけが触る。
開発の手順は `DEVELOPMENT.md` にまとめ、利用者向けの `README.md` からは切り離す。

## アーキテクチャ

サイトの組み立て時に全コンテンツを単一の SQLite ファイルへ格納し、SPA と画像と合わせて静的アセットとして配信する。
組み立ては Rust の単一バイナリ `sqlite-cms` が行う（ADR 0006、0008、0011）。
マイグレーションと SPA はバイナリに埋め込まれており、バイナリはコンテンツリポジトリのディレクトリだけを引数に取る（ADR 0009、0011）。
DB に入れる本文は Markdown のままであり、HTML への変換はブラウザが行う（ADR 0007）。
スキーマは連番 SQL マイグレーションの適用列として定義し、ビルドごとに空 DB へ全件適用する（ADR 0005）。
フロントエンドは Preact（`preact/compat`）の SPA であり（ADR 0039）、起動時に SQLite ファイルを取得してブラウザ内で自前の読み手で読み（sql.js は使わない。ADR 0047）、以後の一覧・本文表示はローカルの DB 参照だけで完結する。
ページ間の遷移にネットワークアクセスは発生しない。

単一 SQLite を選ぶ根拠は二つある。

- **デプロイの原子性**：コンテンツごとのファイル群を配る方式では、アップロード途中に一覧と本体の不整合が生じうる。単一ファイルとバージョン付きファイル名の組では、参照が切り替わった瞬間に必ず完全なスナップショットが見える。
- **クエリ能力**：CLI は SQL で DB を組み立てる。ブラウザは表の行を読み、一覧、日付順、全文検索を JavaScript で行う（ADR 0047）。

DB ファイルはコンテンツハッシュ付きのファイル名で配信し、`immutable` キャッシュを与える。
SPA は無キャッシュの小さなマニフェストを読んで現行 DB の URL を解決する。
これにより、更新のたびに再取得されるのはマニフェストと差し替わった DB だけになる。

## 意図的に採用しなかったもの

- **SSR / SSG**：SEO を考慮しないため、クローラ向けの HTML 生成は持たない。
- **画像の DB 格納**：画像は少数でも DB に入れず、静的ファイルとして配信する。ブラウザキャッシュと遅延読み込みをそのまま使え、DB サイズを本文テキストだけに保てるためである。画像はコンテンツリポジトリの `content/media/` に置く。多用する段になったら CDN の画像最適化に載せる。
- **Cloudflare D1**：サーバサイドの DB 参照は要らない。遷移をネットワーク不要にするというアーキテクチャと矛盾するためである。
- **記事ごとの JSON 配信**：デプロイの原子性で単一 SQLite に劣る。
- **down マイグレーション**：DB は使い捨ての成果物であり、巻き戻しはマイグレーションファイルの削除で足りる。
- **パララックス、スクロール効果、ダークモード**：白背景で本文幅を制限した静的な見た目だけを持つ。

## 見た目

- 背景は白。装飾は持たない。
- 本文の行長は読みやすさのために制限する（およそ 65 字相当の最大幅）。
- CSS は Tailwind で書くが、見た目の目標は古いウェブサイトである。リンクは下線付きの青でよい。
- アクセシビリティとベストプラクティスは Lighthouse で 100 点を保つ。CI で `example/` の代表的なページを計測し、下回れば失敗させる（ADR 0019、0024）。
- JS と DB を読み込むまでの間は、`index.html` に書いた読み込み中の表示を出す（ADR 0023）。
- `index.html` は、DB を先読みし、本体の CSS を埋め込んで持つ（ADR 0038、0047）。

## ディレクトリ構成

このリポジトリの構成は次のとおり。

```
cli/           -- CLI `sqlite-cms`（Rust。利用者向け）
web/           -- Preact SPA（TS。開発者向け。ビルド結果は CLI に埋め込まれる）
migrations/    -- 連番 SQL マイグレーション（CLI に埋め込まれる）
example/       -- 動作確認用のサンプル（コンテンツリポジトリと同じ構成）
```

コンテンツリポジトリの構成は次のとおり（ADR 0010）。

```
site.toml      -- サイトのメタデータ（必須）
content/
  index.md     -- トップページの本文（任意）
  header.md    -- ヘッダ（任意。site.toml の値を Mustache のような書き方で埋め込める。なければ既定のヘッダ。ADR 0043）
  favicon.svg  -- ファビコン（任意。/favicon.svg で配信する。なければサイト名の頭文字を描いた仮のものを配信する。ADR 0022）
  robots.txt   -- クローラへの指示（任意。/robots.txt で配信する。なければすべてのクロールを許可する内容を配信する。url があればサイトマップの場所を足す。ADR 0037）
  posts/       -- post の Markdown
  articles/    -- article の Markdown
  pages/       -- 固定ページの Markdown
  media/       -- 画像など（任意。/media/ で配信する）
.env           -- deploy の認証情報（任意。Git に入れず、配信もしない。ADR 0027）
.env.example   -- .env の見本（init が作る）
.sqlite-cms-cache/ -- ビルドのときに取得したもの（リンクカードの画像。Git に入れない。ADR 0028）
```

`sqlite-cms` は、`site.toml` と `content/` から次の一式を組み立てる。

```
/index.html, /assets/...          -- 埋め込まれた SPA
/db/articles-<hash>.sqlite        -- 全コンテンツを格納した DB
/db/manifest.json                 -- 現行の DB を指すマニフェスト
/media/...                        -- content/media/ の中身
```

組み立てた一式を、`serve` は手元の HTTP サーバで返し、`build` はディレクトリに書き出し、`deploy` は Cloudflare にアップロードする。
`serve` は、要求が受け付けるなら、テキスト、JS、JSON、SVG、wasm、DB を gzip で圧縮して返す（ADR 0026）。起動したら、別のスレッドで先に圧縮しておく（ADR 0038）。
`serve` は `site.toml` と `content/` の変更を監視し、変わったら組み立て直して、Server-Sent Events でブラウザに読み込み直させる。`--no-reload` で止められる（ADR 0034）。

## サイトのメタデータ

`site.toml` で次を指定する。

| キー | 必須 | 用途 |
|---|---|---|
| `title` | 必須 | サイト名。ヘッダとページタイトル |
| `author` | 任意 | 著者名。フッター |
| `license.name` | `[license]` を書くなら必須 | ライセンス名（`CC0 1.0` など）。フッター |
| `license.url` | 任意 | ライセンスの URL。書けばライセンス名をリンクにする |
| `deploy.target` | 任意 | 公開先。`cloudflare`（省略したとき）、`github-pages`、`rsync` |
| `deploy.worker` | Cloudflare なら必須 | 公開先の Cloudflare Worker 名 |
| `deploy.branch`、`deploy.remote`、`deploy.cname` | GitHub Pages で任意 | push するブランチ（既定 `gh-pages`）、リモート（既定 `origin`）、独自ドメイン |
| `deploy.destination` | rsync なら必須 | rsync の送り先（`user@host:/path/` や手元のパス） |
| `timezone` | 任意 | `new` が入れる今日の日付のタイムゾーン。IANA の名前（`Asia/Tokyo`）か時差（`+09:00`）。省略すると環境のタイムゾーン |
| `description` | 任意 | ページの meta description。省略すると「`<title>`。記事とブログを置いているサイトです。」。要約を持つ article のページでは記事の要約を使う（ADR 0022） |
| `base_path` | 任意 | サイトを置くパス。`/` で始める（末尾の `/` は補う）。省略すると `/`。Cloudflare では使えない（ADR 0030） |
| `url` | 任意 | 公開したサイトの URL（`https://…`）。書くと RSS 2.0 のフィード `/rss.xml`（ADR 0033）とサイトマップ `/sitemap.xml`（ADR 0037）を作る。パスは `base_path` と同じにする |

知らないキーはエラーにする。
フッターは `author` と `license` のどちらかがあるときだけ表示する。

## コンテンツフォーマット

frontmatter（YAML）にメタデータを置き、本文は Markdown で書く。
frontmatter は YAML のうち、1 行に 1 つの「キー: 値」だけを受け付ける（ADR 0014）。
値はクォートなし、ダブルクォート、シングルクォートのいずれかで書き、複数行の値、リスト、入れ子、アンカーはエラーにする。
slug はファイル名（拡張子を除く）から取る。

| キー | post | article | page |
|---|---|---|---|
| `title` | 必須 | 必須 | 必須 |
| `date` | 必須 | 必須 | なし |
| `description` | なし | 任意 | なし |
| `updated` | なし | 任意 | なし |

ビルダーは frontmatter の検証だけを行い、本文の Markdown を加工せずに DB へ格納する。
SPA が remark（react-markdown）で本文を描画する。
記法は CommonMark に GFM（表、取り消し線、タスクリスト、自動リンク、脚注）を加えたものとする。
コードブロックは、言語名が登録済みの言語なら Shiki で色を付け、それ以外は色なしで表示する（ADR 0017）。Shiki、KaTeX、mermaid は、それを使うコードブロック、数式、図が画面の近くに来て、本文を描いた後に初めて読み込み、読み込むまでは生のテキストを表示する（ADR 0029、0038）。コードブロックは一つずつ色分けする。
本文中の HTML は、許可したタグと属性だけを残す（ADR 0018）。
Zenn を参考にした拡張（`:::message`、`:::details`、コードのファイル名と `diff 言語`、数式、mermaid の図、画像の幅と説明、インラインの脚注、URL だけの行のリンクカード）を加える（ADR 0025）。記法の解析は自前の remark のプラグインで行い、数式は KaTeX、図は mermaid を、それぞれ必要なページでだけ非同期に読み込んで描く。
リンクカードの画像は、CLI がサイトを組み立てるときに各ページの `og:image` を取得して `/link-cards/` から配信し、URL との対応を DB の `link_cards` に入れる（ADR 0028）。画像は、カードが画面の近くに来てから読み込む（ADR 0038）。
許可するのは Markdown では書けない表現のためのタグ（`details`、`summary`、`kbd`、`sub`、`sup`、`ruby`、`rt`、`rp`、`mark`、`abbr`、`dl`、`dt`、`dd`、`br`）で、それ以外のタグは外して中身の文字を残す。
`script`、`iframe`、`style` とコメントは中身ごと取り除き、`class`、`style`、イベント属性、脚注以外の `id` は取り除く。

本文中のサイト内リンク（`/` で始まる URL）は SPA 内の遷移として扱い、ページの再読み込みを起こさない。

## ルーティング

- `/`：トップページ。`content/index.md` の本文と、article・post の一覧。
- `/posts/:slug`、`/articles/:slug`：本文。
- `/about`：自己紹介（`pages` の slug `about`）。
- `/search`：全文検索。`?q=` に探す言葉を入れる（ADR 0031）。ヘッダに検索ボックスを置き、送ると `/search?q=` に移る。入力中は候補を 5 件まで出し、なければその旨を出す（ADR 0041、0042）。
- `/archive`：post と article をまとめた一覧。年ごとに新しい順に並べる（ADR 0033）。

## CLI

```
sqlite-cms init [SITE_DIR] [--title <TITLE>] [--force]
                                               -- 記事リポジトリに必要なファイルを作る（ADR 0016、0027）
sqlite-cms new <post|article|page> [SLUG] [SITE_DIR] [--title <TITLE>] [--date <YYYY-MM-DD>]
                                               -- 記事の雛形を作る（ADR 0015、0020）
sqlite-cms serve  [SITE_DIR] [--port <PORT>] [--no-reload]
                                               -- 手元でプレビューする（記事の変更を自動で反映する）
sqlite-cms build  [SITE_DIR] [--out <DIR>]     -- 配信用のディレクトリに書き出す
sqlite-cms deploy [SITE_DIR]                   -- Cloudflare Workers に公開する
```

`SITE_DIR` の既定はカレントディレクトリである。

## 公開

`sqlite-cms deploy` は、`site.toml` の `[deploy] target` で選んだ公開先に公開する（ADR 0030）。

| `target` | 公開の方法 |
|---|---|
| `cloudflare`（省略したとき） | Cloudflare Workers の静的アセット配信（下記） |
| `github-pages` | 記事リポジトリの `gh-pages` ブランチ（`branch`）に、リモート `origin`（`remote`）を通して、公開用の一式だけのコミットを強制的に push する。作業ツリー、インデックス、手元のブランチには触れない。`.nojekyll` と、`cname` を書いたときは `CNAME` を置く |
| `rsync` | `destination` に `rsync -rlptz --delete` で送る。送り先が空か、目印の `.sqlite-cms` があるときだけ送る |

サイトをドメインの直下でない場所（GitHub Pages のプロジェクトのページなど）に置くときは、`site.toml` の `base_path` を指定する。
CLI は SPA の `index.html` のアセットのパスを `base_path` から始まる形に書き換え、`<meta name="sqlite-cms-base">` で SPA に伝える。
SPA はルータ、DB の取得、本文のサイト内のリンクと画像のパスに `base_path` を付ける。
`serve` も `base_path` の下で配信する。
Cloudflare では `base_path` を使えない。
SPA のフォールバックの設定がない配信先のため、`index.html` と同じ中身の `404.html` を置く。

### Cloudflare Workers

Cloudflare Workers の静的アセット配信に公開する。
`sqlite-cms deploy` が Cloudflare の API を直接呼び、wrangler も Node も使わない（ADR 0011）。
認証情報は環境変数 `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` で渡し、公開先の Worker 名は `site.toml` の `[deploy] worker` で指定する。
環境変数は記事リポジトリの `.env` にも書ける。両方あれば環境変数を優先する。`init` は見本の `.env.example` を作り、`.gitignore` に `.env` を書く（ADR 0027）。
API トークンに要る権限は、アカウントの「Workers スクリプト：編集」（Workers Scripts Write）だけである。

コンテンツリポジトリの CI は、リリースから `sqlite-cms` のバイナリを取得して `sqlite-cms deploy` を実行する。
このワークフローの雛形を `example/.github/workflows/deploy.yml`（Cloudflare）と `example/.github/workflows/deploy-github-pages.yml`（GitHub Pages）に置く。

## 配布

`v*` のタグを push すると、このリポジトリの CI が SPA を埋め込んだバイナリをビルドし、GitHub Release に添付する。
利用者はこのバイナリを使う。

## 未決事項

次の項目は、ADR 0031、0033、0034 で対応した：全文検索（0031）、統合一覧と RSS（0033）、`serve` の自動反映（0034）。

- golem のプレーンテキスト出力を記事ソースに取り込む経路（`.txt` の記事として受け入れる案を作ったが、golem の仕様が固まるまで保留した）
- X（Twitter）などに URL を貼ったときのカード。SPA の `index.html` はタイトルが空で OGP の meta もなく、JavaScript を実行しない X のクローラにはサイト名も説明も見えない。`build` が `site.toml` の `title` と `description`（記事のページなら記事の題名と要約）を HTML に書き込む方法が考えられる。ページごとに HTML を作る案を作ったが、作るファイルが増えるので保留し、ほかの方法を考える
- Linux x86_64 と macOS arm64 以外のバイナリの配布
- 利用者がコードの色分けの言語を増やす方法。今は色を付ける 15 の言語が SPA に組み込まれ、SPA はバイナリに埋め込まれているので、記事リポジトリの側からは増やせない。案は次のとおりで、どれもまだ決めていない
  - Shiki のすべての言語を言語ごとのチャンクにしてバイナリに入れ、記事で使われた言語だけを読み込む（バイナリが数 MB 大きくなる）
  - 記事リポジトリに TextMate の文法（JSON）を置き、CLI が配信して SPA が読み込む（`site.toml` で言語名と別名を指定する）
  - CDN から読み込む（閲覧のときにほかのサイトと通信するので、ADR 0028 の方針に反する）
