# ADR 0068: adr-guard を 0.4.0 に上げ、文書の ADR の参照をリンクにすることを検査する

ステータス: 採択

## 文脈

[ADR 0064](0064-align-adr-conventions-with-global-rules.md) で、コミットした ADR の書き換えを adr-guard v0.1.0 で止めることにした。
adr-guard は 0.4.0 になり、次の規則と機能が足された。

- 文書（`.md`）の中の ADR の参照（`ADR 0001` の形）には、ADR のファイルへのリンクを付ける。`pre-commit` は、ステージした文書のリンクのない参照を拒む。`adr-guard links` は、作業ツリーのリンクのない参照を知らせ、`--fix` で直す。
- `--allow-links`：コミット済みの ADR で、ADR の参照のリンクだけが違う変更（リンクを付ける、外す、リンク先を変える）を許す。`pre-commit` と `check-range` に付けられる。
- composite action は、`check-range` の後に `adr-guard links` を動かす（入力 `links`）。`--allow-links` は入力 `allow-links: true` で付ける。

このリポジトリでは、文書の ADR の参照をすでにリンクにした（コミット `ad81fb9`。そのときは adr-guard を迂回した）。
`adr-guard links` で調べると、[ADR 0036](0036-機能の統合.md) の題の行の三つ（[ADR 0031](0031-全文検索.md)、[0033](0033-統合一覧とRSS.md)、[0034](0034-serveの自動反映.md)）だけが残っていた。題の行（`# `）を置き換えの対象から外していたためである。

また、利用者のグローバルの CLAUDE.md で、Claude Code の hook は全体の設定（`~/.claude/settings.json`）で有効なので、リポジトリには要らないことになった。
このリポジトリの `.claude/settings.json` には、[ADR 0064](0064-align-adr-conventions-with-global-rules.md) で入れた `adr-guard claude-hook` があり、全体の設定と二重に動く。

## 実装すること

- [ADR 0036](0036-機能の統合.md) の題の行の三つの参照を、`adr-guard links --fix --include-committed` でリンクにする。
- git の pre-commit フック（`.githooks/pre-commit`）を `exec adr-guard pre-commit --allow-links` にする。
- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.4.0` にし、`allow-links: true` を付ける。`adr-guard links`（入力 `links` の既定）で、リンクのない参照も検査する。
- `.claude/settings.json` から `adr-guard claude-hook` を外す。ほかの設定はないので、ファイルごと消す。
- DEVELOPMENT.md と README.md の adr-guard の説明を改める（版、入れ方、`--allow-links`、リンクの規則と `adr-guard links --fix`、Claude Code の hook は全体の設定で動かすこと）。

## 実装しないこと

- 古い形の ADR のステータス（`## 状態` の節）を、新しい形の `ステータス:` の行に改めること。adr-guard が拒む（[ADR 0064](0064-align-adr-conventions-with-global-rules.md) と同じ）。
- 新しいステータスの書き方（`採択→<変わった部分>は[ADR NNNN](...)`）を、既存の ADR に当てること。
- adr-guard の `list`、`proposed` を、このリポジトリの手順に組み込むこと。

## テスト設計

- `adr-guard links` が、リンクのない参照なし（終了コード 0）で終わること。
- `adr-guard check-range --allow-links origin/main HEAD` が、このブランチのコミット（[ADR 0036](0036-機能の統合.md) の題のリンクを含む）を通すこと。
- 一時的なクローンで、`adr-guard pre-commit --allow-links` が次のとおり判断すること。
  - 通すもの：コミット済みの ADR の参照にリンクを付けるだけの変更、新しい ADR を足すこと。
  - 拒むもの：コミット済みの ADR の本文の変更、ステージした文書のリンクのない `ADR 0001` の参照。
- CI の `adr` のワークフローが、この PR で通ること。
- actionlint がワークフローにエラーを出さないこと。

## トレードオフ

- 文書に ADR の番号を書くたびに、リンクを付ける手間が増える（`adr-guard links --fix` で直せる）。
- `--allow-links` で、コミット済みの ADR のリンクだけは書き換えられるようになる。本文の変更は、今までどおり拒む。
- リポジトリから Claude Code の hook を外すので、全体の設定に adr-guard の hook がない人がこのリポジトリで Claude Code を使うと、編集のときには止まらない。pre-commit と CI で止める。
