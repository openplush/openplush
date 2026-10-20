#!/usr/bin/env bash
# The built docs site loads nothing from the network: no telemetry, and no third
# party learns who reads which page. A browser fetches a remote resource on page
# load from these references, so the check fails on any that is http(s):// or
# protocol-relative (//host/...):
#   - in HTML: the src of any element, and the href of <link>;
#   - in CSS, and anywhere in HTML (so <style> and style="" too): url() and @import.
# Links (<a href>) load nothing until followed, and data: URIs load nothing, so
# both pass. Scripts can still fetch at run time; book.toml's settings keep
# mdBook's from doing so.
# Usage: scripts/check-docs-remote.sh <dir>   (a built site, e.g. target/site/dev)
# Tests: scripts/tests/check-docs-remote.test.sh
set -euo pipefail
shopt -s inherit_errexit
export LC_ALL=C

dir=${1:?usage: scripts/check-docs-remote.sh <dir>}
[[ -d $dir ]] || { echo "docs-remote: $dir is not a directory" >&2; exit 1; }
cd "$dir"

remote='(https?:)?//'
value="(\"[[:space:]]*$remote[^\"]*\"|'[[:space:]]*$remote[^']*'|$remote[^[:space:]>\"']+)"
attr='([[:space:]][^>]*)?[[:space:]]'
src_ref="<[a-z][a-z0-9-]*${attr}src[[:space:]]*=[[:space:]]*$value"
link_ref="<link${attr}href[[:space:]]*=[[:space:]]*$value"
url_ref="url\\([[:space:]]*[\"']?[[:space:]]*$remote[^)\"'[:space:]]*"
import_ref="@import[[:space:]]*[\"'][[:space:]]*$remote[^\"']*"

# refs <file> <pattern> <sed script>: each match in the file, rewritten as
# "<kind> <reference>". Newlines are flattened so a tag across lines still matches.
refs() {
  tr '\r\n' '  ' <"$1" | { grep -oiE "$2" || true; } | sed -E "$3"
}

# remote_refs <file>: every remote resource reference in the file, one per line.
remote_refs() {
  local f=$1 q="[\"']"
  if [[ $f == *.html ]]; then
    refs "$f" "$src_ref" "s/^<([a-zA-Z0-9-]+).*[[:space:]][sS][rR][cC][[:space:]]*=[[:space:]]*$q?[[:space:]]*/<\\1> src /; s/$q\$//"
    refs "$f" "$link_ref" "s/^<([a-zA-Z]+).*[[:space:]][hH][rR][eE][fF][[:space:]]*=[[:space:]]*$q?[[:space:]]*/<\\1> href /; s/$q\$//"
  fi
  refs "$f" "$url_ref" "s/^[uU][rR][lL]\\([[:space:]]*$q?[[:space:]]*/url() /"
  refs "$f" "$import_ref" "s/^@[iI][mM][pP][oO][rR][tT][[:space:]]*$q[[:space:]]*/@import /"
}

files=()
while IFS= read -r f; do files+=("${f#./}"); done < <(find . -type f \( -name '*.html' -o -name '*.css' \) | sort)
[[ " ${files[*]} " == *".html "* ]] || { echo "docs-remote: no HTML under $dir; build the site first" >&2; exit 1; }

errors=0
for f in "${files[@]}"; do
  found=$(remote_refs "$f")
  [[ -n $found ]] || continue
  while IFS= read -r ref; do
    echo "docs-remote: $f loads $ref" >&2
    errors=$((errors + 1))
  done <<<"$found"
done

((errors == 0)) || { echo "docs-remote: $errors remote resource reference(s); the site must load nothing from the network" >&2; exit 1; }
echo "docs-remote: ok (${#files[@]} files)"
