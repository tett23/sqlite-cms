#!/bin/sh
# 変わったファイルの一覧（標準入力、1 行に 1 つ）から、CI のジョブが要るかを決め、true か false を出す（ADR 0070）。
#   git diff --name-only BASE...HEAD | sh .github/scripts/ci-needs.sh test
#   git diff --name-only BASE...HEAD | sh .github/scripts/ci-needs.sh browser
# test：SPA のビルド、vitest、clippy、cargo test。browser：E2E、Lighthouse、Core Web Vitals。
# 一覧が空（差分を取れなかった）なら、どちらも要るとみなす。
set -eu

kind=${1:-}
case "$kind" in
  test | browser) ;;
  *) echo "usage: ci-needs.sh test|browser" >&2; exit 64 ;;
esac

needed=false
seen=false
while IFS= read -r file; do
  [ -n "$file" ] || continue
  seen=true
  case "$file" in
    # 動かさなくてよいもの（表示にもテストにも関わらない）。
    docs/* | README.md | DEVELOPMENT.md | .githooks/* | .github/workflows/adr.yml | .github/workflows/release.yml | .github/scripts/* | .claude/* | .gitignore | LICENSE)
      ;;
    # テスト。test は要るが、browser は要らない。Rust の単体テストは本体と同じファイルにあるので、cli/src/ は含めない。
    web/src/*.test.ts | web/src/*.test.tsx | web/scripts/*.test.mjs | cli/tests/*)
      [ "$kind" = test ] && needed=true
      ;;
    *)
      needed=true
      ;;
  esac
done

[ "$seen" = true ] || needed=true
echo "$needed"
