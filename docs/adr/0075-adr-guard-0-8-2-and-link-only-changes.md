# ADR 0075: adr-guard を 0.8.2 に上げ、コミット済みの ADR のリンクだけの変更を許す

ステータス: 採択→版は[ADR 0076](0076-adr-guard-0-8-3-and-accepted-not-implemented-status.md)

## 文脈

[ADR 0073](0073-adr-guard-0-8-1-and-directory-name-variants.md) で、adr-guard を 0.8.1 に上げた。

adr-guard は 0.8.2 になり、`--allow-links` が許す変更が広がった（tett23/adr-guard の [ADR 0015](https://github.com/tett23/adr-guard/blob/main/docs/adr/0015-allow-link-changes.md)、[リリース](https://github.com/tett23/adr-guard/releases/tag/v0.8.2)）。

- 0.8.1 までは、ADR の参照（`ADR 0001`）のリンクだけを変える書き換えを許した。
- 0.8.2 からは、コミット済みの ADR の、リンクだけの変更をすべて許す。インラインリンクと参照形式のリンクの追加、削除、リンク先の変更、画像のリンク先の変更、リンクの参照定義の行（`[1]: url`）の変更である。
- リンクのテキストの変更、画像の追加と削除、自動リンク（`<url>`）や本文の URL の変更は、これまでどおり拒む。記録（`docs/audit/` など）は、これまでどおりすべての変更を拒む。

手元の adr-guard はすでに 0.8.2 になっている。
このリポジトリの pre-commit フックと CI はすでに `--allow-links` を付けているので、上げると、コミット済みの ADR のリンク先の修正（切れたリンクの付け替えなど）が通るようになる。

## 実装すること

- CI（`.github/workflows/adr.yml`）の action を `tett23/adr-guard@v0.8.2` にする。`allow-status: true` と `allow-links: true` はそのままにする。
- git の pre-commit フック（`.githooks/pre-commit`）は `exec adr-guard pre-commit --allow-status --allow-links` のままにし、コメントの ADR の番号にこの ADR を足し、`--allow-links` の説明を「リンクだけの変更」に改める。
- README.md と DEVELOPMENT.md の adr-guard の説明を改める（版と入れるときのタグを v0.8.2 にする、`--allow-links` が許すものを「ADR の参照のリンクだけを変える書き換え」から「リンクだけの変更」にする）。
- CI の YAML のコメントの `--allow-links` の説明と ADR の番号も同じように改める。
- [ADR 0073](0073-adr-guard-0-8-1-and-directory-name-variants.md) のステータスを `採択→版は[ADR 0075](0075-adr-guard-0-8-2-and-link-only-changes.md)` にする。
- `adr-guard links --fix --include-committed` で、リンクのない ADR の参照を直す（あれば）。

## 実装しないこと

- `docs/specifications.md` の変更（adr-guard の版と `--allow-links` に触れていない）。
- [ADR 0064](0064-align-adr-conventions-with-global-rules.md)、[0068](0068-adr-guard-0-4-and-adr-links.md)、[0069](0069-adr-guard-0-5-and-allow-status.md)、[0072](0072-adr-guard-0-8-and-status-lines.md) のステータスの行の変更（直前の adr-guard の更新の ADR だけを改める）。
- コミット済みの ADR の切れたリンクを探して直すこと（必要になったときに、別に行う）。
- Claude Code のフックの変更（利用者の全体の設定にあり、このリポジトリでは管理しない。[ADR 0068](0068-adr-guard-0-4-and-adr-links.md)）。

## テスト設計

- `adr-guard links --fix --include-committed` の後、`adr-guard links` と `adr-guard check-status` が、終了コード 0 で終わること。
- 変更をステージして、手元の adr-guard（0.8.2）の `.githooks/pre-commit` が通ること。この変更には [ADR 0073](0073-adr-guard-0-8-1-and-directory-name-variants.md) のステータスの行の変更が含まれるので、`--allow-status` が効いていることも確かめられる。
- コミットの後、`adr-guard check-range origin/main HEAD --allow-status --allow-links` が終了コード 0 で終わること。
- `.github/workflows/adr.yml` が YAML として読めること。
- この PR の CI の `adr` のワークフローが、v0.8.2 の action で通ること。

## トレードオフ

- コミット済みの ADR のリンク先を、意味の変わる先に付け替えても止まらない。リンクのテキストは変えられないので、何を指すかは本文に残る。レビューで見る。
- 手元の adr-guard が 0.8.2 より古いと、リンクだけの変更が手元の pre-commit フックで止まる。README.md と DEVELOPMENT.md の入れ方のタグを上げて補う。

## 参考文献

- tett23/adr-guard, [ADR 0015: allow link changes](https://github.com/tett23/adr-guard/blob/main/docs/adr/0015-allow-link-changes.md)
- tett23/adr-guard, [Release v0.8.2](https://github.com/tett23/adr-guard/releases/tag/v0.8.2)
