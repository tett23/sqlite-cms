# ADR 0077: adr-guard を 0.10.0 に上げ、ADR の番号の重複を調べる

ステータス: 採択

## 文脈

[ADR 0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md) で、adr-guard を 0.8.3 に上げた。

adr-guard は 0.10.0 になった（[リリース v0.9.0](https://github.com/tett23/adr-guard/releases/tag/v0.9.0)、[v0.9.1](https://github.com/tett23/adr-guard/releases/tag/v0.9.1)、[v0.10.0](https://github.com/tett23/adr-guard/releases/tag/v0.10.0)）。

- 0.9.0 で、`adr-guard check-numbers` が、同じディレクトリで番号が重複した ADR を報告するようになった。CI の action は既定でこれを実行し（入力 `check-numbers`）、重複があると失敗する。重複は `adr-guard renumber [--dry-run]` で直せる。番号を付け直すコミットは、`--allow-renumber`（`pre-commit` と `check-range`、action の入力 `allow-renumber`、既定は false）で許す（tett23/adr-guard の [ADR 0018: 重複したADRの番号の修復](https://github.com/tett23/adr-guard/blob/main/docs/adr/0018-repair-duplicate-adr-numbers.md)）。
- 0.9.1 で、`renumber` の誤報が直った。
- 0.10.0 で、修飾子付きの ADR の参照を扱うようになった。参照の直前の語（空白 1 つを挟む）が修飾子で、`/` を含む語はディレクトリ、`-` を含む語はモジュール（`docs/<モジュール>/adr/`）、それ以外の語はそのモジュールがあるときだけ修飾子になる。修飾子はリンクのテキストに含める。`links` と pre-commit は、ディレクトリや ADR がない修飾子付きの参照、リンク先が修飾子の ADR と違うリンク、リンクの外に出た修飾子を報告する。シンボリックリンクの `.md` は調べない（tett23/adr-guard の [ADR 0019: モジュールで修飾したADRの参照](https://github.com/tett23/adr-guard/blob/main/docs/adr/0019-module-qualified-adr-references.md)、[ADR 0020: シンボリックリンクの文書を飛ばす](https://github.com/tett23/adr-guard/blob/main/docs/adr/0020-skip-symlinked-documents.md)）。

このリポジトリの ADR は `docs/adr/` の 1 つのディレクトリだけにあり、モジュールごとの ADR はない。
今の main で、`adr-guard links`、`adr-guard check-status`、`adr-guard check-numbers` は、0.10.0 で終了コード 0 で終わる。
手元の adr-guard はすでに 0.10.0 になっている。

## 実装すること

- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.10.0` にする。`allow-status: true` と `allow-links: true` と `base`、`head` はそのままにし、`check-numbers` と `allow-renumber` は指定しない（番号の重複を既定どおり調べ、番号の付け直しは許さない）。CI の YAML のコメントに `check-numbers` を足し、ADR の番号にこの ADR を足す。
- git の pre-commit フック（`.githooks/pre-commit`）は `exec adr-guard pre-commit --allow-status --allow-links` のままにし、コメントの ADR の番号にこの ADR を足す。
- README.md と DEVELOPMENT.md の adr-guard の版と入れるときのタグを v0.10.0 にし、DEVELOPMENT.md の adr-guard の ADR の一覧にこの ADR を足す。
- DEVELOPMENT.md と README.md に、CI が ADR の番号の重複も調べること（`adr-guard check-numbers`）を書く。DEVELOPMENT.md には、重複は `adr-guard renumber` で直せることと、ほかのリポジトリの ADR を参照するときは外部リンクの中に入れることを書く。
- [ADR 0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md) のステータスを `採択→版は[ADR 0077](0077-adr-guard-0-10-0-and-duplicate-adr-numbers.md)` にする。
- `adr-guard links --fix --include-committed` で、リンクのない ADR の参照を直す（あれば）。

## 実装しないこと

- `docs/specifications.md` の変更（今の adr-guard の版と CI の検査に触れていない）。
- 修飾子付きの参照の規則の詳しい説明（このリポジトリにはモジュールの ADR がない）。
- pre-commit フックと CI で `--allow-renumber` を許すこと（番号を付け直す必要が出たら、そのときに ADR を書く）。
- [ADR 0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md) より前の adr-guard の更新の ADR のステータスの行の変更（直前の adr-guard の更新の ADR だけを改める）。
- Claude Code のフックの変更（利用者の全体の設定にあり、このリポジトリでは管理しない。[ADR 0068](0068-adr-guard-0-4-and-adr-links.md)）。

## テスト設計

このリポジトリには ADR の書式を調べるテストがないので、自動テストは足さない。次のように確かめる。

- `adr-guard links --fix --include-committed` の後、`adr-guard links`、`adr-guard check-status`、`adr-guard check-numbers` が、終了コード 0 で終わること。
- 変更をステージして、手元の adr-guard（0.10.0）の `.githooks/pre-commit` が通ること。この変更には [ADR 0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md) のステータスの行の変更が含まれるので、`--allow-status` が効いていることも確かめられる。
- コミットの後、`adr-guard check-range origin/main HEAD --allow-status --allow-links` が終了コード 0 で終わること。
- `.github/workflows/adr.yml` が YAML として読めること。
- この PR の CI の `adr` のワークフローが、v0.10.0 の action で通ること。

## トレードオフ

- 別のブランチで同じ番号の ADR を足すと、後からマージした側で CI が失敗する。番号の重複を main に入れないことを取る。重複したときは、ブランチの側で番号を付け直す。
- 修飾子付きの参照の検査で、ほかのリポジトリの ADR への参照（`<リポジトリ> ADR NNNN` の形）が、対象のない参照として報告されうる。外部リンクの中に入れて避ける。
- 手元の adr-guard が 0.10.0 より古いと、`adr-guard check-numbers` が使えず、CI と手元の結果が違いうる。README.md と DEVELOPMENT.md の入れ方のタグを上げて補う。

## 参考文献

- tett23/adr-guard, [ADR 0018: 重複したADRの番号の修復](https://github.com/tett23/adr-guard/blob/main/docs/adr/0018-repair-duplicate-adr-numbers.md)
- tett23/adr-guard, [ADR 0019: モジュールで修飾したADRの参照](https://github.com/tett23/adr-guard/blob/main/docs/adr/0019-module-qualified-adr-references.md)
- tett23/adr-guard, [ADR 0020: シンボリックリンクの文書を飛ばす](https://github.com/tett23/adr-guard/blob/main/docs/adr/0020-skip-symlinked-documents.md)
- tett23/adr-guard, [Release v0.9.0](https://github.com/tett23/adr-guard/releases/tag/v0.9.0)
- tett23/adr-guard, [Release v0.9.1](https://github.com/tett23/adr-guard/releases/tag/v0.9.1)
- tett23/adr-guard, [Release v0.10.0](https://github.com/tett23/adr-guard/releases/tag/v0.10.0)
