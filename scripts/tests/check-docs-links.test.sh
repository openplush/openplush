#!/usr/bin/env bash
# Tests for scripts/check-docs-links.sh: each case builds a throwaway mdBook and
# asserts whether the check accepts or rejects the built site.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-docs-links.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

# expect <pass|fail> <case name> <dir> [<text the failure output must contain>...]
expect() {
  local want=$1 name=$2 dir=$3
  shift 3
  local out status=0 pattern matched=1
  out=$("$check" "$dir" 2>&1) || status=$?
  for pattern in "$@"; do [[ $out == *"$pattern"* ]] || matched=0; done
  if [[ $want == pass && $status -eq 0 ]] || [[ $want == fail && $status -ne 0 && $matched -eq 1 ]]; then
    pass=$((pass + 1))
    echo "ok   $name"
  else
    fail=$((fail + 1))
    echo "FAIL $name (wanted $want, exit $status)"
    while IFS= read -r line; do echo "     | $line"; done <<<"$out"
  fi
}

# file <dir> <path> <content>: write a file of the book's source.
file() { mkdir -p "$(dirname "$1/$2")" && printf '%s\n' "$3" >"$1/$2"; }

# new_book <name>: the source of a book whose every link and anchor resolves.
new_book() {
  local dir="$tmp/$1"
  file "$dir" book.toml '[book]
title = "Fixture"
src = "src"

[build]
build-dir = "site"
create-missing = false'
  file "$dir" src/SUMMARY.md '# Summary

- [Intro](intro.md)
- [Guide](guide/seams.md)'
  file "$dir" src/intro.md '# Intro

See [seams](guide/seams.md) and [the allowance](guide/seams.md#seam-allowance).'
  file "$dir" src/guide/seams.md '# Seams

## Seam allowance

Back to [the intro](../intro.md) or [this section](#seam-allowance).'
  echo "$dir"
}

# build <dir>: build the book and print the site directory.
build() {
  mdbook build "$1" >"$tmp/mdbook.log" 2>&1 || { cat "$tmp/mdbook.log" >&2; exit 1; }
  echo "$1/site"
}

# The clean build also carries mdBook's 404.html, whose <base href="/"> only
# resolves when the check knows the site's root.
d=$(new_book clean)
expect pass "a book whose links and anchors all resolve" "$(build "$d")"

d=$(new_book dead-link)
file "$d" src/intro.md '# Intro

See [the pattern](pattern.md).'
expect fail "a link to a page that does not exist" "$(build "$d")" "intro.html" "pattern.html"

d=$(new_book dead-anchor)
file "$d" src/intro.md '# Intro

See [the allowance](guide/seams.md#allowance).'
expect fail "a link to an anchor that does not exist" "$(build "$d")" "intro.html" "#allowance"

d=$(new_book dead-anchor-same-page)
file "$d" src/guide/seams.md '# Seams

Jump to [the darts](#darts).'
expect fail "a link to a missing anchor on the same page" "$(build "$d")" "seams.html" "#darts"

expect fail "a missing directory" "$tmp/missing" "$tmp/missing"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
