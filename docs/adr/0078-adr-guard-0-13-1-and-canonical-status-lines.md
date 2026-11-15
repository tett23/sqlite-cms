# ADR 0078: adr-guard を 0.13.1 に上げ、ADR のステータスの行を新しい形に移す

Status: accepted-pending

## 文脈

[ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md) で、adr-guard を 0.10.0 に上げた。
CI（`.github/workflows/adr.yml`）の action は `tett23/adr-guard@v0.10.0` で、git の pre-commit フックは `adr-guard pre-commit --allow-status --allow-links` を動かす。
ADR のステータスの行は、すべて `ステータス: 採択` のような日本語の形（以下、旧形式）で書いてある。

adr-guard は 0.13.1 になった（[リリース v0.11.0](https://github.com/tett23/adr-guard/releases/tag/v0.11.0)、[v0.12.0](https://github.com/tett23/adr-guard/releases/tag/v0.12.0)、[v0.13.0](https://github.com/tett23/adr-guard/releases/tag/v0.13.0)、[v0.13.1](https://github.com/tett23/adr-guard/releases/tag/v0.13.1)）。

- 0.11.0 で、Codex の PreToolUse フックとして `adr-guard codex-hook` が加わった（tett23/adr-guard の [ADR 0021: Codex の PreToolUse hook で、コミット済みのADRと記録の編集を拒否する](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0021-codex-hook.md)）。
- 0.12.0 で、ステータスの行の新しい形（以下、新形式）と、状態遷移の検査が加わった（tett23/adr-guard の [ADR 0022: 状態と遷移を別々の代数的データ型で表す](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0022-status-transitions.md)、[ADR 0023: trace で元の決定から修正ADRをたどる](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0023-trace-amendments.md)）。
  - 新形式は `Status: <トークン>` で、トークンは `proposed`、`accepted`、`accepted-pending`、`rejected`、`discarded`、`deprecated`、または `superseded [<変更の JSON 配列>]` である。変更は `{"part":<改めた部分の名前か null>,"by":[<改めた ADR のリポジトリのルートからのパス>]}` の形で書く。
  - 新しい ADR は、最初に `Status: proposed` でコミットしなければならない。足した ADR と、ステータスの行を変えた ADR には、新形式が要る。旧形式の行は、変えなければそのまま読める。
  - pre-commit と `check-range` は、ステータスの行の状態遷移（`proposed` から `accepted` へ、`accepted` から `superseded` へ、など）を調べる。この検査は、`--allow-status`、環境変数 `ADR_GUARD_ALLOW`、コミットのトレーラーでは免除されない。
  - 旧形式を新形式に書き換える移行スクリプト `scripts/migrate-adr-status` が、adr-guard のソースに加わった。リリースのアーカイブには入っていない。
  - 元の判断から、それを改めた ADR をたどる `adr-guard trace` が加わった。
- 0.13.0 で、移行スクリプトに、書き換えずに差分だけを示す `--dry-run` が加わった。移行できない ADR を最後まで数え、`移行できる: N件、移行済み: K件、移行できない: M件` を標準エラーに出す。新形式に `superseded-pending`、変更の `kind` と `note`、`https://` で始まる修正先、`trace --list` の 6 列が加わり、旧形式の修正先の読み方が広がった（tett23/adr-guard の [ADR 0024: 移行スクリプトに --dry-run を設け、失敗を最後まで数える](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0024-migration-dry-run-and-error-count.md)、[ADR 0025: 新形式のステータス行に、実装待ちの修正、補足、注記、別のリポジトリの修正先を加える](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0025-extend-canonical-status-amendments.md)、[ADR 0026: 旧形式のステータス行の修正先を、リンク先と裸の番号からも読む](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0026-read-more-legacy-references.md)、[ADR 0027: 旧形式の `置換（…）` を、修正を受けた状態として読む](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0027-read-legacy-replaced-status.md)）。
- 0.13.1 で、移行スクリプトが作る部分名の不具合が直った。0.13.0 の移行スクリプトは、部分名に修飾子（`ADR NNNN` の直前の語）、先頭の「。」、末尾の「→」を残すことがあった。0.13.1 はこれらを除き、部分名を決められない行は「部分名を決められません」と報告して書かない（tett23/adr-guard の [ADR 0028: 旧形式の部分名から修飾子と区切りの記号を除き、決められない部分名を移行しない](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0028-fix-legacy-part-names.md)）。

手元の adr-guard はすでに 0.13.1 になっている。
手元の pre-commit フックは、新しい ADR を旧形式で書くと拒み、コミットした ADR のステータスの行を旧形式のまま書き換えても拒む（「調査した事項」）。
そのため、ADR を足すたびに、またステータスの行を変えるたびに、新形式と旧形式が混ざっていく。
DEVELOPMENT.md と README.md は、レビューを受けて承認された ADR を、ステータスを「採択」にしてコミットする手順を書いている。
新しい ADR は `proposed` 以外でコミットできないので、この手順はそのままでは通らない。
CI と手元の版をそろえ、すべての ADR のステータスの行を一度に新形式に移す。

## 調査した事項

次のことを、手元で実行して確かめた。

- `adr-guard -V` は `adr-guard 0.13.1` を出した。
- 移行スクリプトは、adr-guard の v0.13.1 のソースの `scripts/migrate-adr-status` を使った。このスクリプトは、`cargo run --example migrate_adr_status` で example の `migrate_adr_status` をビルドして実行する。以下では、ビルドした `migrate_adr_status` を直接実行した。
- このリポジトリのルート（origin/main の c75be55）で `migrate_adr_status --dry-run` を動かすと、終了コード 0 で終わり、標準エラーの最後の行は `移行できる: 75件、移行済み: 0件、移行できない: 0件` だった。作業ツリーは書き換わらなかった。
  - この ADR の草稿を置いた作業ツリーでは、最後の行は `移行できる: 75件、移行済み: 1件、移行できない: 0件` だった。この ADR は新形式なので、移行済みに数えられる。
  - 差分の行は、0.13.0（`0096636`）の移行スクリプトを同じコミットで動かしたときと同じだった。このリポジトリの部分名は、tett23/adr-guard の [ADR 0028](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0028-fix-legacy-part-names.md) の修正の影響を受けない。
  - 75 件のうち、`accepted` が 68 件、`discarded` が 1 件（[0061](0061-ページごとのOGP.md)）、`superseded` が 6 件に移る。
  - [ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md) の行は `Status: accepted` に、[ADR 0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md) の `ステータス: 採択→版は[ADR 0077](...)` は `Status: superseded [{"by":["docs/adr/0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md"],"part":"版"}]` に移る。
- 同じコミットの複製で `migrate_adr_status --write` を動かすと、75 件の ADR のステータスの行だけが書き換わった（`75 files changed, 75 insertions(+), 75 deletions(-)`）。その後の `--dry-run` は `移行できる: 0件、移行済み: 75件、移行できない: 0件` を出した。
  - 書き換えた後の複製で、`adr-guard links`、`adr-guard check-status`、`adr-guard check-numbers` は終了コード 0 で終わった。`adr-guard unimplemented` は何も出さなかった。
  - 書き換えをステージして `adr-guard pre-commit --allow-status --allow-links` は終了コード 0 で終わった。コミットした後、`adr-guard check-range HEAD~1 HEAD --allow-status --allow-links` も終了コード 0 で終わった。
- 移行していない複製（別のリポジトリ shorten-path で試した）で、手元の adr-guard（0.13.1）の pre-commit は次を拒んだ。
  - `ステータス: 提案` の新しい ADR。診断は `` 新形式のステータス行は `Status: ` で始めてください `` だった。
  - `Status: accepted` の新しい ADR。診断は `許可されていない状態遷移: Absent -> accepted` だった。
  - コミットした ADR の `ステータス: 採択` を、旧形式のまま `ステータス: 採択→…` に書き換えた変更。診断は `` 新形式のステータス行は `Status: ` で始めてください `` で、終了コードは 1 だった。
  - `Status: proposed` の新しい ADR は許した。
- 実装の手順を、origin/main の複製で次のとおりに試した。どの段階でも pre-commit（`--allow-status --allow-links`）は終了コード 0 で終わった。
  1. この ADR を `Status: proposed` のまま足してコミットした。
  2. 移行スクリプトの `--write` を動かした。最後の行は `移行できる: 75件、移行済み: 1件、移行できない: 0件` で、この ADR は移行済みに数えられた。
  3. この ADR を `Status: accepted` に、[ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md) を「実装すること」に書いた `superseded` の行にして、移行とあわせて 1 つのコミットにした。
  - その後、`adr-guard check-range <origin/main> HEAD --allow-status --allow-links`、`adr-guard links`、`adr-guard check-status`、`adr-guard check-numbers` は終了コード 0 で終わった。
  - 移行を 1 つのコミットにし、この ADR の採択と [ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md) の変更を次のコミットに分けたときも、同じ結果だった。
- v0.13.1 の `action.yml` の入力の名前は、v0.10.0 と同じ（`base`、`head`、`version`、`allow-status`、`allow-links`、`links`、`check-status`、`check-numbers`、`allow-renumber`）である。

## 実装すること

- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.13.1` にする。`allow-status: true` と `allow-links: true` と `base`、`head` はそのままにし、ほかの入力は指定しない。CI の YAML のコメントの ADR の番号にこの ADR を足す。
- git の pre-commit フック（`.githooks/pre-commit`）は `exec adr-guard pre-commit --allow-status --allow-links` のままにし、コメントの ADR の番号にこの ADR を足す。
- このリポジトリのルートで、adr-guard の v0.13.1 の移行スクリプトを `--dry-run` で動かし、`移行できない: 0件` で終わることを確かめてから、`--write` で全 ADR のステータスの行を新形式に書き換える。adr-guard のソースから動かすときは `scripts/migrate-adr-status --dry-run` と `scripts/migrate-adr-status --write` である。書き換えの結果は手で直さない。
- [ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md) のステータスの行を `Status: superseded [{"by":["docs/adr/0078-adr-guard-0-13-1-and-canonical-status-lines.md"],"part":"版"}]` にする。部分の名前は、移行した後の [ADR 0069](0069-adr-guard-0-5-and-allow-status.md)、[0072](0072-adr-guard-0-8-and-status-lines.md)、[0073](0073-adr-guard-0-8-1-and-directory-name-variants.md)、[0075](0075-adr-guard-0-8-2-and-link-only-changes.md)、[0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md) のステータスの行と同じ「版」にする。
- README.md と DEVELOPMENT.md の adr-guard の版と入れるときのタグを v0.13.1 にし、DEVELOPMENT.md の adr-guard の ADR の一覧にこの ADR を足す。
- README.md と DEVELOPMENT.md の ADR の手順とステータスの書き方を、新形式に合わせる。
  - 書いた ADR は、レビューを受けてから `Status: proposed` でコミットし、承認されたら次のコミットで `Status: accepted`（まだ実装していなければ `Status: accepted-pending`）にする。
  - DEVELOPMENT.md の雛形のステータスの行を `Status: proposed` にし、ステータスのトークン（`proposed`、`accepted`、`accepted-pending`、`superseded [...]`、`rejected`、`discarded`）を書く。判断を改めたときは、改めた側の番号を元の ADR のステータスの行に `superseded` で書く。
  - ステータスの行を、行頭が `Status:` の行と書く。コミットした ADR で許す変更は、`Status:` で始まる行の、許される状態遷移の変更と旧形式から新形式への書き換えであることと、状態遷移の検査が `ADR_GUARD_ALLOW` とトレーラーでは免除されないことを書く。
- `docs/specifications.md` の「古い形の ADR」の、ステータスの行の記述を新形式に合わせる（`ステータス: 採択` を `Status: accepted` に、`ステータス: 破棄` を `Status: discarded` に、`採択→…` を `superseded` にする）。

## 実装しないこと

- Codex のフック（`adr-guard codex-hook`）を使うこと。Claude Code のフックと同じく、このリポジトリでは管理しない。
- Claude Code のフックの変更（利用者の全体の設定にあり、このリポジトリでは管理しない。[ADR 0068](0068-adr-guard-0-4-and-adr-links.md)）。
- 移行スクリプトの結果を超えた、ステータスの行の書き直し（`kind` や `note` を足すこと、古い形の ADR の「状態」の節にある改めた記録を `superseded` に移すことなど）。
- ADR の本文と、ステータスの行のほかの行の変更。古い形の ADR の「状態」の節も、書いたときのまま残す。
- [ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md) より前の adr-guard の更新の ADR のステータスの行を、移行のほかで変えること（直前の adr-guard の更新の ADR だけを改める）。
- pre-commit フックと CI で `--allow-renumber` を許すこと。

## 不変条件

なし。設定の値と移行の手順を決める ADR で、入力によって変わる振る舞いを足さない。

## テスト設計

このリポジトリには ADR の書式を調べるテストがないので、自動テストは足さない。次のように確かめる。

- 書き換える前に、移行スクリプトの `--dry-run` が終了コード 0 で終わり、`移行できない: 0件` を出すこと。この ADR は新形式なので「移行済み」に数えられる。書き換えた後の `--dry-run` が `移行できる: 0件` を出すこと。
- `adr-guard links --fix` の後、`adr-guard links`、`adr-guard check-status`、`adr-guard check-numbers` が、終了コード 0 で終わること。
- `adr-guard unimplemented` の出力が、書き換える前と同じであること（今は空）。
- `adr-guard trace docs/adr/0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md` が、この ADR への部分「版」の辺を出すこと。
- 変更をステージして、手元の adr-guard（0.13.1）の `.githooks/pre-commit` が通ること。実装のコミットには、全 ADR の移行と、[ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md) の `superseded` への変更と、この ADR の `proposed` から `accepted` への変更が含まれる。
- コミットの後、`adr-guard check-range origin/main HEAD --allow-status --allow-links` が終了コード 0 で終わること。
- `.github/workflows/adr.yml` が YAML として読めること。
- この PR の CI の `adr` のワークフローが、v0.13.1 の action で通ること。

## トレードオフ

- ADR のステータスの行が英語のトークンと JSON になり、日本語の形より読みにくい。代わりに、状態遷移の検査と `adr-guard trace` が使え、旧形式と新形式が混ざらない。
- 一度に 75 件の ADR のステータスの行を書き換えるので、差分が大きい。書き換えは移行スクリプトの結果だけで、手で直さないので、`--dry-run` の差分と比べて確かめられる。
- 古い形の ADR では、ステータスの行が英語の新形式に、「状態」の節が日本語のままになり、2 つの書き方が 1 つの ADR に並ぶ。読み方は `docs/specifications.md` の「古い形の ADR」に書く。
- 状態遷移の検査は `ADR_GUARD_ALLOW` とトレーラーで免除されないので、誤ったステータスをすぐに戻す手段が減る。
- 手元の adr-guard が 0.12.0 より古いと、新形式の行と状態遷移が手元で調べられず、CI と手元の結果が違いうる。README.md と DEVELOPMENT.md の入れ方のタグを上げて補う。

## 参考文献

- tett23/adr-guard, [ADR 0021: Codex の PreToolUse hook で、コミット済みのADRと記録の編集を拒否する](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0021-codex-hook.md)
- tett23/adr-guard, [ADR 0022: 状態と遷移を別々の代数的データ型で表す](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0022-status-transitions.md)
- tett23/adr-guard, [ADR 0023: trace で元の決定から修正ADRをたどる](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0023-trace-amendments.md)
- tett23/adr-guard, [ADR 0024: 移行スクリプトに --dry-run を設け、失敗を最後まで数える](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0024-migration-dry-run-and-error-count.md)
- tett23/adr-guard, [ADR 0025: 新形式のステータス行に、実装待ちの修正、補足、注記、別のリポジトリの修正先を加える](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0025-extend-canonical-status-amendments.md)
- tett23/adr-guard, [ADR 0026: 旧形式のステータス行の修正先を、リンク先と裸の番号からも読む](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0026-read-more-legacy-references.md)
- tett23/adr-guard, [ADR 0027: 旧形式の `置換（…）` を、修正を受けた状態として読む](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0027-read-legacy-replaced-status.md)
- tett23/adr-guard, [ADR 0028: 旧形式の部分名から修飾子と区切りの記号を除き、決められない部分名を移行しない](https://github.com/tett23/adr-guard/blob/v0.13.1/docs/adr/0028-fix-legacy-part-names.md)
- tett23/adr-guard, [Release v0.11.0](https://github.com/tett23/adr-guard/releases/tag/v0.11.0)
- tett23/adr-guard, [Release v0.12.0](https://github.com/tett23/adr-guard/releases/tag/v0.12.0)
- tett23/adr-guard, [Release v0.13.0](https://github.com/tett23/adr-guard/releases/tag/v0.13.0)
- tett23/adr-guard, [Release v0.13.1](https://github.com/tett23/adr-guard/releases/tag/v0.13.1)
