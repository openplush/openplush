#!/usr/bin/env bash
# Tests for scripts/check-docs-summary.sh: each case builds a throwaway docs tree
# and asserts whether the check accepts or rejects it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-docs-summary.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

# expect <pass|fail> <case name> <root> [<text the failure output must contain>]
expect() {
  local want=$1 name=$2 root=$3 pattern=${4:-}
  local out status=0
  out=$("$check" "$root" 2>&1) || status=$?
  if [[ $want == pass && $status -eq 0 ]] ||
    [[ $want == fail && $status -ne 0 && ( -z $pattern || $out == *"$pattern"* ) ]]; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (wanted $want, exit $status)"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# page <root> <path under docs/>: create an empty page.
page() { mkdir -p "$(dirname "$1/docs/$2")" && printf '# %s\n' "$2" >"$1/docs/$2"; }

# summary <root> <path of SUMMARY.md under docs/> <lines>...: write a SUMMARY.md.
summary() {
  local file="$1/docs/$2"
  shift 2
  mkdir -p "$(dirname "$file")"
  printf '%s\n' "# Summary" "" "$@" >"$file"
}

# new_case <name>: a complete tree with both books.
new_case() {
  local root="$tmp/$1"
  page "$root" README.md
  page "$root" how-to/README.md
  page "$root" how-to/write-docs.md
  page "$root" user/README.md
  page "$root" user/faq.md
  summary "$root" SUMMARY.md "[Introduction](README.md)" "" \
    "- [How-to guides](how-to/README.md)" "  - [Write docs](./how-to/write-docs.md#1-pick-the-audience)"
  summary "$root" user/SUMMARY.md "[Introduction](README.md)" "- [FAQ](faq.md)" "- [Draft]()"
  echo "$root"
}

r=$(new_case complete)
expect pass "a complete tree" "$r"

r=$(new_case contributor-only)
rm -r "$r/docs/user"
expect pass "a tree with no maker book" "$r"

r=$(new_case unlisted)
page "$r" explanation/seams.md
expect fail "a contributor page in no SUMMARY.md" "$r" "docs/explanation/seams.md"

r=$(new_case unlisted-maker)
page "$r" user/print.md
expect fail "a maker page in no SUMMARY.md" "$r" "docs/user/print.md"

r=$(new_case no-maker-summary)
rm "$r/docs/user/SUMMARY.md"
expect fail "maker pages without docs/user/SUMMARY.md" "$r" "docs/user/faq.md"

r=$(new_case no-summary)
rm "$r/docs/SUMMARY.md"
expect fail "a missing docs/SUMMARY.md" "$r" "docs/SUMMARY.md"

r=$(new_case both)
printf '%s\n' "- [FAQ](user/faq.md)" >>"$r/docs/SUMMARY.md"
expect fail "a page in both books" "$r" "docs/user/faq.md"
expect fail "a page in both books says so" "$r" "both"

r=$(new_case both-contributor)
printf '%s\n' "- [Write docs](../how-to/write-docs.md)" >>"$r/docs/user/SUMMARY.md"
expect fail "a contributor page also in the maker book" "$r" "docs/how-to/write-docs.md"

r=$(new_case maker-in-contributor)
summary "$r" user/SUMMARY.md "[Introduction](README.md)"
printf '%s\n' "- [FAQ](user/faq.md)" >>"$r/docs/SUMMARY.md"
expect fail "a maker page only in the contributor book" "$r" "docs/user/faq.md"

r=$(new_case contributor-in-maker)
summary "$r" SUMMARY.md "[Introduction](README.md)" "- [How-to guides](how-to/README.md)"
printf '%s\n' "- [Write docs](../how-to/write-docs.md)" >>"$r/docs/user/SUMMARY.md"
expect fail "a contributor page only in the maker book" "$r" "docs/how-to/write-docs.md"

r=$(new_case templates)
page "$r" _templates/how-to.md
page "$r" _templates/maker-task.md
expect pass "templates need no navigation" "$r"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
