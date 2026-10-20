#!/usr/bin/env bash
# Tests for scripts/check-docs-plain.sh: each case writes a throwaway maker page
# and asserts whether the check accepts or rejects it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-docs-plain.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

# expect <pass|fail> <case name> <dir> [<text the failure output must contain>]
expect() {
  local want=$1 name=$2 dir=$3 pattern=${4:-}
  local out status=0
  out=$("$check" "$dir" 2>&1) || status=$?
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

# new_case <name> <lines>...: a maker book holding one page made of the lines.
new_case() {
  local dir="$tmp/$1"
  shift
  mkdir -p "$dir"
  printf '%s\n' "# Summary" "" "[Welcome](README.md)" "- [Questions](faq.md)" >"$dir/SUMMARY.md"
  printf '%s\n' "# Welcome" "" "OpenPlush turns a 3D model into a sewing pattern you print." >"$dir/README.md"
  printf '%s\n' "# Questions" "" "$@" >"$dir/faq.md"
  echo "$dir"
}

d=$(new_case plain "## Is it free?" "" "Yes. It costs nothing, and the pattern is yours." "" \
  "Models saved as OBJ, STL or PLY files open (model.obj, for example)." "" \
  "Print on Letter or A4 paper; the 10 cm square on page 1 checks the scale." "" \
  "The licence is [in the repository](https://github.com/openplush/openplush/blob/main/LICENSE-MIT#L1)." "" \
  "See [the fabric question](#which-fabric-works) and the [10 steps](steps.md#1-open-a-model).")
expect pass "plain maker prose, file names, paper sizes, anchors and links" "$d"

d=$(new_case empty-book)
rm "$d/faq.md"
sed -i '/faq.md/d' "$d/SUMMARY.md"
expect pass "a book with only its landing page" "$d"

for id in "W016" "W-003" "D-012" "T042" "H105" "U007" "FR-030" "SC-006" "US3" "ADR 0017" "R4" "Q2" "S1"; do
  d=$(new_case "id-${id// /-}" "This is decided in $id, which you can read.")
  expect fail "a process ID: $id" "$d" "faq.md:3: process ID '$id'"
done

d=$(new_case pr-number "Fixed in #288 last week.")
expect fail "a pull request number" "$d" "faq.md:3: process ID '#288'"

d=$(new_case id-in-heading "## What W016 changed")
expect fail "a process ID in a heading" "$d" "faq.md:3"

d=$(new_case id-in-summary "Plain text.")
printf '%s\n' "- [W016 notes](notes.md)" >>"$d/SUMMARY.md"
printf '%s\n' "# Notes" >"$d/notes.md"
expect fail "a process ID in the navigation" "$d" "SUMMARY.md:5: process ID 'W016'"

d=$(new_case not-ids "US Letter and A4 paper, the F-150 model and 3 of 36 pages are fine; so is 2026-10-20.")
expect pass "words that only look like IDs" "$d"

for span in '`openplush-flatten`' '`Mm::checked_add`' '`GeomError`' '`seam_allowance`' '`flattenChart()`' '`validate`' '``a `b` c``'; do
  d=$(new_case "span-$RANDOM" "The engine calls $span here.")
  expect fail "a code span: $span" "$d" "faq.md:3: code"
done

d=$(new_case fence '```rust' 'let x = 1;' '```')
expect fail "a fenced code block" "$d" "faq.md:3: code"

d=$(new_case tilde-fence '~~~' 'cargo run' '~~~')
expect fail "a tilde-fenced code block" "$d" "faq.md:3: code"

d=$(new_case two-pages "Plain text.")
printf '%s\n' "# Steps" "" "Open the model, then run \`openplush\`." >"$d/steps.md"
expect fail "a code span on a page the navigation does not list" "$d" "steps.md:3: code"

d=$(new_case counts "First W016 and then D-012." "Then \`code\`.")
out=$("$check" "$d" 2>&1) || true
if [[ $out == *"3 problem(s)"* ]]; then
  pass=$((pass + 1)); echo "ok   every problem is counted"
else
  fail=$((fail + 1)); echo "FAIL every problem is counted"; echo "     | $out"
fi

expect fail "a missing directory" "$tmp/nowhere" "not a directory"

echo "check-docs-plain: $pass passed, $fail failed"
((fail == 0))
