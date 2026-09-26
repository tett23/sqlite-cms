#!/bin/sh
# PostToolUse のフック。ツールを使った後に、コミット済みの ADR が変えられていないかを調べ、
# 変えられていればエラー（終了コード 2）にする。判断は .githooks/check-adr.sh と同じで、
# 「状態」の節だけの変更と、新しい ADR を足すのはよい。
# 編集のツールだけでなく、シェルのコマンドでの書き換えも見つけるため、どのツールの後でも作業ツリーを調べる。
set -u

cd "${CLAUDE_PROJECT_DIR:-.}"

# 標準入力（ツールの情報）は使わない。
cat >/dev/null

if ! violations=$(sh .githooks/check-adr.sh --worktree 2>&1); then
  {
    echo "コミット済みの ADR が、状態の節のほかで変更されています。コミットした ADR は書き換えません。"
    echo "$violations"
    echo "変更を取り消してください（git restore --source=HEAD --staged --worktree -- <ファイル>）。"
    echo "決定を改めるときは、次の番号で新しい ADR を書き、前の ADR を改めることをそこに書いてください。"
  } >&2
  exit 2
fi
exit 0
