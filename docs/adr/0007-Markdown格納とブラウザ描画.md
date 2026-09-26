# 0007 Markdown 格納とブラウザ描画

## 状態

採用。
0001 の「Markdown → HTML 変換はビルド時に行い、DB には変換済み HTML だけを入れる」と、0006 の「pulldown-cmark で変換する」を置き換える。

## 決定

DB には本文の Markdown を加工せずに格納し、HTML への変換はブラウザで行う。

### スキーマ

マイグレーション `0003_markdown_body.sql` で、`posts`、`articles`、`pages` の `body_html` 列を `body_md` に改名する。
DB はビルドごとに作り直すので、既存の HTML を Markdown へ戻すデータ変換は要らない。

### ビルダー

`build-db` は frontmatter の分離と検証だけを行い、本文は frontmatter の区切り以降の文字列をそのまま格納する。
Markdown の変換器（pulldown-cmark）への依存を外す。

### 描画

- react-markdown（unified、remark-parse、remark-rehype による remark ベースの描画）に remark-gfm を組み合わせる。
- 対応する GFM の記法は、表、取り消し線、タスクリスト、自動リンク、脚注である。
- 脚注の見出しは「脚注」とし、表示する（remark-rehype の既定では `sr-only` で隠れるが、このサイトは Tailwind にそのクラスを生成させていない）。
- 本文中の `/` で始まるリンクは react-router の `Link` で描画し、遷移で再読み込みを起こさない。
- 描画は `MarkdownBody` コンポーネントにまとめ、post、article、page の三つの本文ページが共有する。

## 採用の理由

- **全文検索との相性**：FTS5 を入れるとき、索引に HTML のタグや属性が混ざらない。
- **描画と内容の分離**：記法の対応や見た目の変更は SPA の変更だけで済み、ビルダーは内容の検証に専念できる。
- **ビルダーの単純化**：Rust 側から Markdown の方言の選択がなくなり、描画規則の実装が remark の一か所にまとまる。

## 実装しないこと

- 生の HTML の描画（rehype-raw）。react-markdown の既定どおり、本文中の HTML は描画しない。必要になった時点で、サニタイズを含めて ADR を追加する。
- シンタックスハイライト。コードブロックは言語名のクラスを付けた `pre` のまま表示する。
- 描画結果のキャッシュ。一本の本文を描画する時間は、記事の規模では体感できない。

## テスト設計

- `cargo test`：本文が加工されずに格納されること、DB の `body_md` に Markdown が入ること、マイグレーション 0003 が適用・記録されること。
- vitest（`MarkdownBody.test.tsx`）：`react-dom/server` で静的に描画し、見出しと強調、GFM の五つの記法、サイト内外のリンクの href、生の HTML が要素にならないことを検証する。
- vitest（`db.test.ts`）：クエリが `body_md` の Markdown をそのまま返すこと。
- 本文中のサイト内リンクで再読み込みが起きないことは、ブラウザで確認する。

## トレードオフ

- JS バンドルが約 48KB（gzip 後）増える。初回に一度だけ払うコストであり、以後の遷移には影響しない。
- 描画が閲覧のたびに実行される。変換を一度で済ませていたビルド時変換と比べると無駄だが、記事一本の変換は短く、DB と wasm の取得に比べれば無視できる。
- 0001 のもとでは本文中の生の HTML が通っていた。この変更で表示されなくなるため、既存の記事に HTML を書いていた場合は Markdown に書き直す必要がある（現在の記事にはない）。
