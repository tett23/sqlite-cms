# 0039 Preact への置き換え

## 状態

提案。
0002 の「Vite + React + TypeScript の SPA とする」を、React の代わりに Preact を使う形に改める。
0038 で実装しないことにした「React を小さなライブラリ（Preact など）に置き換えること」を実装する。

## 決定

SPA の描画に、React（`react`、`react-dom`）の代わりに Preact（`preact` の `preact/compat`）を使う。

0038 の計測で、残る減点のほとんどは Largest Contentful Paint で、本文を描くのに要る JS と wasm の転送量で決まっていた。
本体の JS（gzip 後で約 190 KB）のうち、最も大きいのは react-dom だった。
Preact は React とほぼ同じ API を持ち、`preact/compat` を通せば react-markdown もそのまま動く。

### 置き換え方

- ソースは `react` から import したままにし、ビルドのときに別名で `preact/compat` に差し替える（`web/vite.config.ts` の `resolve.alias`）。react-markdown が内部で `react/jsx-runtime` を使うので、どちらにしても別名は要る。ソースだけ `preact` から import するように書き換えると、書き方が二通りになる。
- 型は `web/tsconfig.json` の `paths` で Preact のものを使う。React の型のままだと、実際に動く Preact との違い（下記）を型で検出できない。
- テストも Preact で描く。サーバーでの描画は `preact-render-to-string` を使い、react-markdown は Vitest が変換する対象に入れる（外部のモジュールのままだと、別名が効かずに React で描かれる）。
- `@vitejs/plugin-react` は外す。JSX の変換は Vite 自体が行う。このプラグインの Fast Refresh は React 専用で、Preact では動かない。

依存は、`react`、`react-dom`、`@types/react`、`@types/react-dom`、`@vitejs/plugin-react` を外し、`preact`（実行時）と `preact-render-to-string`（テスト）を足す。
`react` は react-markdown の peer 依存として `node_modules` に入るが、別名で差し替えるのでバンドルには入らない。

### React との違いで直したもの

- `useSyncExternalStore` は、サーバー用の値を返す関数（3 番目の引数）を取らない。サーバーでの描画でも 2 番目の関数を使う。
- イベントの `target` の型は `EventTarget` なので、入力欄の値は `currentTarget` から読む。
- 要素の属性の型は Signal も受け付けるので、文字列として使う前に型を確かめる。
- サーバーで描いた HTML の書き方が違う。`class` を最後に書き、値のない属性は `=""` を付けず（`alt`、`disabled`）、文字列の中の `>` をエスケープしない。どれも HTML として同じ意味なので、テストの期待値をこの書き方に合わせる。

### 計測結果

本体の JS は、gzip 後で約 190 KB から約 133 KB になった（圧縮前で 621 KB から 421 KB）。

Lighthouse のパフォーマンスは次のとおり変わった（手元の Apple Silicon、`serve` で配信、2 回計測してどちらも同じか 1 点差）。
ほかの三つの分類は、変更の前後とも全ページで 100 点である。

| ページ | React（0038） | Preact |
|---|---:|---:|
| `/` | 86 | 88 |
| `/articles/syntax` | 87 | 90 |
| `/articles/extensions` | 83 | 85〜86 |
| `/articles/getting-started` | 80〜82 | 84〜85 |
| `/posts/ruby` | 83 | 86 |
| `/about` | 88 | 91 |
| `/search?q=記事` | 88 | 90 |
| `/archive` | 88 | 91 |

Largest Contentful Paint は 3.8 秒から 3.5 秒に、First Contentful Paint は 1.8 秒から 1.7 秒になった。
Total Blocking Time はほとんど変わらない。長いタスクは、Shiki の最初の色分けと本文の Markdown の変換で、React の処理ではなかった。

ブラウザ（ヘッドレスの Chrome）で、次のことを確かめた。

- すべての計測対象のページが描かれ、読み込み中の表示が消えること。
- コードブロックに色が付き、数式と図が描かれ、リンクカードの画像が出ること。
- リンクでページを移り、ブラウザの戻るで戻れること。
- 検索の入力で結果が変わり、URL に言葉が残ること。
- `npm run dev`（Vite の開発サーバー）でも描かれること。

## 実装しないこと

- ソースの import を `preact` と `preact/hooks` に書き換えること。別名で足りる。
- Preact 用の開発サーバーのプラグイン（`@preact/preset-vite`。部品を変えたときにページ全体を読み直さずに差し替える）。依存が増えるので、ページ全体の読み直しで済ませる。
- `preact/compat` を通さず Preact 本体だけで動かすこと。react-markdown が React の API を使う。

## テスト設計

- 既存のテストをすべて Preact で描いて通すこと（描いた HTML の書き方だけを期待値に合わせる）。
- 型の検査を Preact の型で通すこと。

## トレードオフ

- 開発サーバーで部品を変えると、状態を保ったままの差し替え（Fast Refresh）ではなく、ページ全体を読み直す。
- React にしかない機能（Server Components、`use` など）は使えない。この SPA では使っていない。
- react-markdown などの依存が React の新しい機能を使うようになると、`preact/compat` で動かなくなるおそれがある。依存を更新するときは、テストとブラウザでの確認で気付く。
