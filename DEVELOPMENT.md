# sqlite-cms の開発

このファイルは sqlite-cms そのものを開発する人向け。
サイトを作って公開するだけなら `README.md` を読めばよく、ここに書いたことは知らなくてよい。

仕様は `docs/specifications.md`、設計決定は `docs/adr/` にある。

## 構成

```
cli/         CLI sqlite-cms（Rust）。利用者が使うのはこれだけ
web/         SPA（React + Vite + Tailwind）。ビルド結果は CLI に埋め込まれる
migrations/  SQLite のマイグレーション。CLI に埋め込まれる
example/     記事リポジトリのサンプル。動作確認とテストに使う
docs/        仕様書と ADR
```

ルートは Cargo のワークスペース。TS は `web/` の中で完結する。

## 必要なもの

- Rust（stable）
- Node 22 以降

型検査には、Go で実装された TypeScript 7 の `tsc` を使う（ADR 0012）。
npm がプラットフォーム用のバイナリを自動で入れるので、Go を別に入れる必要はない。
7 系は JavaScript のコンパイラ API を持たないので、TypeScript の API に依存するツールはそのままでは使えない。

## ADR

設計の判断は `docs/adr/` に、連番の ADR として書く。
コミットした ADR は書き換えない。判断を改めるときは、次の番号で新しい ADR を書き、どの ADR のどの判断を改めるかをそこに書く。
コミットしていない ADR は、コミットするまで直してよい。

Claude Code で作業するときは、`.claude/settings.json` の PostToolUse のフック（`.claude/hooks/check-adr-immutable.sh`）が、ツールを使うたびに作業ツリーとインデックスを HEAD と比べる。
コミット済みの ADR が書き換えられたり、消されたり、名前を変えられたりしていれば、エラーにして取り消すよう伝える。
編集のツールだけでなく、シェルのコマンドでの書き換えも見つける。
ただし、書き換えの後に知らせる仕組みなので、書き換え自体は止めない。

コミットは、git の pre-commit フック（`.githooks/pre-commit`）で止める。
コミットする中身（インデックス）を HEAD と比べ、コミット済みの ADR を書き換え、消し、名前を変えるコミットを拒む。
新しい ADR を足すコミットは通る。
フックはリポジトリで管理しているが、git は自動では使わないので、クローンしたら一度だけ次を実行して有効にする。

```sh
git config core.hooksPath .githooks
```

有効になっているかは `git config --get core.hooksPath` で確かめられる（`.githooks` と出る）。

## 依存の方針

依存は必要なものに絞る（ADR 0013、0014）。
数十行で書けるものは自前で実装し、ライブラリを足さない。

- `web/`：ルーター（`web/src/router.tsx`）と sql.js の型（`web/src/types/sql.js.d.ts`）を自前にしている。sql.js の API を新しく使うときは、型の宣言も足す。
- `web/`：シンタックスハイライトは Shiki（ADR 0017）。言語は `web/src/highlight.ts` の `LANGUAGES` に登録したものだけが色付きになる。言語を足すときは、`@shikijs/langs/<言語>` を登録し、`web/src/highlight.test.tsx` の `SAMPLES` に見本を足し（足さないとテストが落ちる）、`web/src/highlightLanguages.ts` の `HIGHLIGHT_LANGUAGES` に言語名と別名を足し（足さないとテストが落ちる。Shiki を読み込むかをこの一覧で決める、ADR 0029）、README の対応表を更新する。Shiki は、色を付ける言語のコードブロックが表示されたときに `web/src/highlightLoader.ts` 経由で読み込む別チャンクなので、`web/src/highlight.ts` と Shiki のモジュールを本体のコードから静的に import しない（すると本体に取り込まれる）。1 言語で gzip 後 1〜16 KB 増えるので、`web/vite.config.ts` の `chunkSizeWarningLimit` も必要に応じて見直す。
- `web/`：Markdown の拡張の記法（ADR 0025）は、micromark の構文拡張と mdast の書き換えを `web/src/markdown/` に自前で書く。remark や rehype のプラグインのライブラリは足さず、再実装が現実的でない描画のライブラリ（KaTeX、mermaid）だけを使う。KaTeX と mermaid は `web/src/render/loaders.ts` 経由で非同期に読み込む別チャンクなので、`web/src/render/katex.ts` と `web/src/render/mermaid.ts` を本体のコードから静的に import しない。KaTeX のフォントは `web/vite.config.ts` のプラグインで woff2 だけを残す。
- `web/`：SPA は相対のパス（Vite の `base: "./"`）でビルドする。どのパスに置いても読めるよう、CLI が `index.html` のパスを `site.toml` の `base_path` に合わせて書き換え、SPA は `<meta name="sqlite-cms-base">` を読んでパスを組み立てる（ADR 0030）。SPA の中でサイト内のパスを URL にするときは `web/src/base.ts` の `withBasePath` を通す。
- `web/`：本文の HTML は `rehype-raw` で取り込み、`rehype-sanitize` で `web/src/sanitize.ts` の許可一覧以外を取り除く（ADR 0018）。許可するタグを足すときは `EXTRA_TAGS` に足し、`web/src/sanitize.test.tsx` にテストを足す。sanitize は Markdown が作る要素（脚注、タスクリスト、コードブロックの言語名）も検査するので、許可一覧から外すと Markdown の出力が壊れる。Shiki は sanitize の後に通す。
- `cli/`：base64（`base64.rs`）、MIME の対応表（`mime.rs`）、frontmatter のパーサ（`frontmatter.rs`）、テスト用の一時ディレクトリ（`testutil.rs`）を自前にしている。配信するファイルの種類を増やすときは `mime.rs` の表に足す。

## ビルド

CLI は `web/dist` を埋め込むので、SPA を先にビルドする。

```sh
npm --prefix web ci
npm --prefix web run build   # web/dist に SPA を出力
cargo build --release        # target/release/sqlite-cms に SPA が埋め込まれる
```

`web/dist` がないまま `cargo build` すると、警告を出して SPA なしのバイナリができる。
Rust 側だけを触るときはそれで足りるが、`serve`、`build`、`deploy` はエラーになる。

## テスト

```sh
npm --prefix web test        # SPA（vitest）
npm --prefix web run build   # 結合テストの前に必要
cargo test                   # CLI（単体テストと結合テスト）
cargo clippy --all-targets -- -D warnings
```

結合テスト（`cli/tests/cli.rs`）は実際のバイナリを起動する。
`deploy` のテストは、テスト内に立てた偽の Cloudflare API に向けて実行する（`CLOUDFLARE_API_BASE_URL`）。
結合テストは `SQLITE_CMS_OFFLINE=1` で起動し、見本の記事のリンクカードの画像を外部から取得しない。リンクカードの取得のテストは、テスト内に立てた偽のサイトから取得する。
テストは `example/.env` を一時ディレクトリに写さない（本物の認証情報で公開しないため）。
GitHub Pages と rsync への公開の結合テストは、手元の空のリモート（`git init --bare`）と手元のディレクトリに向けて実行する。`git` と `rsync` が要る（macOS の openrsync でも動く）。

`serve` や Lighthouse の計測で見本を配信すると、リンクカードの画像を取得して `example/.sqlite-cms-cache/` に保存する（Git には入れない）。

## SPA の開発

```sh
npm --prefix web run dev
```

`sqlite-cms build --data-only` で DB と画像を `web/public` に書き出してから、Vite の開発サーバを起動する。
記事は既定で `example/` のものを使う。別の記事リポジトリで試すときは `SITE_DIR` で指定する。

```sh
SITE_DIR=../../blog npm --prefix web run dev   # web/ から見た相対パス
```

`--data-only` は開発用のオプションで、利用者向けのヘルプには出さない。
出力先の `db/` と `media/` だけを置き換え、ほかのファイルには触れない。

完成形の確認には、CLI のプレビューを使う。

```sh
cargo run -- serve example
```

## マイグレーション

`migrations/NNNN_名前.sql` に連番で足す。既存のファイルは編集しない。
ビルドスクリプトが自動で埋め込むので、コードの変更は要らない。
スキーマを変えたら、`web/src/db.ts` のクエリも合わせて直す。

## CI

`main` への push と PR のたびに、`.github/workflows/ci.yml` が次を実行する。

1. SPA のビルド（型検査を含む）と vitest
2. SPA を埋め込んだ `cargo clippy --all-targets -- -D warnings` と `cargo test`（結合テストを含む）
3. Lighthouse の計測（下記）。レポートは成功しても失敗しても `lighthouse-reports` という Artifact に保存し、スコアの表は実行結果の概要に出す

同じ PR に続けて push したときは、古い実行を取り消す。`main` への push は取り消さない。

## Lighthouse

```sh
npm --prefix web run build   # SPA を最新にしてから
npm --prefix web run lighthouse
```

`sqlite-cms serve` で `example/` を配信し、代表的な 6 ページ（`web/scripts/lighthouse.mjs` の `PAGES`）をヘッドレスの Chrome で計測する（ADR 0019）。
手元に Chrome が要る。
レポート（HTML と JSON）は `web/lighthouse-reports/` に日本語で出る。
配信には既定で `cargo run` を使う。ビルド済みのバイナリで計測するときは、`SQLITE_CMS_BIN` にそのパスを渡す。

アクセシビリティとベストプラクティスは 100 点でなければ失敗し、足りない項目と要素を表示する（ADR 0024）。
ほかの項目（パフォーマンス、SEO）は計測して記録するだけで、基準はまだ決めていない。

`sqlite-cms serve` は、本番の Cloudflare と同じく応答を gzip で圧縮する（ADR 0026）。圧縮は自前の実装で、zlib より 1〜2 割ほど大きくなるので、パフォーマンスの値は本番より少し悪く出る。
基準を足すときは `REQUIRED` に書く。

## リリース

`v*` のタグを push すると、`.github/workflows/release.yml` が次を行う。

1. SPA をビルドし、テストを実行する
2. Linux x86_64 と macOS arm64 のバイナリをビルドする
3. Linux のビルドで作ったバイナリで Lighthouse を計測し、レポートを `lighthouse-reports` の Artifact に保存する。アクセシビリティかベストプラクティスが 100 点を割ると、ここで失敗して Release は作られない
4. `sqlite-cms-<target>.tar.gz` を GitHub Release に添付する

タグを付けずに試すときは、`gh workflow run release.yml` で手動実行する。ビルドと梱包までを行い、Release は作らない（成果物は実行結果の Artifacts から取れる）。

記事リポジトリの公開ワークフロー（雛形は `example/.github/workflows/deploy.yml`）は、最新のリリースからバイナリを取得する。
