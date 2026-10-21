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
- C コンパイラ（リンクカードの画像を WebP にする libwebp を、ビルドのときに一緒にビルドする。[ADR 0049](docs/adr/0049-リンクカードのWebPと後からの読み込み.md)。macOS は Xcode の Command Line Tools）
- Node 22 以降
- [adr-guard](https://github.com/tett23/adr-guard)（コミットするとき。下の「ADR」）

型検査には、Go で実装された TypeScript 7 の `tsc` を使う（[ADR 0012](docs/adr/0012-TypeScript7による型検査.md)）。
npm がプラットフォーム用のバイナリを自動で入れるので、Go を別に入れる必要はない。
7 系は JavaScript のコンパイラ API を持たないので、TypeScript の API に依存するツールはそのままでは使えない。

## ADR

設計の判断は `docs/adr/` に、連番の ADR として書く（[ADR 0064](docs/adr/0064-align-adr-conventions-with-global-rules.md)）。

- 実装に入る前に、ADR を書く。書いた ADR はコミットせず、レビューを受ける。承認されたら、ステータスを「採択」にしてコミットする。
- ファイル名は `0064-align-adr-conventions-with-global-rules.md` のように、連番の後を、内容が分かる英語の kebab-case にする。
- 中身は次の形で書く。データの構造やアルゴリズムを変えるときは、その節も立てる。

```markdown
# ADR 0064: タイトル

ステータス: 提案

## 文脈

## 実装すること

## 実装しないこと

## テスト設計

## トレードオフ
```

- ステータスは、提案、採択、採択（未実装）、採択→変更後の ADR 番号、却下、破棄など。採択したがまだ実装していない ADR は「採択（未実装）」にし、`adr-guard unimplemented` で一覧できる。ステータスの行（行頭が `ステータス:` の行）は、どの ADR にもちょうど 1 つ置く。
- コミットした ADR は、ステータスのほかは書き換えない。判断を改めるときは、次の番号で新しい ADR を書き、どの ADR のどの判断を改めるかをそこに書く。
- ADR で仕様が変わったら、`docs/specifications.md` と `README.md` も同じときに直す。
- ADR を破棄するときは、ステータスを「破棄」にして残し、理由を `docs/specifications.md` の「破棄した ADR」に書く。

[0063](docs/adr/0063-loose-list-paragraph-spacing.md) より前の ADR は、日本語のファイル名と、題名と「状態」の節（`## 状態`）の古い形で書いてある。
adr-guard（下記）が、コミットした ADR の名前の変更と、`ステータス:` の行のほかの変更を拒むので、古い形のまま残す。
古い形の ADR にも、題名の行の次に `ステータス:` の行を足してあり、今のステータスはこの行にある（[ADR 0072](docs/adr/0072-adr-guard-0-8-and-status-lines.md)）。
状態の節は書いたときのままなので、今と合わないもの（[0038](docs/adr/0038-表示の速さの改善.md) から [0062](docs/adr/0062-開発サーバーのDBの書き出し直し.md) の「提案。」）がある。読み方は `docs/specifications.md` の「古い形の ADR」にある。

コミットした ADR の書き換えは、[adr-guard](https://github.com/tett23/adr-guard)（v0.8.3）で止める（[ADR 0064](docs/adr/0064-align-adr-conventions-with-global-rules.md)、[0068](docs/adr/0068-adr-guard-0-4-and-adr-links.md)、[0069](docs/adr/0069-adr-guard-0-5-and-allow-status.md)、[0072](docs/adr/0072-adr-guard-0-8-and-status-lines.md)、[0073](docs/adr/0073-adr-guard-0-8-1-and-directory-name-variants.md)、[0075](docs/adr/0075-adr-guard-0-8-2-and-link-only-changes.md)、[0076](docs/adr/0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md)）。
adr-guard は、コミットした ADR の変更、削除、名前の変更を、すべて違反とする。
新しい ADR を足すことと、まだコミットしていない ADR の編集はよい。
ただし、`ステータス:` で始まる行の変更と、ステータスの行がなかった ADR への 1 行の追加（`--allow-status`）と、リンクだけの変更（ADR の参照などのリンクを付ける、外す、リンク先を変える、リンクの参照定義の行を変える。リンクのテキスト、画像の追加と削除、自動リンクや本文の URL の変更は許さない。`--allow-links`）は許す。
ADR は、行頭が `ステータス:`（または `Status:`）のステータスの行を、ちょうど 1 つ持たなければならない。ないものと 2 つ以上あるものは違反である（`adr-guard check-status` で調べられる）。
`docs/audit/`、`docs/audits/`、`docs/benchmarks/`、`docs/benchmark/`（どの深さの `docs/` の下でも）に置いた監査や計測の記録も、コミットした後の変更、削除、名前の変更を違反とする。記録には `--allow-status` と `--allow-links` が効かない。今のこのリポジトリには、これらのディレクトリはない。

文書（`.md`）の中の ADR の参照（`ADR 0001` の形）には、ADR のファイルへのリンクを付ける（`[ADR 0001](docs/adr/0001-….md)`、続く番号は `[0002](…)`）。
インラインのコード、コードブロック、リンクのテキストの中と、ADR 自身の番号は対象外である。
リンクのない参照は `adr-guard links` で見つかり、`adr-guard links --fix` で直せる（コミット済みの ADR も直すときは `--include-committed` を付ける）。

- git の pre-commit フック（`.githooks/pre-commit`）：`adr-guard pre-commit --allow-status --allow-links` で、コミットする中身（インデックス）を HEAD と比べ、違反のあるコミットを拒む。ステージした文書の、リンクのない ADR の参照と、ステージした ADR のステータスの行がちょうど 1 つでないものも拒む。
- Claude Code のフック：`adr-guard claude-hook --allow-status` を、利用者の全体の設定（`~/.claude/settings.json` の PreToolUse、matcher は `Write|Edit|MultiEdit`）で動かす。リポジトリの `.claude/settings.json` には置かない（[ADR 0068](docs/adr/0068-adr-guard-0-4-and-adr-links.md)）。シェルのコマンドでの書き換えは見ないので、pre-commit フックと CI で止める。
- CI（`.github/workflows/adr.yml`）：`main` への push と PR で、`adr-guard check-range --allow-status --allow-links` が、範囲のマージ以外の各コミットを親と比べ、`adr-guard links` が、リンクのない ADR の参照を調べ、`adr-guard check-status` が、すべての ADR のステータスの行がちょうど 1 つあることを調べる。範囲の中で一度でも ADR を書き換えたコミットがあると、後で戻しても失敗するので、マージする前にブランチの履歴を直す。

手元では、adr-guard を入れてから、クローンしたら一度だけ git のフックを有効にする。
adr-guard がないと、pre-commit フックがコマンドを見つけられず、コミットできない。

```sh
cargo install --git https://github.com/tett23/adr-guard --tag v0.8.3
git config core.hooksPath .githooks
```

リリースのバイナリから入れる方法は、adr-guard の README にある。
有効になっているかは `git config --get core.hooksPath` で確かめられる（`.githooks` と出る）。

## 依存の方針

依存は必要なものに絞る（[ADR 0013](docs/adr/0013-JS依存の更新と削減.md)、[0014](docs/adr/0014-Rust依存の更新と削減.md)）。
数十行で書けるものは自前で実装し、ライブラリを足さない。

- `web/`：ルーター（`web/src/router.tsx`）と SQLite の DB の読み手（`web/src/sqlite.ts`、[ADR 0047](docs/adr/0047-自前のSQLiteの読み手.md)）を自前にしている。SPA は sql.js を使わない。sql.js は、読み手が SQLite と同じ行を読むことを確かめるテストでだけ使う（開発用の依存）。その型（`web/src/types/sql.js.d.ts`）も自前で、テストで sql.js の API を新しく使うときは型の宣言も足す。
- `web/`：シンタックスハイライトは Shiki（[ADR 0017](docs/adr/0017-Shikiによるシンタックスハイライト.md)）。言語は `web/src/highlight.ts` の `LANGUAGES` に登録したものだけが色付きになる。言語を足すときは、`@shikijs/langs/<言語>` を登録し、`web/src/highlight.test.tsx` の `SAMPLES` に見本を足し（足さないとテストが落ちる）、`web/src/highlightLanguages.ts` の `HIGHLIGHT_LANGUAGES` に言語名と別名を足し（足さないとテストが落ちる。Shiki を読み込むかをこの一覧で決める、[ADR 0029](docs/adr/0029-必要になったときの読み込み.md)）、README の対応表を更新する。Shiki は、色を付ける言語のコードブロックが画面の近くに来たときに `web/src/highlightLoader.ts` が起動する Web Worker（`web/src/highlight.worker.ts`）の中で動き、正規表現は Oniguruma（wasm）で照合する（[ADR 0038](docs/adr/0038-表示の速さの改善.md)、[0046](docs/adr/0046-メインスレッドを空ける.md)）。`web/src/highlight.ts` と Shiki のモジュールを本体のコードから静的に import しない（すると本体に取り込まれる）。登録した言語は、Oniguruma のエンジンで色分けできることをテストで確かめている。1 言語で gzip 後 1〜16 KB 増えるので、`web/vite.config.ts` の `chunkSizeWarningLimit` も必要に応じて見直す。
- `web/`：Markdown の拡張の記法（[ADR 0025](docs/adr/0025-Markdownの拡張.md)）は、micromark の構文拡張と mdast の書き換えを `web/src/markdown/` に自前で書く。remark や rehype のプラグインのライブラリは足さず、再実装が現実的でない描画のライブラリ（KaTeX、mermaid）だけを使う。KaTeX と mermaid は `web/src/render/loaders.ts` 経由で非同期に読み込む別チャンクなので、`web/src/render/katex.ts` と `web/src/render/mermaid.ts` を本体のコードから静的に import しない。KaTeX のフォントは `web/vite.config.ts` のプラグインで woff2 だけを残す。
- `web/`：SPA は相対のパス（Vite の `base: "./"`）でビルドする。どのパスに置いても読めるよう、CLI が `index.html` のパスを `site.toml` の `base_path` に合わせて書き換え、SPA は `<meta name="sqlite-cms-base">` を読んでパスを組み立てる（[ADR 0030](docs/adr/0030-GitHubPagesとrsyncへの公開.md)）。SPA の中でサイト内のパスを URL にするときは `web/src/base.ts` の `withBasePath` を通す。
- `web/`：本文の HTML は自前の rehype のプラグイン（`web/src/markdown/rawHtml.ts`、[ADR 0046](docs/adr/0046-メインスレッドを空ける.md)）で取り込み、`rehype-sanitize` で `web/src/sanitize.ts` の許可一覧以外を取り除く（[ADR 0018](docs/adr/0018-本文で許可するHTML.md)）。許可するタグを足すときは `EXTRA_TAGS` に足し、`web/src/sanitize.test.tsx` にテストを足す。sanitize は Markdown が作る要素（脚注、タスクリスト、コードブロックの言語名）も検査するので、許可一覧から外すと Markdown の出力が壊れる。Shiki の色分けは sanitize の後に、Web Worker の中で行う（[ADR 0046](docs/adr/0046-メインスレッドを空ける.md)）。
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

`sqlite-cms serve` で `example/` を配信し、ヘッドレスの Chrome を Chrome DevTools Protocol で動かして、ブラウザでの動きを確かめる（`web/scripts/e2e.mjs`、[ADR 0044](docs/adr/0044-E2EとCIの最適化.md)）。
計測するすべてのページがエラー（コンソールのエラー、400 以上の応答）なく描かれ、レイアウトがずれないこと、色分け、数式、図、画面の中の移動、ヘッダの検索と候補、カスタムのヘッダと既定のヘッダを確かめる。
既定のヘッダは、`example/` から `content/header.md` を除いた写しを一時ディレクトリに作って確かめる（`.env` は写さない）。
Lighthouse と同じく、手元に Chrome が要り、`SQLITE_CMS_BIN` でビルド済みのバイナリを使える。
jsdom などの DOM の代わりになるライブラリは入れず、描画を伴う動きはこの E2E で確かめる。

場面ごとに新しいタブを開く。ページを開く（`goto`）ときは、読み込みの完了（`Page.loadEventFired`）を待ってから本文を確かめる。
条件を待つ間（`waitFor`）の一回ごとの評価は 5 秒で区切り、Chrome が応えなければやり直す。
Chrome が命令に 30 秒応えずに失敗した場面だけ、新しいタブで一度やり直し、通れば結果の一覧に「（Chrome が応えず、やり直して通った）」と一回目の失敗を書く。
条件が成り立たない、値が違うといった失敗はやり直さない（[ADR 0071](docs/adr/0071-e2e-chrome-unresponsive.md)）。
場面の動かし方は `web/scripts/scenarios.mjs`、CDP の補助は `web/scripts/cdp.mjs` にあり、どちらも単体テストがある（`npm --prefix web test`）。

E2E、Lighthouse、Core Web Vitals は、`sqlite-cms serve` を `SQLITE_CMS_OFFLINE=1` で起動し（`web/scripts/server.mjs`）、外部と通信しない（[ADR 0066](docs/adr/0066-offline-browser-checks-with-committed-cache.md)）。
見本のリンクカードの画像とコードの埋め込みは、コミットしたキャッシュ（`example/.sqlite-cms-cache/`）から使う。外部のサイトの回数の制限や障害で、検査が落ちないようにするためである。
見本に URL だけの行を足したり変えたりしたら、通信できる場所で見本を組み立てて取得し、増えたキャッシュをコミットする。

```sh
cargo run -- build example --out "$(mktemp -d)/out"
git add example/.sqlite-cms-cache
```

取り忘れると、CLI の単体テスト（`example_has_cached_results_for_every_url`）が、その URL を挙げて失敗する。
取り直すときは、`example/.sqlite-cms-cache/` を消してから組み立てる。
見本の Gist のコードの埋め込みは中身が変わりうるので、通信できる場所で見本を組み立てると取り直される（[ADR 0074](docs/adr/0074-refresh-mutable-code-embeds.md)）。Gist が更新されていればキャッシュが書き換わるので、その結果をコミットする。

## SPA の開発

```sh
npm --prefix web run dev
```

`sqlite-cms build --data-only` で DB と画像を `web/public` に書き出してから、Vite の開発サーバを起動する。
SPA は Preact で描くので（[ADR 0039](docs/adr/0039-Preactへの置き換え.md)）、部品を変えると状態を保った差し替え（Fast Refresh）ではなく、ページ全体を読み直す。
記事は既定で `example/` のものを使う。別の記事リポジトリで試すときは `SITE_DIR` で指定する。

```sh
SITE_DIR=../../blog npm --prefix web run dev   # web/ から見た相対パス
```

開発サーバーは、起動のときと `/db/` への要求のたびに DB を確かめ、DB が消えていたり、DB のマイグレーションが `migrations/` の最新より古かったりしたら、`npm run data` で書き出し直してから応える（`web/dev/devDatabase.ts`、[ADR 0062](docs/adr/0062-開発サーバーのDBの書き出し直し.md)）。マイグレーションを足したときに、開発サーバーを起動し直さなくてよい。

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
3. E2E（上記）
4. Lighthouse の計測（下記）。3 つの Chrome で並行して計測する。レポートは成功しても失敗しても `lighthouse-reports` という Artifact に保存し、スコアの表は実行結果の概要に出す
5. Core Web Vitals の計測（下記）。ページを二つに分けて計測し、結果は `web-vitals-reports-1` と `web-vitals-reports-2` という Artifact に保存し、表は実行結果の概要に出す

1 と 2 は一つのジョブ（`test`）で、速く終わる検査から順に行う（[ADR 0044](docs/adr/0044-E2EとCIの最適化.md)）。
3 から 5 は、`test` が作った `target/debug/sqlite-cms` を Artifact で受け取り、別々の機械（`browser` のジョブ）で並行して行う（[ADR 0052](docs/adr/0052-ブラウザの検査の並行化.md)）。どれかが失敗しても、ほかの検査は最後まで行う。
ADR の検査（`.github/workflows/adr.yml`）は別のワークフローで、すべての変更で動く（[ADR 0064](docs/adr/0064-align-adr-conventions-with-global-rules.md)）。

表示に関わらない変更では、ジョブを飛ばす（[ADR 0070](docs/adr/0070-ci-job-skips-and-required-checks.md)）。
最初の `changes` のジョブが、変わったファイル（PR では元との分かれ目から、`main` への push では push の前から）を `.github/scripts/ci-needs.sh` で調べる。

| 変わったファイル | `test` | `browser`（E2E、Lighthouse、Core Web Vitals） |
|---|---|---|
| 文書と開発の手順だけ（`docs/`、`README.md`、`DEVELOPMENT.md`、`.githooks/`、`adr.yml`、`release.yml`、`.github/scripts/`、`.claude/`、`.gitignore`、`LICENSE`） | 飛ばす | 飛ばす |
| テスト（`web/src/**/*.test.ts(x)`、`web/scripts/*.test.mjs`、`cli/tests/`）と上のものだけ | 動かす | 飛ばす |
| それ以外（`cli/src/` は、Rust の単体テストが本体と同じファイルにあるので、こちら） | 動かす | 動かす |

差分を取れないときは、どちらも動かす。判断の決まりを変えたら、`.github/scripts/test-ci-needs.sh` も直す（`changes` のジョブで動かす）。
リリースのワークフローは、今までどおり、タグのたびにすべての検査を行う。

最後の `ci-result` のジョブが、`test` と `browser` が成功か飛ばしたものなら成功する。
main のブランチは保護してあり、`ci-result` と `adr` の通過をマージの条件にしている。
ブランチは PR にして、検査が通ってから、次のようにマージする（no-ff のマージで、マージのコミットの書き方は今までどおり）。
検査が通っていなければ、`gh` が拒む。手元で `git merge` して main に push すると、管理者は保護を迂回できてしまうので、しない。

```sh
gh pr merge <番号> --merge --subject "Merge branch '<ブランチ>'"
```

版を上げるコミット（「版を X にする」）は、今までどおり main に直に push する。
同じ PR に続けて push したときは、古い実行を取り消す。`main` への push は取り消さない。

## Lighthouse

```sh
npm --prefix web run build   # SPA を最新にしてから
npm --prefix web run lighthouse
```

`sqlite-cms serve` で `example/` を配信し、代表的な 8 ページと、重いページと複雑なページの見本 6 ページ（`web/scripts/pages.mjs` の `PAGES`。E2E も開く）をヘッドレスの Chrome で計測する（[ADR 0019](docs/adr/0019-Lighthouseの計測とアクセシビリティ.md)）。
手元に Chrome が要る。
レポート（HTML と JSON）は `web/lighthouse-reports/` に日本語で出る。
配信には既定で `cargo run` を使う。ビルド済みのバイナリで計測するときは、`SQLITE_CMS_BIN` にそのパスを渡す。
`LIGHTHOUSE_CONCURRENCY` に 2 以上を渡すと、ページを分けて、その数の Chrome で並行して計測する（[ADR 0044](docs/adr/0044-E2EとCIの最適化.md)。CI は 3）。速くなるが、CPU を取り合うのでパフォーマンスの点数は揺れる。パフォーマンスを比べるときは、既定の 1 で計測する。

アクセシビリティとベストプラクティスは 100 点でなければ失敗し、足りない項目と要素を表示する（[ADR 0024](docs/adr/0024-ベストプラクティスの必須化.md)）。
ほかの項目（パフォーマンス、SEO）は計測して記録するだけで、基準はまだ決めていない。

`sqlite-cms serve` は、本番の Cloudflare と同じく応答を gzip で圧縮する（[ADR 0026](docs/adr/0026-serveのgzip圧縮.md)、[0038](docs/adr/0038-表示の速さの改善.md)）。圧縮は自前の実装で、大きさは zlib とほぼ同じになる。本番の Cloudflare は Brotli で配信するので、パフォーマンスの値は本番より少し悪く出る。
計測ごとに数点の揺れがある。パフォーマンスを改善したときは、改善の前後を何度か計測して比べる。
基準を足すときは `REQUIRED` に書く。

## Core Web Vitals

```sh
npm --prefix web run build   # SPA を最新にしてから
npm --prefix web run web-vitals
```

Lighthouse と同じページ（`PAGES`）を、操作まで含めて計測し、LCP、INP、CLS を出す（`web/scripts/web-vitals.mjs`、[ADR 0050](docs/adr/0050-CoreWebVitalsの計測.md)）。
Lighthouse の通常の計測はページを開くだけで INP を測れないので、Chrome DevTools Protocol で自前で計測する。

- 条件は Lighthouse のスマートフォンの計測と同じにする（412 × 823、CPU 4 倍の遅さ、遅い 4G、キャッシュなし）。
- ページを開いて 2 秒待ち、LCP を読む。続けて、ヘッダの検索ボックスを押して「sql」と打ち、Escape で閉じ、ヘッダの「一覧」を押す。入力は本物の入力（CDP の `Input`）で送る。
- 値はブラウザの `PerformanceObserver` で集め、`web-vitals` ライブラリと同じ数え方で計算する（`web/scripts/vitals.mjs`。vitest でテストする）。

どれかのページで「良好」の上限（LCP 2.5 秒、INP 200 ms、CLS 0.1）を超えると失敗し、超えた値を表示する。
結果は `web/web-vitals-reports/` に JSON と表（Markdown）で出る。
CPU を取り合うと INP が揺れるので、ページは一つずつ計測する（14 ページで 1 分半ほど）。
`WEB_VITALS_SHARD` に `番号/数`（例 `1/2`）を渡すと、ページを分けたうちの一つだけを計測する（CI は別々の機械で `1/2` と `2/2`、[ADR 0052](docs/adr/0052-ブラウザの検査の並行化.md)）。
手元の機械が遅くて LCP が揺れるときは、`WEB_VITALS_CPU_THROTTLE` で CPU を遅くする倍率（既定 4）を変えられる。CI では変えない。
E2E と共通のブラウザの操作は `web/scripts/cdp.mjs` にある。

## リリース

`v*` のタグを push すると、`.github/workflows/release.yml` が次を行う。

1. SPA をビルドし、テストを実行する。テストはリリースと同じ出力先（`--target`）で行い、続くビルドでそのまま使う（[ADR 0044](docs/adr/0044-E2EとCIの最適化.md)）
2. Linux x86_64 と macOS arm64 のバイナリをビルドする
3. Linux のビルドで作ったバイナリで、E2E、Lighthouse、Core Web Vitals を別々の機械で並行して行い（[ADR 0052](docs/adr/0052-ブラウザの検査の並行化.md)）、結果を `lighthouse-reports` と `web-vitals-reports-*` の Artifact に保存する。E2E が失敗するか、アクセシビリティかベストプラクティスが 100 点を割るか、Core Web Vitals が「良好」の上限を超えると、ここで失敗して Release は作られない
4. `sqlite-cms-<target>.tar.gz` を GitHub Release に添付する

タグを付けずに試すときは、`gh workflow run release.yml` で手動実行する。ビルドと梱包までを行い、Release は作らない（成果物は実行結果の Artifacts から取れる）。

記事リポジトリの公開ワークフロー（雛形は `example/.github/workflows/deploy.yml`）は、最新のリリースからバイナリを取得する。
