#!/usr/bin/env bash
# The maker guide is written for someone who sews, not someone who programs.
# Every page under docs/user/ is checked for the two things that tell a maker a
# page was not written for them:
#   - a process ID (a decision, task, requirement or story ID, an ADR number, or
#     a pull request number), anywhere in the text;
#   - code: an inline code span or a fenced block. A maker page names a file or
#     a paper size in plain words, so any code span is a crate, type or
#     identifier that belongs in the contributor book.
# Usage: scripts/check-docs-plain.sh [dir]   (default: docs/user)
# Tests: scripts/tests/check-docs-plain.test.sh
set -euo pipefail
shopt -s inherit_errexit
export LC_ALL=C

dir=${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/docs/user}
[[ -d $dir ]] || { echo "docs-plain: $dir is not a directory" >&2; exit 1; }

# An ID is a prefix the specs use, bounded so that dates, paper sizes and
# counts never match: D-NNN, W-NNN, FR-NNN, SC-NNN, R4, Q2, S1; TNNN, HNNN,
# UNNN, WNNN; US3; ADR NNNN; #NNN.
id='((D|W|H|S|R|Q|FR|SC)-[0-9]+|[THUW][0-9]{3}|US[0-9]|ADR ?[0-9]{4}|[RSQ][0-9]{1,2})'
before='(^|[^A-Za-z0-9_-])'
after='([^A-Za-z0-9-]|$)'

problems=0
while IFS= read -r page; do
  rel=${page#"$dir"/}
  found=$(awk -v rel="$rel" -v id="$id" -v before="$before" -v after="$after" '
    function report(kind, what) { printf "docs-plain: %s:%d: %s %s; a maker page has neither process IDs nor code\n", rel, NR, kind, what; n++ }
    BEGIN { n = 0; fence = "" }
    {
      line = $0
      if (fence != "") {
        if (line ~ ("^ *" fence)) fence = ""
        report("code", "inside a fenced block")
        next
      }
      if (match(line, /^ *(```|~~~)/)) {
        fence = substr(line, RSTART, RLENGTH); sub(/^ */, "", fence)
        report("code", "fenced block")
        next
      }
      if (line ~ /`/) report("code", "span " line)
      rest = line
      while (match(rest, before id after)) {
        hit = substr(rest, RSTART, RLENGTH)
        sub(/^[^A-Za-z0-9#]/, "", hit); sub(/[^A-Za-z0-9]$/, "", hit)
        report("process ID", "\x27" hit "\x27")
        rest = substr(rest, RSTART + RLENGTH - 1)
      }
      rest = line
      while (match(rest, /(^|[[:space:]])#[0-9]+([^A-Za-z0-9-]|$)/)) {
        hit = substr(rest, RSTART, RLENGTH)
        sub(/^[[:space:]]/, "", hit); sub(/[^0-9]$/, "", hit)
        report("process ID", "\x27" hit "\x27")
        rest = substr(rest, RSTART + RLENGTH - 1)
      }
    }
    END { exit n > 0 }' "$page") || true
  [[ -z $found ]] || { echo "$found" >&2; problems=$((problems + $(grep -c . <<<"$found"))); }
done < <(find "$dir" -type f -name '*.md' | sort)

((problems == 0)) || { echo "docs-plain: $problems problem(s) in the maker guide under $dir" >&2; exit 1; }
echo "docs-plain: ok"
