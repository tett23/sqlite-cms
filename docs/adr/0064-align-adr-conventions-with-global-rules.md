# ADR 0064: ADR の決まりを、グローバルの CLAUDE.md にそろえ、adr-guard で守る

ステータス: 採択

## 文脈

利用者のグローバルの CLAUDE.md に、ADR の決まりが書かれた。要点は次のとおり。

- 実装の前に ADR を書き、人間のレビューを受けるまでコミットしない。
- ファイル名は `0001-feature-details.md` のように、連番の後を kebab-case にする。
- 中身は `# ADR 0001: タイトル`、`ステータス:` の行、文脈、実装すること、実装しないこと、テスト設計、トレードオフの節で書く。
- コミットした ADR は、ステータスの行を除いて変えない。仕様を変えるときは、新しい ADR を書く。
- 破棄するときは、ステータスを「破棄」にして、理由を docs/specifications.md に書く。
- 書き換えを防ぐ方法は、[tett23/adr-guard](https://github.com/tett23/adr-guard) の README に従う。
- ADR で仕様が変わったら、specifications.md と README.md も同じときに直す。

adr-guard は、`ステータス:` で始まる行のほかの変更、削除、名前の変更を、すべて違反とする。
git の pre-commit フック、Claude Code の PreToolUse フック、CI（`check-range`）の三か所で守る。

このリポジトリの今の状態は、次の点で決まりとずれている。

1. ファイル名：0063 のほかの 61 件は、日本語のファイル名である。
2. 中身の形：0063 のほかは、`# 0001 タイトル` と `## 状態`、`## 決定` などの古い形である。
3. ステータス：0038 から 0062 は、実装してリリースした後も、状態の節が「提案。」のままである。
4. 破棄した ADR：0032 と 0035 の経緯は ADR 0036 の中にしかない。0061 の経緯は 0061 の中にしかない。どれも specifications.md に理由が書かれていない。
5. 書き換えを防ぐ仕組み：自前のフック（`.githooks/check-adr.sh` と `.claude/hooks/check-adr-immutable.sh`）で守っていて、adr-guard を使っていない。CI では検査していない。

1 から 3 は、コミットした ADR の名前の変更か、`ステータス:` の行のほかの変更になるので、adr-guard の決まりでは直せない。

## 実装すること

### 破棄した ADR の理由を仕様書に書く

- specifications.md に「破棄した ADR」の節を作り、0032、0035、0061 の経緯と理由を書く（0032 と 0035 は ADR 0036 から、0061 は ADR 0061 から写す）。
- 0061 のファイルは残す。状態の節はすでに「破棄。」である。0032 と 0035 は、もともとファイルがない。
- specifications.md の「未決事項」の、X などのカードの項目に、ページごとの OGP を試して採らなかったこと（0061）を書き足す。

### 古い形の ADR のステータスを仕様書に書く

古い形の ADR（0001 から 0062）は、状態の節を書き換えられない。そこで、specifications.md の「破棄した ADR」の前に「古い形の ADR」の節を作り、次を書く。

- 0063 より前の ADR は、日本語のファイル名と、状態の節の古い形のまま残す。
- 0038 から 0060 と 0062 は、状態の節が「提案。」のままだが、実装してリリースしたので採択とみなす。
- 0001 から 0037 の「採用。」は、採択と同じ意味である。

### adr-guard に置き換える

adr-guard の README の「リポジトリへの導入」に従う。版は v0.1.0 にする。

- git の pre-commit フック：`.githooks/pre-commit` を `exec adr-guard pre-commit` にする。`git config core.hooksPath .githooks` で有効にするのは、今までどおり。
- Claude Code のフック：`.claude/settings.json` の PostToolUse の自前のフックをやめ、PreToolUse で `Write|Edit|MultiEdit` に `adr-guard claude-hook` を掛ける。
- CI：ADR を検査するワークフロー `.github/workflows/adr.yml` を足し、`tett23/adr-guard@v0.1.0` の composite action で `check-range` を動かす。`ci.yml` は、ドキュメントだけの変更では動かない（ADR 0044）ので、別のワークフローにして、パスで絞らない。`main` への push とプルリクエストで動かす。
- 自前の `.githooks/check-adr.sh` と `.claude/hooks/check-adr-immutable.sh` を消す。

### 文書

- DEVELOPMENT.md の「ADR」の節を、決まりに合わせて書き改める（ファイル名の付け方、新しい ADR の形、ステータスの書き方、レビューの前にコミットしないこと、破棄した ADR の扱い、adr-guard の入れ方と有効にする方法、古い形の ADR のこと）。
- README.md に「設計の記録（ADR）」の節を足し、ADR の置き場所と決まりの要点、adr-guard を入れて `core.hooksPath` を設定する手順を書く。

## 実装しないこと

- 今までの ADR のファイル名を kebab-case に付け直すこと。adr-guard が名前の変更を拒む。
- 今までの ADR の状態の節を書き換えること（「提案。」を「採択。」にすることを含む）。adr-guard が拒む。
- 今までの ADR の本文を、新しい形に書き直すこと。
- 0061 を消して欠番にすること。破棄した ADR は、ステータスを「破棄」にして残す。
- 自前のフックを残して adr-guard と併用すること。

## テスト設計

- 手元で、`adr-guard pre-commit` が、次のステージした変更を拒むこと：コミット済みの ADR の本文の変更、名前の変更、削除。新しい ADR を足すことは通すこと。
- Claude Code で、コミット済みの ADR を Edit しようとすると、フックが止めること。
- `adr-guard check-range origin/main HEAD` が、このブランチのコミットを通すこと。
- CI の `adr` のワークフローが、このブランチのプルリクエストか main への push で通ること。
- actionlint がワークフローにエラーを出さないこと。

## トレードオフ

- 0063 より前の ADR は、ファイル名も形も決まりに合わないまま残る。今のステータスは、ADR のファイルではなく specifications.md を見ないと分からない。
- adr-guard の Claude Code のフックは、Write・Edit・MultiEdit だけを見る。今の自前のフックはシェルのコマンドでの書き換えも見つけていたが、それはなくなる。代わりに、pre-commit フックと CI で止める。
- 手元に adr-guard を入れないと、コミットできなくなる（pre-commit フックがコマンドを見つけられない）。
- CI では、範囲の中で一度でも ADR を書き換えたコミットがあると、後で戻しても失敗する。ブランチのうちに履歴を直す（マージする前の rebase は許される）。
