#!/usr/bin/env bash
# Task counts copied into prose drift each time a task is ticked, and a review of
# the diff never sees the page the tick made wrong. The counts are generated into
# the status page; every other page links to it.
#
# Reads README.md, CONTRIBUTING.md, specs/README.md and every *.md and *.inc under
# docs/ (mdBook renders both), except the generated docs/start/status-counts.inc
# and docs/{adr,explanation,how-to,reference,tutorials}/index.inc, whose text is
# copied from pages this check reads.
#
# Fails on a copied task count, case-insensitive:
#   - a number then "task(s)" or "work item(s)". A zero-padded number is a feature
#     ("003 tasks U008"), not a count, so it never matches.
#   - "N of M" on a line that says "task", "ticked" or "work item", or in a table
#     whose header row says so. A table ends at its first line not starting "|".
# A number counts only when it stands alone: a letter, digit, "_", "-" or "." on
# either side, as in a date (2026-10-20), a version (0.5.4) or a paper size (A4),
# rules it out; a "." then a non-digit is the end of a sentence and still counts.
#
# Warns, without failing, on "not built", "not yet" and "planned" anywhere but the
# status page (docs/start/status.md), and prints how many.
# Usage: scripts/check-docs-facts.sh [root]   (default: the repository)
# Tests: scripts/tests/check-docs-facts.test.sh
set -euo pipefail
export LC_ALL=C

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"
status_page=docs/start/status.md
generated='docs/start/status-counts\.inc|docs/(adr|explanation|how-to|reference|tutorials)/index\.inc'

# checked: every file this check reads, one per line, sorted.
checked() {
  local f
  for f in README.md CONTRIBUTING.md specs/README.md; do
    [[ -f $f ]] && echo "$f"
  done
  [[ -d docs ]] && find docs -type f \( -name '*.md' -o -name '*.inc' \) | grep -vxE "$generated"
  return 0
}

# scan <file>...: one line per hit, "fail" or "warn", a tab, then file:line: text.
scan() {
  awk -v status_page="$status_page" '
    function hit(kind, text) { printf "%s\t%s:%d: %s\n", kind, FILENAME, FNR, text }
    FNR == 1 { in_table = 0; task_table = 0 }
    {
      line = tolower($0)
      if (line !~ /^[ \t]*\|/) { in_table = 0; task_table = 0 }
      else if (!in_table) { in_table = 1; task_table = (line ~ context) }
      if (line ~ (lead "[1-9][0-9]* (tasks?|work items?)" trail)) hit("fail", $0)
      else if (line ~ (lead "[0-9]+ of [0-9]+" trail) && (task_table || line ~ context)) hit("fail", $0)
      if (FILENAME != status_page && match(line, lead "(not built|not yet|planned)" trail)) {
        claim = substr(line, RSTART, RLENGTH)
        match(claim, /not built|not yet|planned/)
        hit("warn", "\"" substr(claim, RSTART, RLENGTH) "\"")
      }
    }
  ' lead='(^|[^[:alnum:]_.-])' trail='([^[:alnum:]_.-]|[.]([^0-9]|$)|$)' \
    context='task|ticked|work item' "$@"
}

files=()
while IFS= read -r f; do files+=("$f"); done < <(checked | sort)
((${#files[@]})) || { echo "docs-facts: ok (no files to check)"; exit 0; }

hits=$(scan "${files[@]}")
fails=$(grep -c '^fail' <<<"$hits" || true)
warns=$(grep -c '^warn' <<<"$hits" || true)

if ((warns)); then
  grep '^warn' <<<"$hits" | cut -f2- | sed 's/^/docs-facts: warning: /'
  echo "docs-facts: $warns status claim(s) outside $status_page; the status page is their one home"
fi
if ((fails)); then
  grep '^fail' <<<"$hits" | cut -f2- | sed 's/^/docs-facts: /' >&2
  echo "docs-facts: $fails copied task count(s); link to the status page ($status_page) instead," \
    "whose counts scripts/gen-status.py generates" >&2
  exit 1
fi
echo "docs-facts: ok"
