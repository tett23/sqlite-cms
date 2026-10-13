# ADR 0072: adr-guard を 0.8.0 に上げ、古い形の ADR にステータスの行を足す

ステータス: 採択→版は[ADR 0073](0073-adr-guard-0-8-1-and-directory-name-variants.md)

## 文脈

[ADR 0069](0069-adr-guard-0-5-and-allow-status.md) で、adr-guard を 0.5.0 に上げ、pre-commit フックと CI の action に `--allow-status`（`allow-status: true`）を付けた。

adr-guard は 0.8.0 になり、次の規則が足された。

- 0.6.0：どの深さの `docs/adr/`、`docs/<モジュール>/adr/` も守る（tett23/adr-guard の [ADR 0011](https://github.com/tett23/adr-guard/blob/main/docs/adr/0011-nested-adr-directories.md)）。
- 0.7.0：ADR は、行頭が `ステータス:` か `Status:` のステータスの行を、ちょうど 1 つ持たなければならない。無いものと 2 つ以上あるものは、pre-commit フックと新しいサブコマンド `adr-guard check-status` で違反になる。action は既定で `check-status` も動かす（入力 `check-status: false` で止められる）。`--allow-status` は、ステータスの行が無かった ADR に 1 行を足すことも許す（tett23/adr-guard の [ADR 0012](https://github.com/tett23/adr-guard/blob/main/docs/adr/0012-status-line-and-status-adt.md)）。
- 0.8.0：どの深さの `docs/` の `docs/audit/` と `docs/benchmarks/` も、コミットしたファイルの変更、削除、名前の変更を違反とする。`--allow-status` と `--allow-links` は、これらの記録には効かない（tett23/adr-guard の [ADR 0013](https://github.com/tett23/adr-guard/blob/main/docs/adr/0013-protect-audit-and-benchmark-records.md)）。

手元の adr-guard はすでに 0.8.0 になっている。
`adr-guard check-status` を動かすと、古い形の ADR（[0001](0001-記事フォーマットとSQLite生成.md) から [0062](0062-開発サーバーのDBの書き出し直し.md) の 60 件。0032 と 0035 はファイルがない）が、ステータスの行がないので違反になる。
古い形の ADR は `## 状態` の節で状態を書いていて、今のステータスは `docs/specifications.md` の「古い形の ADR」に書いた（[ADR 0064](0064-align-adr-conventions-with-global-rules.md)）。
0.7.0 からは、ステータスの行を足すことが `--allow-status` で許されるので、[ADR 0069](0069-adr-guard-0-5-and-allow-status.md) で実装しなかった「古い形の ADR のステータスを、`ステータス:` の行で書く」ことができる。

このリポジトリには `docs/audit/` と `docs/benchmarks/` はない。

## 実装すること

- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.8.0` にする。`allow-status: true` と `allow-links: true` はそのままにし、`check-status: false` は付けない（下で違反をなくすので、既定の `check-status` を動かす）。
- git の pre-commit フック（`.githooks/pre-commit`）は `exec adr-guard pre-commit --allow-status --allow-links` のままにし、コメントの ADR の番号にこの ADR を足し、ステータスの行がちょうど 1 つ要ることを書く。
- 古い形の ADR の 60 件に、題名の行の次に `ステータス: <値>` の行を 1 行だけ足す（tett23/adr-guard の README の「ステータスの行が無いADRがある場合」の手順）。空行などは足さない。`## 状態` の節はそのまま残す。値は `docs/specifications.md` の「古い形の ADR」に従う。
  - [0001](0001-記事フォーマットとSQLite生成.md) から [0060](0060-記事のカテゴリ.md) と [0062](0062-開発サーバーのDBの書き出し直し.md)：`ステータス: 採択`
  - [0061](0061-ページごとのOGP.md)：`ステータス: 破棄`
- `docs/specifications.md` の「古い形の ADR」を、ステータスの行を足した今の状態に合わせる（今のステータスはステータスの行にあり、`## 状態` の節は書いたときのまま残っていること）。
- README.md と DEVELOPMENT.md の adr-guard の説明を改める（入れるときのタグを v0.8.0 にする、ADR はステータスの行をちょうど 1 つ持つこと、CI が `check-status` を動かすこと、`docs/audit/` と `docs/benchmarks/` を置けば守られること、古い形の ADR にもステータスの行があること）。
- [ADR 0069](0069-adr-guard-0-5-and-allow-status.md) のステータスを `採択→版は[ADR 0072](0072-adr-guard-0-8-and-status-lines.md)` にする。

## 実装しないこと

- 古い形の ADR のステータスを、後の ADR で一部を改めたことに合わせて `採択→…` にすること。古い形の ADR の多くは、後の ADR で一部を改めていて、その記録は改めた側（とときに改められた側）の `## 状態` の節にある。すべてを洗い出して `採択→…` の形に書き直すのは別の作業とし、ここでは「古い形の ADR」に書いた今のステータスを、そのまま行にする。
- 古い形の ADR のファイル名や、`## 状態` の節の書き換え（ステータスの行のほかの変更なので、adr-guard が拒む）。
- [ADR 0064](0064-align-adr-conventions-with-global-rules.md)、[0068](0068-adr-guard-0-4-and-adr-links.md) のステータスの行の変更（[ADR 0069](0069-adr-guard-0-5-and-allow-status.md) と同じく、直前の adr-guard の更新の ADR だけを改める）。
- Claude Code のフックの変更（利用者の全体の設定にあり、このリポジトリでは管理しない。[ADR 0068](0068-adr-guard-0-4-and-adr-links.md)）。
- `docs/audit/` と `docs/benchmarks/` を作ること。

## テスト設計

- `adr-guard check-status` が、違反なし（終了コード 0）で終わること。
- `adr-guard links` が、リンクのない参照なし（終了コード 0）で終わること。
- 変更をステージして、手元の adr-guard（0.8.0）の `.githooks/pre-commit` が通ること。この変更には、ステータスの行がなかった 60 件への行の追加と、[ADR 0069](0069-adr-guard-0-5-and-allow-status.md) のステータスの行の変更が含まれるので、`--allow-status` が効いていることも確かめられる。
- コミットした ADR の本文を一時的に変えてステージすると、`.githooks/pre-commit` が拒むこと（確かめた後に戻す）。
- `.github/workflows/adr.yml` が YAML として読めること。
- この PR の CI の `adr` のワークフローが、v0.8.0 の action で通ること。ジョブのログで、`check-range --allow-status --allow-links`、`links`、`check-status` が動いていることを確かめる。

## トレードオフ

- 手元の adr-guard が 0.7.0 より古いと、ステータスの行がない ADR の検査が手元で働かず、CI で初めて気づく。README.md と DEVELOPMENT.md の入れ方のタグを上げて補う。
- 古い形の ADR には、題名の行の次の `ステータス:` の行と、`## 状態` の節の二か所に状態が書かれる。食い違うとき（「提案。」のままの 0038 から 0062）は、ステータスの行を正とする。
- 一部を後の ADR で改めた古い形の ADR も `ステータス: 採択` にするので、ステータスの行だけからは改められたことが分からない。`## 状態` の節と、改めた側の ADR を読む必要がある。
