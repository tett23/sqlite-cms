# 0011 CLI による管理と SPA の埋め込み

## 状態

採用。
次の決定を置き換える。

- 0003 の「wrangler でデプロイし、このリポジトリの CI が公開する」
- 0008 の「サブコマンドは導入しない」
- 0009 の「このリポジトリの CI が記事リポジトリを取り込んで公開する」「npm スクリプトが `SITE_DIR` で記事リポジトリを受け取る」「`--public` で配信用ディレクトリを受け取る」

## 決定

サイトの管理（プレビュー、書き出し、公開）は、すべて Rust の CLI `sqlite-cms` で行う。
利用者（記事リポジトリの持ち主）は、CLI のバイナリと記事リポジトリだけを扱う。
TS で書いた SPA は開発者だけが触り、利用者には意識させない。

### SPA の埋め込み

- SPA を `web/` に移し、vite の出力先を `web/dist` にする。`web/public` の中身（開発用の DB と画像）は出力にコピーしない。
- CLI のビルドスクリプトが `web/dist` を `include_bytes!` でバイナリに埋め込む。
- `web/dist` がないときは、警告を出して SPA なしでビルドする。SPA を使うコマンド（`serve`、`build`、`deploy`）は、SPA がないとエラーにする。開発中に Rust 側だけを触るときに、SPA のビルドを強制しないためである。
- キャッシュ方針の `_headers` は、配信の設定なので CLI の定数として持つ。

### サブコマンド

```
sqlite-cms serve  [SITE_DIR] [--port <PORT>]
sqlite-cms build  [SITE_DIR] [--out <DIR>]
sqlite-cms deploy [SITE_DIR]
```

三つとも、SPA、DB、マニフェスト、画像からなる一式をメモリ上に組み立てる。
違うのは組み立てた一式の行き先だけである。

- **serve**：`127.0.0.1` の HTTP サーバで返す。拡張子のないパスには `index.html` を返す（Cloudflare の SPA 設定と同じ動作）。ファイルシステムは参照せず、組み立てた一式からだけ探すので、パスの細工で手元のファイルを読まれることはない。記事の変更の反映には再起動が要る。
- **build**：ディレクトリに書き出す（既定は `dist`）。出力先に目印のファイル `.sqlite-cms` を置き、次の書き出しでは中身を消して作り直す。目印のない、空でないディレクトリには書き出さない。`--out .` のような指定の誤りで記事リポジトリを消さないためである。
- **deploy**：Cloudflare にアップロードする（次節）。

開発者向けに `build --data-only` を持つ。DB とマニフェストと画像だけを書き出し、出力先の `db/` と `media/` だけを置き換える。`web/` の開発サーバがこれを使う。利用者向けのヘルプには載せない。

### Cloudflare への公開

wrangler を使わず、Workers の静的アセットの API を直接呼ぶ。
手順は wrangler（4.x）の実装に合わせる。

1. `POST /accounts/{account}/workers/scripts/{worker}/assets-upload-session` にマニフェスト（パスごとのハッシュとサイズ）を送る。ハッシュは、ファイル内容の base64 に拡張子（ドットなし）をつなげた文字列の blake3 の 16 進表記の先頭 32 桁とする。
2. 応答の `buckets` に含まれるファイルだけを、バケット単位で `POST /accounts/{account}/workers/assets/upload?base64=true` に multipart で送る。認証には手順 1 の JWT を使う。JWT の本文に `wrangler_single_asset_uploads: true` があるときは、ファイルごとに `POST .../workers/assets/upload/{hash}` へそのまま送る。`buckets` が空なら、手順 1 の JWT が完了トークンである。
3. `PUT /accounts/{account}/workers/scripts/{worker}` に、`metadata` パートだけを持つ multipart を送る。`metadata` には完了トークン、`not_found_handling: single-page-application`、`_headers`、互換日付を入れる。スクリプトなしの Worker になる。
4. `POST .../workers/scripts/{worker}/subdomain` で workers.dev の URL を有効にし、`GET .../workers/subdomain` で URL を組み立てて表示する。この手順の失敗は警告にとどめる。サイトの公開は手順 3 で済んでいるためである。

- 認証情報は環境変数 `CLOUDFLARE_API_TOKEN` と `CLOUDFLARE_ACCOUNT_ID` で渡す。トークンはリクエストのヘッダにだけ載せ、出力には出さない。
- 公開先の Worker 名は `site.toml` の `[deploy] worker` で指定する。API の URL のパスに入るため、英小文字、数字、ハイフンの 63 文字以内に限る。
- 環境変数 `CLOUDFLARE_API_BASE_URL` で API の接続先を変えられる。wrangler と同じ名前であり、テストでは偽の API サーバに向けるのに使う。
- 通信の失敗と HTTP 429、5xx は、間隔を空けて 3 回まで送り直す。
- 25 MiB を超えるファイルは、通信の前にエラーにする（Cloudflare の上限）。

### リポジトリの構成と配布

- ルートを Cargo のワークスペースにし、TS は `web/` にまとめる。ルートの `README.md` は利用者向け、`DEVELOPMENT.md` は開発者向けとする。
- `v*` のタグで、SPA を埋め込んだバイナリ（Linux x86_64、macOS arm64）をビルドし、GitHub Release に添付する。
- このリポジトリの CI は、テストとリリースだけを行う。記事リポジトリを取り込んで公開するワークフローはやめる。
- 記事リポジトリ用の公開ワークフローの雛形を `example/.github/workflows/deploy.yml` に置く。リリースからバイナリを取得し、`sqlite-cms deploy` を実行するだけのものである。
- wrangler の依存と `wrangler.jsonc` を削除する。

## 実装しないこと

- `serve` での変更の監視と自動の再読み込み。
- 独自ドメインとルートの設定。Cloudflare のダッシュボードで行う。
- プレビュー用の URL（Worker のバージョンごとの URL）。
- 上記以外のプラットフォーム向けのバイナリ、パッケージマネージャでの配布。

## テスト設計

`cargo test` で次を検証する。

- 引数の解釈（サブコマンドごとの既定値とオプション、不明なコマンドとオプション、値の検証、`--help` の優先）。
- 組み立て（DB、マニフェスト、画像、SPA が揃うこと、SPA がないとエラーになること）と書き出し（`_headers` と目印、前回の出力の置き換え、目印のない非空ディレクトリの拒否、`--data-only` が `db/` と `media/` だけを置き換えること）。
- API クライアント（エラーの本文の報告、トークンを出力しないこと、送り直しと打ち切り、multipart の形式、JWT による単一アップロード方式の判定）。
- 公開の手順（偽の HTTP で、リクエストの順序と内容、アップロード済みのときの省略、単一アップロード方式、マニフェストにないハッシュ、完了トークンの欠落、上限を超えるファイル、workers.dev の失敗が警告になること）。
- `serve` の応答（Content-Type、SPA の代替応答、パーセントエンコードの復号、404、405）。

結合テスト（実際のバイナリを起動する）で次を検証する。

- `--help` の出力と、コマンドなしのエラー。
- `build` と `build --data-only` の出力。
- `serve` が SPA、DB、画像を返すこと。
- `deploy` が、テスト内に立てた偽の Cloudflare API に対して、実際の HTTP で手順 1 から 4 を行うこと。
- `deploy` が `[deploy]` と認証情報の欠落で止まること。

SPA を使う結合テストは、`web/dist` があることを前提とする（ないときは手順を示して失敗する）。
本物の Cloudflare への公開は、認証情報のある環境で手動で確認する。

## トレードオフ

- 公開の手順は、Cloudflare の公開ドキュメントと wrangler の実装に合わせているが、API の仕様変更に wrangler ほど早くは追随できない。変更があれば、このリポジトリで直してリリースし直す必要がある。
- SPA を埋め込むため、SPA の変更にも CLI のリリースが要る。利用者から見ると、ツールの版が一つにまとまる利点のほうが大きい。
- バイナリは約 6 MB になる。SPA（wasm を含む）と SQLite を同梱するためである。
- 開発者は、SPA のビルドを CLI のビルドより先に行う必要がある。
