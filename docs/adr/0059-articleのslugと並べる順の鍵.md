# 0059 article の slug と並べる順の鍵

## 状態

提案。
0058 の「post と article の URL は常に UUIDv7」を、article については frontmatter の slug を優先する形に改める。

## 決定

article は、frontmatter に `slug` を書けば、それを URL（`/articles/<slug>`）にする。書かなければ、0058 のとおりファイル名から取った slug（UUIDv7 か、ファイル名）にする。
post と固定ページは変えない（post の URL は UUIDv7 かファイル名、固定ページはファイル名）。

- `sqlite-cms new article <slug> --title …` は、ファイル名を `<UUIDv7>-<slug>.md` にし、frontmatter に `slug: <slug>` を書く。ファイル名の後ろの名前だけでは、指定した slug とタイトルから作った名前を区別できないので、frontmatter に書く。slug を省いたときは書かない。
- frontmatter の slug は、文字、数字、`-`、`_` だけを使える。ほかの文字があればエラーにする。
- ほかの article と slug が重なれば、両方のファイル名から取った名前を挙げてエラーにする。

### 並べる順の鍵

slug を書き手が決められるので、slug の順では作った順に並ばない。
そこで、ファイル名から取ったもの（UUIDv7 か、ファイル名）を並べる順の鍵にし、DB の `posts` と `articles` に `sort_key` の列を足す（マイグレーション 0010）。

- 一覧（post、article、統合一覧）と RSS は、日付の新しい順に並べ、同じ日付の中は鍵の降順にする。UUIDv7 なら、後から作ったものが先になる。
- `sort_key` の列のない DB（列を足す前の sqlite-cms で作ったもの）では、slug で並べる。

### 見本

見本の article に、今までの名前を frontmatter の slug として書き、今までの URL（`/articles/syntax` など）に戻した。並びは、ファイル名の UUIDv7（記事の日付から作ったもの）で決まり、今までと同じになる。

## テスト設計

- frontmatter の slug を URL にし、並べる順の鍵はファイル名から取ったものにすること。使えない文字のある slug をエラーにすること。
- `new article <slug>` が frontmatter に slug を書き、slug を省けば書かないこと。post には書かないこと。
- DB：article の slug と `sort_key` が入ること。post の `sort_key` が slug と同じになること。frontmatter の slug が重なればエラーにすること。
- SPA：同じ日付の記事を、slug ではなく `sort_key` の降順に並べること（post と article の一覧、統合一覧）。`sort_key` のない DB では slug で並べること（既存のテスト）。
- RSS：同じ日付の article を、slug ではなく鍵の順に並べること。
- E2E、Lighthouse、Core Web Vitals：見本の article が今までの URL で開けること。

## トレードオフ

- article の URL を書き手が決められる分、URL の重なりに気を付ける必要がある（ビルドで知らせる）。
- ファイル名を変えると（UUIDv7 を変えると）、並びが変わる。slug を書いていれば、URL は変わらない。
