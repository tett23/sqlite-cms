# sqlite-cms の開発

このファイルは sqlite-cms そのものを開発する人向け。
サイトを作って公開するだけなら `README.md` を読めばよく、ここに書いたことは知らなくてよい。

仕様は `docs/specifications.md`、設計決定は `docs/adr/` にある。

## 構成

```
cli/         CLI sqlite-cms（Rust）。利用者が使うのはこれだけ
web/         SPA（Preact + Vite + Tailwind）。ビルド結果は CLI に埋め込まれる
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

例外として、「状態」の節（`## 状態` の行から、次の `## ` で始まる行の手前まで）だけは、コミットした後も書き換えてよい。
提案として書いた ADR を承認して「採用」にするときなどに使う。

この決まりは、二つのフックが同じ判断（`.githooks/check-adr.sh`）で守る。
HEAD にある ADR が、状態の節のほかで書き換えられたり、消されたり、名前を変えられたりしていれば、問題とみなす。

Claude Code で作業するときは、`.claude/settings.json` の PostToolUse のフック（`.claude/hooks/check-adr-immutable.sh`）が、ツールを使うたびに作業ツリーを HEAD と比べる。
問題があれば、エラーにして取り消すよう伝える。
編集のツールだけでなく、シェルのコマンドでの書き換えも見つける。
ただし、書き換えの後に知らせる仕組みなので、書き換え自体は止めない。

コミットは、git の pre-commit フック（`.githooks/pre-commit`）で止める。
コミットする中身（インデックス）を HEAD と比べ、問題のあるコミットを拒む。
状態の節だけを変えるコミットと、新しい ADR を足すコミットは通る。
フックはリポジトリで管理しているが、git は自動では使わないので、クローンしたら一度だけ次を実行して有効にする。

```sh
git config core.hooksPath .githooks
```

有効になっているかは `git config --get core.hooksPath` で確かめられる（`.githooks` と出る）。

## 依存の方針

依存は必要なものに絞る（ADR 0013、0014）。
数十行で書けるものは自前で実装し、ライブラリを足さない。

- `web/`：ルーター（`web/src/router.tsx`）と SQLite の DB の読み手（`web/src/sqlite.ts`、ADR 0047）を自前にしている。SPA は sql.js を使わない。sql.js は、読み手が SQLite と同じ行を読むことを確かめるテストでだけ使う（開発用の依存）。その型（`web/src/types/sql.js.d.ts`）も自前で、テストで sql.js の API を新しく使うときは型の宣言も足す。
- `web/`：シンタックスハイライトは Shiki（ADR 0017）。言語は `web/src/highlight.ts` の `LANGUAGES` に登録したものだけが色付きになる。言語を足すときは、`@shikijs/langs/<言語>` を登録し、`web/src/highlight.test.tsx` の `SAMPLES` に見本を足し（足さないとテストが落ちる）、`web/src/highlightLanguages.ts` の `HIGHLIGHT_LANGUAGES` に言語名と別名を足し（足さないとテストが落ちる。Shiki を読み込むかをこの一覧で決める、ADR 0029）、README の対応表を更新する。Shiki は、色を付ける言語のコードブロックが画面の近くに来たときに `web/src/highlightLoader.ts` が起動する Web Worker（`web/src/highlight.worker.ts`）の中で動き、正規表現は Oniguruma（wasm）で照合する（ADR 0038、0046）。`web/src/highlight.ts` と Shiki のモジュールを本体のコードから静的に import しない（すると本体に取り込まれる）。登録した言語は、Oniguruma のエンジンで色分けできることをテストで確かめている。1 言語で gzip 後 1〜16 KB 増えるので、`web/vite.config.ts` の `chunkSizeWarningLimit` も必要に応じて見直す。
- `web/`：Markdown の拡張の記法（ADR 0025）は、micromark の構文拡張と mdast の書き換えを `web/src/markdown/` に自前で書く。remark や rehype のプラグインのライブラリは足さず、再実装が現実的でない描画のライブラリ（KaTeX、mermaid）だけを使う。KaTeX と mermaid は `web/src/render/loaders.ts` 経由で非同期に読み込む別チャンクなので、`web/src/render/katex.ts` と `web/src/render/mermaid.ts` を本体のコードから静的に import しない。KaTeX のフォントは `web/vite.config.ts` のプラグインで woff2 だけを残す。
- `web/`：SPA は相対のパス（Vite の `base: "./"`）でビルドする。どのパスに置いても読めるよう、CLI が `index.html` のパスを `site.toml` の `base_path` に合わせて書き換え、SPA は `<meta name="sqlite-cms-base">` を読んでパスを組み立てる（ADR 0030）。SPA の中でサイト内のパスを URL にするときは `web/src/base.ts` の `withBasePath` を通す。
- `web/`：本文の HTML は自前の rehype のプラグイン（`web/src/markdown/rawHtml.ts`、ADR 0046）で取り込み、`rehype-sanitize` で `web/src/sanitize.ts` の許可一覧以外を取り除く（ADR 0018）。許可するタグを足すときは `EXTRA_TAGS` に足し、`web/src/sanitize.test.tsx` にテストを足す。sanitize は Markdown が作る要素（脚注、タスクリスト、コードブロックの言語名）も検査するので、許可一覧から外すと Markdown の出力が壊れる。Shiki の色分けは sanitize の後に、Web Worker の中で行う（ADR 0046）。
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

### E2E

```sh
npm --prefix web run build   # SPA を最新にしてから
npm --prefix web run e2e
```

`sqlite-cms serve` で `example/` を配信し、ヘッドレスの Chrome を Chrome DevTools Protocol で動かして、ブラウザでの動きを確かめる（`web/scripts/e2e.mjs`、ADR 0044）。
計測するすべてのページがエラー（コンソールのエラー、400 以上の応答）なく描かれ、レイアウトがずれないこと、色分け、数式、図、画面の中の移動、ヘッダの検索と候補、カスタムのヘッダと既定のヘッダを確かめる。
既定のヘッダは、`example/` から `content/header.md` を除いた写しを一時ディレクトリに作って確かめる（`.env` は写さない）。
Lighthouse と同じく、手元に Chrome が要り、`SQLITE_CMS_BIN` でビルド済みのバイナリを使える。
jsdom などの DOM の代わりになるライブラリは入れず、描画を伴う動きはこの E2E で確かめる。

`serve` や Lighthouse の計測で見本を配信すると、リンクカードの画像を取得して `example/.sqlite-cms-cache/` に保存する（Git には入れない）。

## SPA の開発

```sh
npm --prefix web run dev
```

`sqlite-cms build --data-only` で DB と画像を `web/public` に書き出してから、Vite の開発サーバを起動する。
SPA は Preact で描くので（ADR 0039）、部品を変えると状態を保った差し替え（Fast Refresh）ではなく、ページ全体を読み直す。
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
3. E2E（上記）。`cargo test` が作った `target/debug/sqlite-cms` で配信する
4. Lighthouse の計測（下記）。3 つの Chrome で並行して計測する。レポートは成功しても失敗しても `lighthouse-reports` という Artifact に保存し、スコアの表は実行結果の概要に出す

一つのジョブで、速く終わる検査から順に行う（ADR 0044）。
`docs/`、`README.md`、`DEVELOPMENT.md` だけを変えたときは動かさない。
同じ PR に続けて push したときは、古い実行を取り消す。`main` への push は取り消さない。

## Lighthouse

```sh
npm --prefix web run build   # SPA を最新にしてから
npm --prefix web run lighthouse
```

`sqlite-cms serve` で `example/` を配信し、代表的な 8 ページと、重いページと複雑なページの見本 6 ページ（`web/scripts/pages.mjs` の `PAGES`。E2E も開く）をヘッドレスの Chrome で計測する（ADR 0019）。
手元に Chrome が要る。
レポート（HTML と JSON）は `web/lighthouse-reports/` に日本語で出る。
配信には既定で `cargo run` を使う。ビルド済みのバイナリで計測するときは、`SQLITE_CMS_BIN` にそのパスを渡す。
`LIGHTHOUSE_CONCURRENCY` に 2 以上を渡すと、ページを分けて、その数の Chrome で並行して計測する（ADR 0044。CI は 3）。速くなるが、CPU を取り合うのでパフォーマンスの点数は揺れる。パフォーマンスを比べるときは、既定の 1 で計測する。

アクセシビリティとベストプラクティスは 100 点でなければ失敗し、足りない項目と要素を表示する（ADR 0024）。
ほかの項目（パフォーマンス、SEO）は計測して記録するだけで、基準はまだ決めていない。

`sqlite-cms serve` は、本番の Cloudflare と同じく応答を gzip で圧縮する（ADR 0026、0038）。圧縮は自前の実装で、大きさは zlib とほぼ同じになる。本番の Cloudflare は Brotli で配信するので、パフォーマンスの値は本番より少し悪く出る。
計測ごとに数点の揺れがある。パフォーマンスを改善したときは、改善の前後を何度か計測して比べる。
基準を足すときは `REQUIRED` に書く。

## リリース

`v*` のタグを push すると、`.github/workflows/release.yml` が次を行う。

1. SPA をビルドし、テストを実行する。テストはリリースと同じ出力先（`--target`）で行い、続くビルドでそのまま使う（ADR 0044）
2. Linux x86_64 と macOS arm64 のバイナリをビルドする
3. Linux のビルドで作ったバイナリで E2E を行い、Lighthouse を計測して、レポートを `lighthouse-reports` の Artifact に保存する。E2E が失敗するか、アクセシビリティかベストプラクティスが 100 点を割ると、ここで失敗して Release は作られない
4. `sqlite-cms-<target>.tar.gz` を GitHub Release に添付する

タグを付けずに試すときは、`gh workflow run release.yml` で手動実行する。ビルドと梱包までを行い、Release は作らない（成果物は実行結果の Artifacts から取れる）。

記事リポジトリの公開ワークフロー（雛形は `example/.github/workflows/deploy.yml`）は、最新のリリースからバイナリを取得する。
