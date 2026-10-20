#!/usr/bin/env bash
# Tests for scripts/check-docs-facts.sh: each case builds a throwaway tree of pages
# and asserts whether the check accepts or rejects it, and what it prints.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-docs-facts.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

# expect <pass|fail> <case name> <root> [<text the output must contain>]
expect() {
  local want=$1 name=$2 root=$3 pattern=${4:-}
  local out status=0
  out=$("$check" "$root" 2>&1) || status=$?
  if [[ $want == pass && $status -eq 0 || $want == fail && $status -ne 0 ]] &&
    [[ -z $pattern || $out == *"$pattern"* ]]; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (wanted $want, exit $status)"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# refute <case name> <root> <text the output must not contain>
refute() {
  local name=$1 root=$2 pattern=$3 out
  out=$("$check" "$root" 2>&1) || true
  if [[ $out != *"$pattern"* ]]; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (output names '$pattern')"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# new_case <name>: a tree whose checked files hold nothing to flag.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root/docs/start" "$root/specs"
  printf '# OpenPlush\n\nHow far each feature has got is on the status page.\n' >"$root/README.md"
  printf '# Status\n\nThe layout stage is not built yet; printing is planned.\n' >"$root/docs/start/status.md"
  echo "$root"
}

# write <root> <path> <lines>...: write a file of those lines.
write() {
  local file="$1/$2"
  shift 2
  mkdir -p "$(dirname "$file")"
  printf '%s\n' "$@" >"$file"
}

counts_table=("| Feature | Tasks ticked |" "|---|---|" "| 001 pattern pipeline | 102 of 189 |")

r=$(new_case clean)
expect pass "nothing to flag" "$r" "docs-facts: ok"

r=$(new_case work-items)
write "$r" README.md "# OpenPlush" "" "| [Tasks](specs/001/tasks.md) | 179 work items |"
expect fail "a copied count of work items" "$r" "README.md:3:"
expect fail "a copied count names the fix" "$r" "docs/start/status.md"

r=$(new_case tasks)
write "$r" CONTRIBUTING.md "Feature 002 has 82 tasks left."
expect fail "a copied count of tasks" "$r" "CONTRIBUTING.md:1:"

r=$(new_case one-task)
write "$r" docs/how-to/start.md "Only 1 task is open."
expect fail "a copied count of one task" "$r" "docs/how-to/start.md:1:"

r=$(new_case specs-index)
write "$r" specs/README.md "| 004 docs site | 43 tasks |"
expect fail "a copied count in the specs index" "$r" "specs/README.md:1:"

r=$(new_case feature-number)
write "$r" docs/adr/0011-tool.md "* 003 research R6; 003 tasks U008, U009."
expect pass "a zero-padded feature number before tasks is not a count" "$r"

r=$(new_case ticked-line)
write "$r" docs/start/README.md "Feature 001 has 102 of 189 tasks ticked."
expect fail "N of M on a line about tasks" "$r" "docs/start/README.md:1:"

r=$(new_case ticked-sentence-end)
write "$r" docs/start/README.md "Ticked so far: 102 of 189."
expect fail "N of M at a sentence end on a line about ticks" "$r" "docs/start/README.md:1:"

r=$(new_case tasks-table)
write "$r" docs/start/progress.md "# Progress" "" "${counts_table[@]}" "" "Done when every row is full."
expect fail "N of M in a table headed Tasks" "$r" "docs/start/progress.md:5:"

r=$(new_case include)
write "$r" docs/start/status-counts.inc "<!-- Generated; do not edit. -->" "" "${counts_table[@]}"
expect pass "the generated include is ignored" "$r"

r=$(new_case hand-include)
write "$r" docs/start/other-counts.inc "${counts_table[@]}"
expect fail "a hand-kept include is checked" "$r" "docs/start/other-counts.inc:3:"

r=$(new_case prose-of)
write "$r" docs/adr/0007-nesting.md "The half turn was used on 3 of 6 pieces." "" \
  "| Piece | Turns |" "|---|---|" "| ear | 1 of 2 |"
expect pass "N of M outside a task context" "$r"

r=$(new_case table-ends)
write "$r" docs/start/progress.md "${counts_table[0]}" "${counts_table[1]}" "| 001 | all |" "" \
  "Step 1 of 3 is the import."
expect pass "a tasks table ends at its last row" "$r"

r=$(new_case dates-paper)
write "$r" docs/user/print.md "Tasks ticked on 2026-10-20 and 2026-10-21." \
  "Print the tasks sheet on A4 or Letter, 2 copies of A4." \
  "The task runner needs mdbook 0.5.4 of 2026-10-20 and lychee 0.24.2." \
  "Tasks: version 1.2 of 3.4 is pinned." "Tasks 10-20 of 2026-10-20." \
  "| Tasks | Paper |" "|---|---|" "| 2026-10-20 | A4 of Letter |"
expect pass "dates, versions and paper sizes never match" "$r"

r=$(new_case out-of-scope)
write "$r" specs/001/tasks.md "179 work items, 102 of 189 tasks ticked."
write "$r" docs/notes.txt "179 work items"
expect pass "files outside the checked set are not read" "$r"

r=$(new_case warn)
write "$r" docs/explanation/pipeline.md "# Pipeline" "" "The layout stage is not built."
expect pass "a status claim only warns" "$r" "docs/explanation/pipeline.md:3:"
expect pass "the warnings are counted" "$r" "1 status claim"
refute "the status page may hold status claims" "$r" "docs/start/status.md:"

r=$(new_case warn-each)
write "$r" README.md "Printing is Planned." "Seams are not yet drawn."
write "$r" docs/reference/cli.md "An unplanned stop is reported, as is a notyet flag."
expect pass "each status phrase warns" "$r" "README.md:1:"
expect pass "not yet warns" "$r" "README.md:2:"
expect pass "warnings count every hit" "$r" "2 status claim"
refute "a word that only contains a phrase does not warn" "$r" "cli.md"

r=$(new_case warn-and-fail)
write "$r" README.md "Printing is planned." "There are 179 work items."
expect fail "a failure still lists the warnings" "$r" "README.md:1:"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
