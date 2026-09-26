# 0001 記事フォーマットと SQLite 生成

## 状態

採用。

## 決定

記事は `modules/blog/articles/*.md` に置き、frontmatter（YAML）でメタデータを持つ。
ビルドスクリプト `scripts/build-db.ts` が全記事を読み、単一の SQLite ファイルを生成する。

### 記事フォーマット

- ファイル名（拡張子を除く）を slug とする。
- frontmatter の必須キーは `title` と `date`（`YYYY-MM-DD`）。
- 不明なキーは無視する。必須キーの欠落はビルドエラーとする。

### スキーマ

```sql
CREATE TABLE articles (
  slug         TEXT PRIMARY KEY,
  title        TEXT NOT NULL,
  published_at TEXT NOT NULL,  -- YYYY-MM-DD
  body_html    TEXT NOT NULL
);
```

### 生成

- Markdown → HTML 変換はビルド時に行う（marked を使用）。DB には変換済み HTML だけを入れる。
- DB の生成には sql.js を使う。ランタイムの読み手と同じ実装であり、ネイティブ依存（better-sqlite3）を持ち込まないためである。
- 生成物は `public/db/articles-<hash>.sqlite`（hash は DB バイト列の SHA-256 先頭 16 桁）と、無キャッシュで配る `public/db/manifest.json`（`{"db": "/db/articles-<hash>.sqlite"}`）の二つ。
- 生成前に `public/db/` 内の旧 DB を削除する。

## 実装しないこと

- タグ、カテゴリ、下書きフラグ。必要になった時点で ADR を追加する。
- FTS5 の仮想テーブル。スキーマ拡張は後方互換に行えるため、先取りしない。
- 画像の BLOB 格納。画像は `public/images/` の静的ファイルとする。

## テスト設計

vitest でビルドスクリプトを直接テストする。

- フィクスチャの Markdown 数件から DB を生成し、sql.js で開いて行数、slug、title、HTML 変換結果を検証する。
- 必須キー欠落の記事でビルドがエラーになることを検証する。
- 同一入力から同一ハッシュのファイル名が得られることを検証する。

## トレードオフ

- sql.js での DB 生成は better-sqlite3 より遅いが、記事数百件の規模では問題にならない。ネイティブビルド不要の利点を取る。
- HTML をビルド時に焼くため、Markdown レンダラの変更は全記事の再生成を意味する。単一 DB を毎回作り直す方式なので、追加コストはない。
