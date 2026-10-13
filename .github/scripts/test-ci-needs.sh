#!/bin/sh
# ci-needs.sh の判断を確かめる（ADR 0070）。CI の changes のジョブで動かす。
#   sh .github/scripts/test-ci-needs.sh
set -eu

needs=$(dirname "$0")/ci-needs.sh
failures=0

# expect <test が要るか> <browser が要るか> <説明> <ファイル…>
expect() {
  want_test=$1
  want_browser=$2
  name=$3
  shift 3
  files=$(printf '%s\n' "$@")
  got_test=$(printf '%s\n' "$files" | sh "$needs" test)
  got_browser=$(printf '%s\n' "$files" | sh "$needs" browser)
  if [ "$got_test $got_browser" != "$want_test $want_browser" ]; then
    echo "✗ ${name}：test=${want_test} browser=${want_browser} のはずが test=${got_test} browser=${got_browser}" >&2
    failures=$((failures + 1))
  fi
}

expect false false "文書だけ" docs/adr/0001-a.md README.md DEVELOPMENT.md
expect false false "hook と adr.yml だけ" .githooks/pre-commit .github/workflows/adr.yml .github/workflows/release.yml
expect false false "このスクリプトだけ" .github/scripts/ci-needs.sh
expect false false "そのほかの開発の手順" .claude/launch.json .gitignore LICENSE
expect true false "テストだけ" web/src/a.test.ts web/src/render/A.test.tsx cli/tests/cli.rs web/scripts/cdp.test.mjs
expect true false "テストと文書" web/src/a.test.ts docs/specifications.md
expect true true "SPA の本体" web/src/a.ts
expect true true "Rust のファイル（テストだけの変更でも）" cli/src/codeembed.rs
expect true true "見本の記事" example/content/index.md
expect true true "マイグレーション" migrations/0012_code_embeds.sql
expect true true "依存" web/package.json
expect true true "CI のワークフロー" .github/workflows/ci.yml
expect true true "E2E の本体" web/scripts/e2e.mjs
expect true true "テストと本体" web/src/a.test.ts web/src/a.ts
expect true true "空の一覧（差分を取れないとき）"

if [ "$failures" -gt 0 ]; then
  echo "ci-needs.sh の判断が $failures 件違う" >&2
  exit 1
fi
echo "ci-needs.sh の判断はすべて正しい"
