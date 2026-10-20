#!/usr/bin/env bash
# Every documentation page is in exactly one book's navigation. mdBook publishes
# only the pages a SUMMARY.md lists, so an unlisted page would exist in the repo
# and silently never reach the site.
#
# Books: docs/user/** is the maker book (docs/user/SUMMARY.md); the rest of docs/
# is the contributor book (docs/SUMMARY.md). docs/_templates/ is never published.
#
# Fails when a page:
#   - is in no SUMMARY.md;
#   - is in both;
#   - is only in the other book's SUMMARY.md.
# A SUMMARY.md entry for a missing file is left to mdBook (create-missing = false).
# Usage: scripts/check-docs-summary.sh [root]   (default: the repository)
# Tests: scripts/tests/check-docs-summary.test.sh
set -euo pipefail
export LC_ALL=C

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"
contributor=docs/SUMMARY.md
maker=docs/user/SUMMARY.md

[[ -f $contributor ]] || { echo "docs-summary: $contributor is missing" >&2; exit 1; }

# normalize <path>: the path with . and .. segments resolved, textually.
normalize() {
  local seg out=()
  local IFS=/
  for seg in $1; do
    case $seg in
      '' | .) ;;
      ..) ((${#out[@]})) && unset 'out[-1]' ;;
      *) out+=("$seg") ;;
    esac
  done
  echo "${out[*]}"
}

# listed <SUMMARY.md>: the repository path of every page it links, one per line.
listed() {
  local dir target
  dir=$(dirname "$1")
  [[ -f $1 ]] || return 0
  grep -oE '\]\([^)#]+\.md(#[^)]*)?\)' "$1" | sed -E 's/^\]\(//; s/(#[^)]*)?\)$//' |
    while IFS= read -r target; do normalize "$dir/$target"; done
}

declare -A in_contributor=() in_maker=()
while IFS= read -r p; do in_contributor[$p]=1; done < <(listed "$contributor")
while IFS= read -r p; do in_maker[$p]=1; done < <(listed "$maker")

errors=0
report() {
  echo "docs-summary: $1" >&2
  errors=$((errors + 1))
}

while IFS= read -r page; do
  page=${page#./}
  if [[ $page == docs/user/* ]]; then own=$maker; else own=$contributor; fi
  c=${in_contributor[$page]:-} m=${in_maker[$page]:-}
  if [[ -n $c && -n $m ]]; then
    report "$page is in both books; list it only in $own"
  elif [[ -z $c && -z $m ]]; then
    report "$page is in no SUMMARY.md; add it to $own"
  elif [[ $own == "$maker" && -z $m ]] || [[ $own == "$contributor" && -z $c ]]; then
    report "$page is in the other book; move it to $own"
  fi
done < <(find ./docs -path ./docs/_templates -prune -o -type f -name '*.md' -print |
  grep -vxE '\./(docs/SUMMARY\.md|docs/user/SUMMARY\.md)' | sort)

((errors == 0)) || { echo "docs-summary: $errors page(s) misplaced in navigation" >&2; exit 1; }
echo "docs-summary: ok"
