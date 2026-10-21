#!/usr/bin/env bash
# Tests for scripts/gen-indexes.py: each case builds a throwaway docs/ tree and
# asserts what the generator writes.
#
# What matters: a folder's list follows docs/SUMMARY.md, not the directory; each
# entry takes its title from the page's H1 and its description from the index line;
# an ADR row takes its status from the front matter; the output is byte-identical
# run to run; and a page the list cannot be read from fails, naming the file, with
# nothing written.
set -euo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gen="$here/../gen-indexes.py"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

# expect <pass|fail> <case name> <root> [<text the failure output must contain>]
expect() {
  local want=$1 name=$2 root=$3 pattern=${4:-}
  local out status=0
  out=$("$gen" "$root" 2>&1) || status=$?
  if [[ $want == pass && $status -eq 0 ]] ||
    [[ $want == fail && $status -eq 1 && ( -z $pattern || $out == *"$pattern"* ) ]]; then
    ok "$name"
  else
    bad "$name (wanted $want, exit $status)" "$out"
  fi
}

# write <root> <path> <lines>...: write a file of those lines.
write() {
  local file="$1/$2"
  shift 2
  mkdir -p "$(dirname "$file")"
  printf '%s\n' "$@" >"$file"
}

# page <root> <path> <title> <description>: a page with its H1 and index line.
page() {
  write "$1" "$2" "# $3" "" "<!-- index: $4 -->" "" "Opening paragraph." "" "## Section" "" "Body."
}

# adr <root> <file> <status> <title>
adr() {
  write "$1" "docs/adr/$2" "---" "status: $3" "date: 2026-10-21" "deciders: maintainer" "---" "" "# $4" "" \
    "## Context and problem statement" "" "Why."
}

# new_case <name>: a tree whose pages are created in an order that is neither the
# navigation's nor sorted, with every index generated.
new_case() {
  local root="$tmp/$1"
  write "$root" docs/SUMMARY.md "# Summary" "" "[Docs](README.md)" "" \
    "- [ADRs](adr/README.md)" \
    "  - [0000 Template](adr/0000-template.md)" \
    "  - [0001 First](adr/0001-first.md)" \
    "  - [0002 Second](adr/0002-second.md)" \
    "- [Explanation](explanation/README.md)" \
    "  - [Zebra](explanation/zebra.md)" \
    "  - [Moth](explanation/moth.md)" \
    "  - [Ant](explanation/ant.md)" \
    "- [How-to guides](how-to/README.md)" \
    "  - [Sew](how-to/sew.md)" \
    "- [Reference](reference/README.md)" \
    "  - [Terms](reference/terms.md)" \
    "- [Tutorials](tutorials/README.md)" \
    "  - [First](tutorials/first.md)"
  for dir in adr explanation how-to reference tutorials; do write "$root" "docs/$dir/README.md" "# $dir" "" "Prose."; done
  page "$root" docs/explanation/moth.md "Moths" "why moths eat wool."
  page "$root" docs/explanation/ant.md 'Ants: a `colony`' "how ants find the picnic, step by step."
  page "$root" docs/explanation/zebra.md "Zebras" "stripes, and why they never line up."
  page "$root" docs/how-to/sew.md "How to sew" "thread, needle, done."
  page "$root" docs/reference/terms.md "Terms" "every word."
  page "$root" docs/tutorials/first.md "Tutorial: a first plush" "from nothing to a plush."
  write "$root" docs/adr/0000-template.md "---" "# Proposed | Accepted" "status: Proposed" "---" "" "# Short title"
  adr "$root" 0002-second.md "Proposed" "Do the second thing"
  adr "$root" 0001-first.md "Superseded by ADR-0002" "Do the first thing"
  "$gen" "$root" >/dev/null
  echo "$root"
}

# body <file>: the file without its generated-by comment and blank lines.
body() { awk '!/^<!--/ && NF' "$1" 2>/dev/null || true; }

# includes <root>: every generated include, concatenated in path order.
includes() { cat "$1"/docs/*/index.inc 2>/dev/null || true; }

r=$(new_case order)
want="- [Zebras](zebra.md): stripes, and why they never line up.
- [Moths](moth.md): why moths eat wool.
- [Ants: a \`colony\`](ant.md): how ants find the picnic, step by step."
got=$(body "$r/docs/explanation/index.inc")
if [[ $got == "$want" ]]; then ok "entries follow SUMMARY.md's order, with the H1 and the index line"; else bad "entries follow SUMMARY.md's order, with the H1 and the index line" "$got"; fi

ok_all=1
for dir in how-to reference tutorials; do [[ -s $r/docs/$dir/index.inc ]] || ok_all=0; done
if ((ok_all)) && [[ $(body "$r/docs/how-to/index.inc") == "- [How to sew](sew.md): thread, needle, done." ]]; then
  ok "every folder gets its list"
else
  bad "every folder gets its list" "$(ls "$r"/docs/*/)"
fi

want="| [0001](0001-first.md) | Do the first thing | Superseded by ADR-0002 |
| [0002](0002-second.md) | Do the second thing | Proposed |"
got=$(body "$r/docs/adr/index.inc")
if [[ $got == "$want" ]]; then ok "ADR rows in number order, the status from the front matter, no template or README"; else bad "ADR rows in number order, the status from the front matter, no template or README" "$got"; fi

r=$(new_case summary-moves)
sed -i 's#  - \[Ant\](explanation/ant.md)##; s#  - \[Zebra\]#  - [Ant](explanation/ant.md)\n  - [Zebra]#' "$r/docs/SUMMARY.md"
"$gen" "$r" >/dev/null || true
got=$(body "$r/docs/explanation/index.inc" | cut -d']' -f1 | tr '\n' ' ')
if [[ $got == "- [Ants: a \`colony\` - [Zebras - [Moths " ]]; then ok "a move in SUMMARY.md moves the entry"; else bad "a move in SUMMARY.md moves the entry" "$got"; fi

r=$(new_case twice)
includes "$r" >"$tmp/first.inc"
"$gen" "$r" >/dev/null || true
includes "$r" >"$tmp/second.inc"
if [[ -s $tmp/first.inc ]] && cmp -s "$tmp/first.inc" "$tmp/second.inc"; then ok "two runs are byte-identical"; else bad "two runs are byte-identical" "$(diff "$tmp/first.inc" "$tmp/second.inc")"; fi
# ends_in_lf <file>...: every file ends in a newline.
ends_in_lf() {
  local f
  for f in "$@"; do [[ $(tail -c 1 "$f" | od -An -c | tr -d ' ') == '\n' ]] || return 1; done
}
if ! grep -q $'\r' "$tmp/first.inc" && ends_in_lf "$r"/docs/*/index.inc; then
  ok "LF endings and a trailing newline"
else
  bad "LF endings and a trailing newline"
fi

# fails_cleanly <case> <pattern> <command run on the tree>: the run fails naming the
# file, and writes no include, not even for the folders it could read.
fails_cleanly() {
  local name=$1 pattern=$2 r
  r=$(new_case "fail-${name//[^a-zA-Z0-9]/_}")
  rm -f "$r"/docs/*/index.inc
  (cd "$r" && eval "$3")
  expect fail "$name" "$r" "$pattern"
  if [[ -z $(includes "$r") ]]; then ok "a failed run writes nothing: $name"; else bad "a failed run writes nothing: $name" "$(ls "$r"/docs/*/index.inc)"; fi
}

fails_cleanly "a page with no index line" "docs/explanation/moth.md" \
  "sed -i '/<!-- index:/d' docs/explanation/moth.md"
fails_cleanly "an index line that is not just after the H1" "docs/explanation/moth.md" \
  "sed -i '/<!-- index:/d; s/^Body.\$/<!-- index: too late. -->/' docs/explanation/moth.md"
fails_cleanly "an empty index line" "docs/how-to/sew.md" \
  "sed -i 's/<!-- index: .* -->/<!-- index:  -->/' docs/how-to/sew.md"
fails_cleanly "a page with no H1" "docs/reference/terms.md" \
  "sed -i '/^# /d' docs/reference/terms.md"
fails_cleanly "a listed page that does not exist" "docs/tutorials/first.md" \
  "rm docs/tutorials/first.md"
fails_cleanly "an ADR with no status" "docs/adr/0002-second.md" \
  "sed -i '/^status:/d' docs/adr/0002-second.md"
fails_cleanly "an ADR whose status is only in the body" "docs/adr/0001-first.md" \
  "sed -i '/^status:/d' docs/adr/0001-first.md && echo 'Status: Accepted' >>docs/adr/0001-first.md"
fails_cleanly "an ADR with no H1" "docs/adr/0001-first.md" \
  "sed -i '/^# /d' docs/adr/0001-first.md"
fails_cleanly "a folder SUMMARY.md lists no page of" "docs/tutorials/" \
  "sed -i '/tutorials\/first.md/d' docs/SUMMARY.md"

r=$(new_case no-summary)
rm "$r/docs/SUMMARY.md"
expect fail "no SUMMARY.md fails" "$r" "docs/SUMMARY.md"

for args in "--bogus" "one two"; do
  status=0
  # shellcheck disable=SC2086
  out=$("$gen" $args 2>&1) || status=$?
  if [[ $status -eq 2 && $out == *"usage"* ]]; then ok "a usage error exits 2: $args"; else bad "a usage error exits 2: $args (got $status)" "$out"; fi
done

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
