# ADR 0067: シンタックスハイライトの言語を増やし、ほかの言語は使うときに読み込む

ステータス: 採択

## 文脈

コードブロックの色分けは Shiki で行う（[ADR 0017](0017-Shikiによるシンタックスハイライト.md)）。
色を付けられる言語は 15（css、diff、haskell、html、javascript、json、markdown、python、rust、shellscript、sql、toml、tsx、typescript、yaml）で、`web/src/highlight.ts` で文法を直接 import している。
文法は Shiki と同じチャンクに入り、色を付けるコードブロックが画面の近くに来たときに、Worker の中で読み込む（[ADR 0029](0029-必要になったときの読み込み.md)、[0038](0038-表示の速さの改善.md)、[0046](0046-メインスレッドを空ける.md)）。
Shiki を読み込む前に色を付けるかを判断するため、言語名と別名の一覧を本体に別に持ち（`web/src/highlightLanguages.ts`）、手で同期している。

Go、C、Java、Kotlin、Swift など、よく書かれる言語の多くに色が付かない。
コードの埋め込み（[ADR 0065](0065-embed-github-files-and-gists.md)）では、ほかのリポジトリのファイルを表示するので、色の付かない言語に当たることが多い。
また、拡張子のないファイル（`Dockerfile`、`Makefile` など）には、言語を決められない。

文法の大きさ（Shiki 4.4、gzip 後。中に含む言語を含む）は、言語によって大きく違う。

- 今の 15 言語：合わせて約 108 KB。
- 小さいもの：go 5 KB、java 4 KB、kotlin 2 KB、zig 1.5 KB など。
- 大きいもの：ruby 280 KB（haml、jsx、graphql、cpp、lua などを含む）、php 106 KB、cpp 77 KB、vue 137 KB、julia 114 KB、typst 1.6 MB など。
- Shiki の全 242 言語：合わせて約 1.3 MB。

## 実装すること

### いつも読み込む言語を 50 にする

Shiki と同じチャンクに入れる言語（以下、いつも読み込む言語）を、今の 15 に次の 35 を足した 50 にする。
文法が小さく、よく書かれるものを選んだ。合わせて gzip 後で約 234 KB（今より約 126 KB 増える）。

| 種類 | 足す言語（Shiki の id） |
|---|---|
| 汎用 | go、c、java、kotlin、swift、csharp、scala、dart、zig、lua、groovy |
| 関数型 | ocaml、fsharp、clojure、r |
| シェルと設定 | powershell、bat、fish、docker、make、cmake、nix、hcl、terraform、ini、xml、proto、prisma |
| Web | jsx、scss、less、sass、jsonc |
| そのほか | solidity、gleam |

### ほかの言語は使うときに読み込む

Shiki の残りの言語（約 190。ruby、php、cpp、elixir、vue、svelte、graphql、latex など）は、言語ごとに別のチャンクにする。
ページにその言語のコードブロックがあるときだけ、Worker が、そのコードブロックを色付けする前に読み込む（Shiki の `loadLanguage`）。
中に別の言語を含む文法（vue の html、css、js など）は、Shiki の言語のモジュールが依存ごと返すので、まとめて読み込む。
一度読み込んだ言語は、同じページの間、読み込み直さない。読み込めなかったときは、そのコードブロックを色なしのままにする（今の、知らない言語と同じ）。

### 言語名と別名の一覧を、Shiki から作る

- Shiki の言語の id と別名（`shiki/langs` の `bundledLanguagesInfo`、約 350）から、本体に置く一覧（`web/src/highlightLanguageNames.generated.ts`）を、スクリプト（`npm run gen:languages`）で作り、コミットする。本体には名前だけを置き、文法の読み込みの関数は置かない。
- `needsHighlight` は、この一覧でいつも読み込む言語と、使うときに読み込む言語の両方を判断する。手で同期する今の一覧（`HIGHLIGHT_LANGUAGES`）はやめる。
- 単体テストで、生成した一覧が今の Shiki と一致することを確かめる。Shiki を上げて言語が変わったら、テストが落ちて、作り直しを知らせる。
- `diff 言語`（[ADR 0025](0025-Markdownの拡張.md)）の言語にも、使うときに読み込む言語を使える。

### ファイル名から言語を決める（コードの埋め込み）

コードの埋め込みで使う、ファイル名から言語を決める対応（`languageOfFile`）を、同じ一覧から作る。

1. ファイル名そのものの対応表で決める（大文字と小文字を区別する）。`Dockerfile` → docker、`Makefile`・`GNUmakefile` → make、`CMakeLists.txt` → cmake、`Gemfile`・`Rakefile` → ruby、`.bashrc`・`.zshrc`・`.profile` → shellscript、`flake.lock` → json など。
2. 拡張子が、Shiki の言語の id か別名に当たれば、その言語にする（`rs`、`py`、`go`、`kt` など）。
3. 拡張子の対応表で決める。id と別名にない拡張子だけを書く（`h` → c、`hpp`・`cc` → cpp、`cs` → csharp、`ex`・`exs` → elixir、`tf` → terraform、`htm` → html、`yml` → yaml など）。
4. どれにも当たらなければ、色を付けない。

対応表は `web/src/highlightLanguages.ts` に手で書き、表のすべての言語が一覧にあることを単体テストで確かめる。

### 文書

- README の、色を付けられる言語の説明を改める（いつも読み込む言語と、それ以外も使えること、`Dockerfile` などのファイル名）。
- specifications.md のコンテンツフォーマットに書き足す。

## 実装しないこと

- Shiki の言語の一覧にない言語の文法を、自前で足すこと。
- 言語ごとに、どの別名を受け付けるかを変えること（Shiki の別名をそのまま使う）。
- テーマの変更（[ADR 0019](0019-Lighthouseの計測とアクセシビリティ.md) のコントラストを保つテーマのまま）。
- 使うときに読み込む言語を、ページを開いたときに先読みすること（`modulepreload`）。
- いつも読み込む言語を、見本の記事に合わせて決めること。

## テスト設計

- 単体テスト（vitest）
  - 生成した言語名の一覧が、`bundledLanguagesInfo` の id と別名と一致すること。
  - `needsHighlight`：いつも読み込む言語、使うときに読み込む言語（`ruby`、`php`）、その別名（`rb`）、`diff-ruby` を、色を付けるものと判断すること。知らない言語と `mermaid` を、色を付けないと判断すること。
  - 色分け（テストでは同じスレッドで行う）：いつも読み込む言語（`go`）と、使うときに読み込む言語（`ruby`）の両方に色が付くこと。使うときに読み込む言語を、二度目は読み込み直さないこと。
  - `languageOfFile`：ファイル名の表（`Dockerfile`、`Makefile`、`.zshrc`）、id と別名に当たる拡張子（`main.go`、`a.rb`）、拡張子の表（`a.h`、`a.cs`、`main.tf`）、当たらないもの（`LICENSE`、`a.unknown`）。表のすべての言語が一覧にあること。
- E2E
  - 見本の「記法の一覧」に、いつも読み込む言語（Go）と、使うときに読み込む言語（Ruby）のコードブロックを足し、どちらにも色が付くこと。Ruby の文法のチャンクは、Ruby のあるページだけで読み込むこと。
  - 既存の場面（コードのないページで Shiki を読まない、など）が通ること。
- 大きさ：実装の前と後で、Shiki のチャンクの大きさ（gzip 後）、`web/dist` の大きさ、リリースのバイナリの大きさを測り、PR に書く。Lighthouse と Core Web Vitals が CI で通ること。

## トレードオフ

- いつも読み込む言語の文法が約 126 KB（gzip 後）増える。コードブロックのあるページで、最初に色が付くまでが遅くなる。コードのないページには影響しない（Shiki を読まない）。
- 使うときに読み込む言語は、そのコードブロックに色が付くまでに、取得が一回増える。
- すべての言語の文法を SPA に含めるので、`web/dist` とリリースのバイナリが大きくなる（文法は合わせて圧縮前で約 9 MB、gzip 後で約 1.3 MB）。配信先に置くファイルも増える。
- 言語名の一覧（約 350 の名前）が本体に入るので、本体が少し大きくなる。
- Shiki を上げるたびに、一覧を作り直す手間がある（テストが知らせる）。
