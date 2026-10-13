# ADR 0069: adr-guard を 0.5.0 に上げ、ステータスの行の変更を `--allow-status` で許す

ステータス: 採択→版は[ADR 0072](0072-adr-guard-0-8-and-status-lines.md)

## 文脈

[ADR 0064](0064-align-adr-conventions-with-global-rules.md) で、コミットした ADR の書き換えを adr-guard で止めることにし、[ADR 0068](0068-adr-guard-0-4-and-adr-links.md) で、adr-guard を 0.4.0 に上げ、pre-commit フックと CI の action に `--allow-links`（`allow-links: true`）を付けた。

adr-guard は 0.5.0 になり、コミット済みの ADR のステータスの行（`ステータス:` で始まる行）の変更も、`--allow-status` を付けたときだけ許すようになった（tett23/adr-guard の [ADR 0010](https://github.com/tett23/adr-guard/blob/main/docs/adr/0010-allow-status-option.md)）。
付けないと、新しい ADR を採択したときに、古い ADR のステータスを `採択→…` に変えることもできない。

手元の adr-guard はすでに 0.5.0 になっている。
このリポジトリの pre-commit フックは `--allow-links` だけを付けているので、いまはステータスの行を変えるコミットが pre-commit フックで止まる。

文書のリンクのない ADR の参照は、`adr-guard links` で調べると、すでに 0 件である。

## 実装すること

- git の pre-commit フック（`.githooks/pre-commit`）を `exec adr-guard pre-commit --allow-status --allow-links` にし、コメントを改める。
  - この ADR の実装のコミットでは、[ADR 0068](0068-adr-guard-0-4-and-adr-links.md) のステータスの行を変えるので、フックの変更を同じコミットか、それより前のコミットに含める。
- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.5.0` にし、入力 `allow-status: true` を足す（`allow-links: true` はそのまま）。checkout は `actions/checkout@v7` のまま、`fetch-depth: 0` と `persist-credentials: false` も変えない。
- DEVELOPMENT.md と README.md の adr-guard の説明を改める（入れるときのタグを v0.5.0 にする、ステータスの行の変更は `--allow-status` で許すこと、pre-commit フックと CI のオプション）。
- `docs/specifications.md` の「古い形の ADR」の、adr-guard がステータスの行のほかの書き換えを拒むという記述に、ステータスの行の変更は `--allow-status` で許すことを足す。
- [ADR 0068](0068-adr-guard-0-4-and-adr-links.md) のステータスを `採択→版とステータスの行の許可は[ADR 0069](0069-adr-guard-0-5-and-allow-status.md)` にする。

## 実装しないこと

- 古い形の ADR のステータス（`## 状態` の節）を、新しい形の `ステータス:` の行に改めること。`--allow-status` が許すのは `ステータス:` で始まる行だけなので、今も adr-guard が拒む。
- [ADR 0064](0064-align-adr-conventions-with-global-rules.md) のステータスの行を変えること。adr-guard の版は [ADR 0068](0068-adr-guard-0-4-and-adr-links.md) ですでに改めており、0068 のときも 0064 のステータスは変えていない。
- Claude Code のフックの変更。フックは利用者の全体の設定（`~/.claude/settings.json`）にあり、このリポジトリでは管理しない（[ADR 0068](0068-adr-guard-0-4-and-adr-links.md)）。全体の設定を `adr-guard claude-hook --allow-status` にするのは各自の環境で行う。
- action の指定の仕方（版のタグ）の変更。

## テスト設計

- `adr-guard links` が、リンクのない参照なし（終了コード 0）で終わること。
- 変更をステージして、手元の adr-guard（0.5.0）の `adr-guard pre-commit --allow-status --allow-links` が通ること。この変更には [ADR 0068](0068-adr-guard-0-4-and-adr-links.md) のステータスの行の変更が含まれるので、`--allow-status` が効いていることも確かめられる。
- `.github/workflows/adr.yml` が YAML として読めること。
- この PR の CI の `adr` のワークフローが、v0.5.0 の action で通ること。ジョブのログで、`check-range --allow-status --allow-links` と `links` が動いていることを確かめる。

## トレードオフ

- pre-commit フックに `--allow-status` を付けるので、手元の adr-guard が 0.5.0 より古いと、引数の誤りでコミットが止まる。README.md と DEVELOPMENT.md の入れ方のタグを上げて補う。
- ステータスの行の変更を許すかどうかが、フックと action の設定から読めるようになる。その分、設定のオプションが増える。
