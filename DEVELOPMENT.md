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

## 依存の方針

依存は必要なものに絞る（ADR 0013、0014）。
数十行で書けるものは自前で実装し、ライブラリを足さない。

- `web/`：ルーター（`web/src/router.tsx`）と sql.js の型（`web/src/types/sql.js.d.ts`）を自前にしている。sql.js の API を新しく使うときは、型の宣言も足す。
- `web/`：シンタックスハイライトは Shiki（ADR 0017）。言語は `web/src/highlight.ts` の `LANGUAGES` に登録したものだけが色付きになる。言語を足すときは、`@shikijs/langs/<言語>` を登録し、`web/src/highlight.test.tsx` の `SAMPLES` に見本を足し（足さないとテストが落ちる）、README の対応表を更新する。Shiki は `web/src/highlightLoader.ts` 経由で非同期に読み込む別チャンクなので、`web/src/highlight.ts` と Shiki のモジュールを本体のコードから静的に import しない（すると本体に取り込まれる）。1 言語で gzip 後 1〜16 KB 増えるので、`web/vite.config.ts` の `chunkSizeWarningLimit` も必要に応じて見直す。
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

`sqlite-cms serve` で `example/` を配信し、代表的な 5 ページ（`web/scripts/lighthouse.mjs` の `PAGES`）をヘッドレスの Chrome で計測する（ADR 0019）。
手元に Chrome が要る。
レポート（HTML と JSON）は `web/lighthouse-reports/` に出る。

アクセシビリティは 100 点でなければ失敗し、足りない項目と要素を表示する。
ほかの項目（パフォーマンス、ベストプラクティス、SEO）は計測して記録するだけで、基準はまだ決めていない。
基準を足すときは `REQUIRED` に書く。

## リリース

`v*` のタグを push すると、`.github/workflows/release.yml` が次を行う。

1. SPA をビルドし、テストを実行する
2. Linux x86_64 と macOS arm64 のバイナリをビルドする
3. `sqlite-cms-<target>.tar.gz` を GitHub Release に添付する

タグを付けずに試すときは、`gh workflow run release.yml` で手動実行する。ビルドと梱包までを行い、Release は作らない（成果物は実行結果の Artifacts から取れる）。

記事リポジトリの公開ワークフロー（雛形は `example/.github/workflows/deploy.yml`）は、最新のリリースからバイナリを取得する。
