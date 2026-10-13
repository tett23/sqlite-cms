# ADR 0070: 表示に関わらない変更ではブラウザの検査を飛ばし、検査の通過をマージの条件にする

ステータス: 採択

## 文脈

CI（`.github/workflows/ci.yml`）は、`test` のジョブ（SPA のビルド、vitest、clippy、cargo test）の後に、`browser` のジョブで E2E、Lighthouse、Core Web Vitals（2 つ）を、別々の機械で並行して行う（[ADR 0044](0044-E2EとCIの最適化.md)、[0052](0052-ブラウザの検査の並行化.md)）。
ワークフローは、`docs/`、`README.md`、`DEVELOPMENT.md` だけを変えたときは動かない（ワークフローの `paths`、[ADR 0044](0044-E2EとCIの最適化.md)）。

ところが、次のような変更でも、すべての検査が動いている。

- 開発の手順だけの変更：`.githooks/`、`.github/workflows/adr.yml`、`.claude/`、`.gitignore`、`LICENSE`。ADR の決まりを adr-guard に合わせた PR（#9、#10）は、どれも文書と hook と `adr.yml` だけの変更だったが、ブラウザの検査まで動いた。
- テストだけの変更：リグレッションテストを足した PR（#8）は、`*.test.ts(x)` と Rust のテストだけの変更だったが、ブラウザの検査まで動いた。

ブラウザの検査は、一回の CI で機械 4 台を使い、`test` の後に数分かかる。表示（サイトの組み立てと SPA）に関わらない変更では、結果は変わらない。

また、main のブランチは保護しておらず、検査が落ちていてもマージできる。
検査の通過をマージの条件にするには、ブランチを保護して、必須の検査（required status checks）を決める。
このとき、ワークフローの `paths` で動かなかったワークフローの検査は、報告されないまま「待ち」になり、PR をマージできなくなる。ジョブの `if` で飛ばしたジョブは「skipped」と報告され、通過とみなされる。
`browser` のジョブは matrix なので、飛ばすと検査の名前（E2E など）が報告されない。

## 実装すること

### ワークフローは常に動かし、ジョブを飛ばす

- `ci.yml` から、ワークフローの `paths` をなくす。すべての PR と `main` への push で動かす。
- 最初に `changes` のジョブを置き、変わったファイルから、`test` と `browser` が要るかを出力（`test`、`browser`）にする。
- `test` のジョブは、`test` が `true` のときだけ動く。`browser` のジョブは、`browser` が `true` のときだけ動く（`if`）。
- 最後に `ci-result` のジョブを置く。`changes`、`test`、`browser` が終わった後に必ず動き（`if: always()`）、`changes` が成功し、`test` と `browser` が成功か skipped なら成功、そうでなければ失敗する。

### 要るかの決め方

- 変わったファイルは、PR では PR の元（base）と先（head）の分かれ目からの差分（`base...head`）、`main` への push では push の前（`github.event.before`）と後の差分で決める。差分を取れないとき（前のコミットがない、履歴にない）は、どちらも要るとみなす。
- 次の「動かさなくてよいもの」だけなら、`test` も `browser` も要らない。
  - `docs/**`、`README.md`、`DEVELOPMENT.md`
  - `.githooks/**`、`.github/workflows/adr.yml`、`.github/workflows/release.yml`、`.github/scripts/**`、`.claude/**`
  - `.gitignore`、`LICENSE`
- 次の「テスト」と、上の「動かさなくてよいもの」だけなら、`test` は要るが、`browser` は要らない。
  - `web/src/**/*.test.ts`、`web/src/**/*.test.tsx`、`web/scripts/*.test.mjs`、`cli/tests/**`
- それ以外のファイルが一つでもあれば、どちらも要る。Rust の単体テストは、本体と同じファイル（`#[cfg(test)]`）にあるので、`cli/src/` の変更は、テストだけでも `browser` が要るとみなす。
- 決め方は、外部の action を使わず、シェルのスクリプト（`.github/scripts/ci-needs.sh`）に書く。ファイルの一覧を標準入力で受け取り、引数（`test` か `browser`）が要るかを `true` か `false` で出す。`changes` のジョブで、このスクリプトのテスト（`.github/scripts/test-ci-needs.sh`）も動かす。

### 検査の通過をマージの条件にする

- main のブランチを保護し、必須の検査を `ci-result`（`ci` のワークフロー）と `adr`（`adr` のワークフロー）にする。
- 「マージする前にブランチを最新にする」（strict）は求めない。マージのたびに CI をやり直さないためである。
- 管理者にも保護を当てること（enforce admins）はしない。版を上げるコミット（「版を X にする」）は、今までどおり main に直に push する。
- ブランチは PR でマージし、`gh pr merge <番号> --merge --subject "Merge branch '<ブランチ>'"` で行う（no-ff のマージで、マージのコミットの書き方も今までどおり）。必須の検査が通っていなければ、`gh` が拒む。手元で `git merge` して main に push すると、管理者は保護を迂回できてしまうので、しない。

### 変えないもの

- リリースのワークフロー（`release.yml`）は、今までどおり、タグのたびにブラウザの検査を行う。配るバイナリで確かめるためである。
- `test` のジョブの中身と、ブラウザの検査の分け方。

### 文書

- DEVELOPMENT.md の「CI」に、ジョブを飛ばす条件、`ci-result`、必須の検査と、PR のマージの仕方を書く。

## 実装しないこと

- `test` のジョブを、変わったファイルで分けること（Rust だけ、SPA だけ）。Rust のテストは埋め込んだ SPA を使うので、分けても SPA のビルドを待つ（[ADR 0044](0044-E2EとCIの最適化.md)）。
- 変わったファイルの種類ごとに、ブラウザの検査の一部（E2E だけ、など）を選ぶこと。
- 外部の action（`dorny/paths-filter` など）を使うこと。
- レビューの承認をマージの条件にすること。
- 版を上げるコミットも PR を通すこと。

## テスト設計

- `.github/scripts/test-ci-needs.sh` で、次を確かめる。
  - どちらも要らない：文書だけ（`docs/adr/0001-a.md`、`README.md`）、hook と `adr.yml` だけ、`.github/scripts/ci-needs.sh` だけ。
  - `test` だけ要る：テストだけ（`web/src/a.test.ts`、`web/src/render/A.test.tsx`、`cli/tests/cli.rs`、`web/scripts/cdp.test.mjs`）、テストと文書の組み合わせ。
  - どちらも要る：`web/src/a.ts`、`cli/src/codeembed.rs`、`example/content/index.md`、`migrations/0012_code_embeds.sql`、`web/package.json`、`.github/workflows/ci.yml`、テストと本体の組み合わせ、空の一覧（差分を取れないとき）。
- actionlint がワークフローにエラーを出さないこと。
- この ADR の PR の CI で、`ci.yml` を変えるので、すべてのジョブが動き、`ci-result` が成功すること。
- 保護を付けた後、`gh api` でブランチの保護を読み、必須の検査が `ci-result` と `adr` であること。
- 後で、文書だけの PR で `test` と `browser` が skipped になり、`ci-result` が成功してマージできることを、CI の画面で確かめる。

## トレードオフ

- 文書だけの変更でも、`changes` と `ci-result` のジョブの分だけ機械を使う（数十秒）。必須の検査を報告するためである。
- 表示に関わらないと決めた変更が、実は表示に関わっていたとき、ブラウザの検査を通らずにマージされうる。リリースのワークフローで、タグのたびに確かめる。
- 飛ばす条件を、ファイルの名前の決まりに頼る。テストのファイルの名前の決まりを変えたら、スクリプトも直す必要がある。
- 管理者は保護を迂回して main に push できる。マージは `gh pr merge` で行うという手順で守る。
- ブラウザの検査が一時的に落ちた（Chrome が応えない、など）ときも、流し直して通るまでマージできない。
