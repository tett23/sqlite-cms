#!/bin/sh
# コミット済みの ADR（HEAD にある docs/adr/ のファイル）が変えられていないかを調べる。
# 変えてよいのは「状態」の節（`## 状態` の行から、次の `## ` で始まる行の手前まで）だけで、
# ほかの部分の書き換え、ファイルの削除、名前の変更はだめ。新しい ADR を足すのはよい。
#
#   sh .githooks/check-adr.sh --staged    コミットする中身（インデックス）を調べる（pre-commit）
#   sh .githooks/check-adr.sh --worktree  作業ツリーを調べる（Claude Code の PostToolUse のフック）
#
# 問題があれば、ファイルを標準エラー出力に書いて終了コード 1 で終える。
set -eu

mode=${1:---staged}
case "$mode" in
  --staged) cached=--cached ;;
  --worktree) cached= ;;
  *) echo "usage: check-adr.sh --staged|--worktree" >&2; exit 64 ;;
esac

# 最初のコミット（HEAD がない）では調べない。
git rev-parse --verify --quiet HEAD >/dev/null || exit 0

# 状態の節を取り除く。
strip_status() {
  awk '/^## / { skip = ($0 == "## 状態") } !skip'
}

# 今の中身（インデックスか作業ツリー）。
current() {
  if [ "$mode" = --staged ]; then git show ":$1"; else cat "$1"; fi
}

violations=$(
  # 名前の変更は、元の ADR の削除として数える（--no-renames）。日本語のファイル名はそのまま出す。
  git -c core.quotepath=false diff $cached --no-renames --name-status --diff-filter=MDT HEAD -- docs/adr |
    while IFS="$(printf '\t')" read -r status path; do
      if [ "$status" != M ]; then
        echo "$path"
      elif [ "$(git show "HEAD:$path" | strip_status)" != "$(current "$path" | strip_status)" ]; then
        echo "$path"
      fi
    done
)
[ -n "$violations" ] || exit 0

echo "$violations" | sed 's/^/  - /' >&2
exit 1
