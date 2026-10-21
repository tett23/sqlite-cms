# ADR 0076: adr-guard を 0.8.3 に上げ、採択（未実装）のステータスを足す

ステータス: 採択

## 文脈

[ADR 0075](0075-adr-guard-0-8-2-and-link-only-changes.md) で、adr-guard を 0.8.2 に上げた。

adr-guard は 0.8.3 になり、ステータスの値に `採択（未実装）` が加わった（tett23/adr-guard の [ADR 0016: 採択（未実装）のステータス](https://github.com/tett23/adr-guard/blob/main/docs/adr/0016-accepted-not-implemented-status.md)、[リリース](https://github.com/tett23/adr-guard/releases/tag/v0.8.3)）。

- `採択（未実装）` は、採択したが、まだ実装していない ADR を表す。括弧は全角でも半角でもよい（英語は `Accepted (not implemented)`）。
- 新しいサブコマンド `adr-guard unimplemented [--status]` が、ステータスが `採択（未実装）` の ADR を一覧する。

利用者の全体の規則でも、ADR のステータスの候補に `採択（未実装）` が加わった。
手元の adr-guard はすでに 0.8.3 になっている。

## 実装すること

- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.8.3` にする。`allow-status: true` と `allow-links: true` と `base`、`head` はそのままにする。CI の YAML のコメントの ADR の番号にこの ADR を足す。
- git の pre-commit フック（`.githooks/pre-commit`）は `exec adr-guard pre-commit --allow-status --allow-links` のままにし、コメントの ADR の番号にこの ADR を足す。
- README.md と DEVELOPMENT.md の adr-guard の版と入れるときのタグを v0.8.3 にし、DEVELOPMENT.md の adr-guard の ADR の一覧にこの ADR を足す。
- DEVELOPMENT.md のステータスの候補に `採択（未実装）` を足し、`adr-guard unimplemented` で一覧できることを書く。README.md の ADR の説明にも、採択したがまだ実装していない ADR は `採択（未実装）` にすることを書く。
- [ADR 0075](0075-adr-guard-0-8-2-and-link-only-changes.md) のステータスを `採択→版は[ADR 0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md)` にする。
- `adr-guard links --fix --include-committed` で、リンクのない ADR の参照を直す（あれば）。

## 実装しないこと

- `docs/specifications.md` の変更（adr-guard の版とステータスの候補に触れていない）。
- 既存の ADR のステータスを `採択（未実装）` に改めること（今のこのリポジトリの採択した ADR は、実装してある）。
- [ADR 0073](0073-adr-guard-0-8-1-and-directory-name-variants.md) より前の adr-guard の更新の ADR のステータスの行の変更（直前の adr-guard の更新の ADR だけを改める）。
- Claude Code のフックの変更（利用者の全体の設定にあり、このリポジトリでは管理しない。[ADR 0068](0068-adr-guard-0-4-and-adr-links.md)）。

## テスト設計

このリポジトリには ADR のステータスの書式を調べるテストがないので、自動テストは足さない。次のように確かめる。

- `adr-guard links --fix --include-committed` の後、`adr-guard links` と `adr-guard check-status` が、終了コード 0 で終わること。
- `adr-guard unimplemented` が、終了コード 0 で終わること。
- 変更をステージして、手元の adr-guard（0.8.3）の `.githooks/pre-commit` が通ること。この変更には [ADR 0075](0075-adr-guard-0-8-2-and-link-only-changes.md) のステータスの行の変更が含まれるので、`--allow-status` が効いていることも確かめられる。
- コミットの後、`adr-guard check-range origin/main HEAD --allow-status --allow-links` が終了コード 0 で終わること。
- `.github/workflows/adr.yml` が YAML として読めること。
- この PR の CI の `adr` のワークフローが、v0.8.3 の action で通ること。

## トレードオフ

- ステータスの候補が 1 つ増え、採択と採択（未実装）の使い分けを書き手が判断する。実装を後に回す ADR を一覧できる利点を取る。
- 手元の adr-guard が 0.8.3 より古いと、`adr-guard unimplemented` が使えない。README.md と DEVELOPMENT.md の入れ方のタグを上げて補う。

## 参考文献

- tett23/adr-guard, [ADR 0016: 採択（未実装）のステータス](https://github.com/tett23/adr-guard/blob/main/docs/adr/0016-accepted-not-implemented-status.md)
- tett23/adr-guard, [Release v0.8.3](https://github.com/tett23/adr-guard/releases/tag/v0.8.3)
