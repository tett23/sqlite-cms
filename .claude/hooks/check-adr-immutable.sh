#!/bin/sh
# PostToolUse のフック。ツールを使った後に、コミット済みの ADR（HEAD にある docs/adr/ のファイル）が
# 書き換えられたか消されたかを調べ、そうなっていればエラー（終了コード 2）にする。
# 編集のツールだけでなく、シェルのコマンド（sed や python など）での書き換えも見つけるため、
# どのツールの後でも作業ツリーと HEAD の差分で判断する。
# コミットしていない ADR（新しく書いたもの）は自由に直してよい。
set -eu

cd "${CLAUDE_PROJECT_DIR:-.}"

# 標準入力（ツールの情報）は使わない。
cat >/dev/null

git rev-parse --verify --quiet HEAD >/dev/null || exit 0

# HEAD にある ADR のうち、作業ツリーかインデックスで書き換えや削除のあるもの。
# 名前の変更は、元の ADR の削除として数える（--no-renames）。日本語のファイル名はそのまま出す。
changed=$(git -c core.quotepath=false diff --no-renames --name-only --diff-filter=MDT HEAD -- docs/adr)
[ -n "$changed" ] || exit 0

{
  echo "コミット済みの ADR が変更されています。コミットした ADR は書き換えません。"
  echo "$changed" | sed 's/^/  - /'
  echo "変更を取り消してください（git restore --source=HEAD --staged --worktree -- <ファイル>）。"
  echo "決定を改めるときは、次の番号で新しい ADR を書き、前の ADR を改めることをそこに書いてください。"
} >&2
exit 2
