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

## リリース

`v*` のタグを push すると、`.github/workflows/release.yml` が次を行う。

1. SPA をビルドし、テストを実行する
2. Linux x86_64 と macOS arm64 のバイナリをビルドする
3. `sqlite-cms-<target>.tar.gz` を GitHub Release に添付する

記事リポジトリの公開ワークフロー（雛形は `example/.github/workflows/deploy.yml`）は、最新のリリースからバイナリを取得する。
