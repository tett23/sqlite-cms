# ADR 0073: adr-guard を 0.8.1 に上げ、ADR と記録のディレクトリの別名も守る

ステータス: 採択→版は[ADR 0075](0075-adr-guard-0-8-2-and-link-only-changes.md)

## 文脈

[ADR 0072](0072-adr-guard-0-8-and-status-lines.md) で、adr-guard を 0.8.0 に上げ、古い形の ADR にステータスの行を足した。

adr-guard は 0.8.1 になり、守るディレクトリの名前が増えた（tett23/adr-guard の [ADR 0014](https://github.com/tett23/adr-guard/blob/main/docs/adr/0014-directory-name-variants.md)、[リリース](https://github.com/tett23/adr-guard/releases/tag/v0.8.1)）。

- ADR のディレクトリとして、`adr` のほかに `adrs`、`decisions`、`decision` も守る。ADR として扱うので、ステータスの行の検査とリンクの検査もかかる。
- 記録のディレクトリとして、`docs/audit/` と `docs/benchmarks/` のほかに `docs/audits/` と `docs/benchmark/` も守る。

手元の adr-guard はすでに 0.8.1 になっている。
このリポジトリには、新しく守られる名前のディレクトリ（`adrs`、`decisions`、`decision`、`docs/audits/`、`docs/benchmark/`）はない（追跡しているファイルにも、追跡していないディレクトリにもない）。
そのため、上げても今の ADR と文書の扱いは変わらない。

## 実装すること

- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.8.1` にする。`allow-status: true` と `allow-links: true` はそのままにする。
- git の pre-commit フック（`.githooks/pre-commit`）は `exec adr-guard pre-commit --allow-status --allow-links` のままにし、コメントの ADR の番号にこの ADR を足す。
- README.md と DEVELOPMENT.md の adr-guard の説明を改める（版と入れるときのタグを v0.8.1 にする、守る記録のディレクトリに `docs/audits/` と `docs/benchmark/` を足す）。
- [ADR 0072](0072-adr-guard-0-8-and-status-lines.md) のステータスを `採択→版は[ADR 0073](0073-adr-guard-0-8-1-and-directory-name-variants.md)` にする。

## 実装しないこと

- ADR のディレクトリの別名（`adrs`、`decisions`、`decision`）を README.md と DEVELOPMENT.md に書くこと。このリポジトリの ADR は `docs/adr/` だけに置き、別名のディレクトリは使わない。
- `docs/audits/`、`docs/benchmark/` などのディレクトリを作ること。
- `docs/specifications.md` の変更（adr-guard の版と記録のディレクトリに触れていない）。
- [ADR 0064](0064-align-adr-conventions-with-global-rules.md)、[0068](0068-adr-guard-0-4-and-adr-links.md)、[0069](0069-adr-guard-0-5-and-allow-status.md) のステータスの行の変更（直前の adr-guard の更新の ADR だけを改める）。
- Claude Code のフックの変更（利用者の全体の設定にあり、このリポジトリでは管理しない。[ADR 0068](0068-adr-guard-0-4-and-adr-links.md)）。

## テスト設計

- `adr-guard check-status` と `adr-guard links` が、終了コード 0 で終わること。
- 変更をステージして、手元の adr-guard（0.8.1）の `.githooks/pre-commit` が通ること。この変更には [ADR 0072](0072-adr-guard-0-8-and-status-lines.md) のステータスの行の変更が含まれるので、`--allow-status` が効いていることも確かめられる。
- `.github/workflows/adr.yml` が YAML として読めること。
- この PR の CI の `adr` のワークフローが、v0.8.1 の action で通ること。

## トレードオフ

- 後で `adrs` などの名前のディレクトリを ADR や記録のつもりでなく作ると、adr-guard がそれを守ってしまう。そのときは名前を変える。
- 手元の adr-guard が 0.8.1 より古いと、`docs/audits/` と `docs/benchmark/` の記録の書き換えが手元で止まらず、CI で初めて気づく。README.md と DEVELOPMENT.md の入れ方のタグを上げて補う。
