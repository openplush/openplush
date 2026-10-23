#!/usr/bin/env bash
# Tests for scripts/gen-depgraph.py: each case builds a throwaway git repository and
# asserts the graph the generator embeds in the page it writes.
#
# What matters: edges follow what the code names (Rust mod and use paths, including a
# test crate's own modules and a dependency renamed in its manifest, with string
# literals read as text; TypeScript imports, Python imports a requirements.txt
# declares, script references); a dev-dependency or an inline test module never
# reads as a production edge, so a loop through test code is not reported as a
# cycle; the page loads nothing from the network; the output is byte-identical run
# to run and whatever the checkout directory is called; and a manifest the graph
# cannot be read from fails, naming the file, with nothing written.
set -euo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gen="$here/../gen-depgraph.py"
remote="$here/../check-docs-remote.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

# write <root> <path> <lines>...: write a file of those lines.
write() {
  local file="$1/$2"
  shift 2
  mkdir -p "$(dirname "$file")"
  printf '%s\n' "$@" >"$file"
}

# crate <root> <dir> <name> <dependency lines>...: a Cargo package manifest.
crate() {
  local root=$1 dir=$2 name=$3
  shift 3
  write "$root" "$dir/Cargo.toml" "[package]" "name = \"$name\"" 'version = "0.0.0"' "" "$@"
}

# new_repo <name>: a repository with two crates, a test-only helper crate, a
# TypeScript package, a research script and two shell scripts, committed to the index.
new_repo() {
  local root="$tmp/$1"
  crate "$root" crates/alpha alpha "[dependencies]" 'thiserror = "2"'
  write "$root" crates/alpha/src/lib.rs "mod util;" "mod strings;" "mod raw;" "mod loop_a;" "mod loop_b;" "mod first;" "mod second;" \
    "pub use util::Thing;" "#[derive(thiserror::Error)]" "pub struct E;"
  write "$root" crates/alpha/src/util.rs "pub struct Thing;"
  # Comment markers and braces inside string literals are text, not comments or blocks.
  write "$root" crates/alpha/src/strings.rs 'pub const OPEN: &str = "/* not a comment";' \
    "use crate::util::Thing;" "/* a real comment */" "#[cfg(test)]" "mod tests {" \
    '    const B: &str = "}";' "    use crate::loop_b::b;" "}"
  # A raw string's quotes and braces are text too.
  write "$root" crates/alpha/src/raw.rs "#[cfg(test)]" "mod tests {" \
    '    const J: &str = r#"{"a": "}"}"#;' "    use crate::loop_b::b;" "}" "use crate::util::Thing;"
  # A loop only test code closes: not a cycle.
  write "$root" crates/alpha/src/loop_a.rs "pub fn a() {}" "#[cfg(test)]" "mod tests {" \
    "    use crate::loop_b::b;" "    #[test] fn t() { b() }" "}"
  write "$root" crates/alpha/src/loop_b.rs "use crate::loop_a::a;" "pub fn b() { a() }"
  # A loop production code closes: a cycle.
  write "$root" crates/alpha/src/first.rs "use crate::second::two;" "pub fn one() { two() }"
  write "$root" crates/alpha/src/second.rs "use crate::first::one;" "pub fn two() { one() }"
  crate "$root" crates/beta beta "[dependencies]" 'alpha = { path = "../alpha" }' 'unused = "1"' \
    "" "[dev-dependencies]" 'kit = { path = "../kit" }'
  write "$root" crates/beta/src/lib.rs "use alpha::util::Thing;" "pub fn f(_: Thing) {}"
  write "$root" crates/beta/tests/it.rs "mod helper;" "use kit::assert_ok;" "#[test] fn t() { helper::h() }"
  write "$root" crates/beta/tests/helper/mod.rs "pub fn h() { beta::f(alpha::Thing) }"
  crate "$root" crates/gamma gamma "[dependencies]" 'renamed = { package = "alpha", path = "../alpha" }'
  write "$root" crates/gamma/src/lib.rs "use renamed::util::Thing;" "pub fn g(_: Thing) {}"
  crate "$root" crates/kit kit
  write "$root" crates/kit/src/lib.rs "pub fn assert_ok() {}"
  write "$root" packages/client/package.json '{ "name": "@x/client", "exports": { ".": "./src/index.ts" },' \
    '  "devDependencies": { "vitest": "5.0.3" } }'
  write "$root" packages/client/src/index.ts 'import { x } from "./x";' 'import { it } from "vitest";' "export { x };"
  write "$root" packages/client/src/x.ts "export const x = 1;"
  write "$root" research/probe/requirements.txt "numpy==2.5.3  # the sweep"
  write "$root" research/probe/run.py "import json" "import numpy as np" "from helper import h"
  write "$root" research/probe/helper.py "def h(): pass"
  write "$root" scripts/build.sh '#!/usr/bin/env bash' 'scripts/lint.sh "$@"'
  write "$root" scripts/lint.sh '#!/usr/bin/env bash' 'true'
  git -C "$root" init -q
  git -C "$root" add -A
  echo "$root"
}

# graph <root> <out>: run the generator, then print the embedded graph's facts the
# cases assert, one per line: "edge <source> <target> <kind>", "node <id>", "cycle <ids>".
graph() {
  "$gen" "$1" "$2" >/dev/null
  python3 -I - "$2/index.html" <<'PY'
import json, re, sys
html = open(sys.argv[1], encoding="utf-8").read()
m = re.search(r'<script id="graph-data" type="application/json">(.*?)</script>', html, re.S)
data = json.loads(m.group(1))
for n in data["l3"]["nodes"]:
    print("node", n["id"])
for e in data["l3"]["links"]:
    print("edge", e["source"], e["target"], e["kind"])
for c in data["cycles"]:
    print("cycle", " ".join(c))
PY
}

# has <name> <facts> <line>: the facts include that exact line.
has() { if grep -qxF -- "$3" <<<"$2"; then ok "$1"; else bad "$1: no '$3'" "$2"; fi; }
# lacks <name> <facts> <pattern>: no fact matches the extended regex.
lacks() { if grep -qE -- "$3" <<<"$2"; then bad "$1: found '$3'" "$(grep -E -- "$3" <<<"$2")"; else ok "$1"; fi; }

root=$(new_repo graph)
facts=$(graph "$root" "$tmp/out")
has "a mod declaration is an edge" "$facts" "edge crates/alpha/src/lib.rs crates/alpha/src/util.rs mod"
has "a use path resolves to the module it names in another crate" "$facts" \
  "edge crates/beta/src/lib.rs crates/alpha/src/util.rs use"
has "a test crate's own module is reached" "$facts" "edge crates/beta/tests/it.rs crates/beta/tests/helper/mod.rs mod"
has "a test crate's module resolves its uses" "$facts" "edge crates/beta/tests/helper/mod.rs crates/beta/src/lib.rs use"
has "a dev-dependency is a dev edge" "$facts" "edge crates/beta/tests/it.rs crates/kit/src/lib.rs dev"
has "an inline test module's use is a test edge" "$facts" "edge crates/alpha/src/loop_a.rs crates/alpha/src/loop_b.rs test"
has "an external crate is a node" "$facts" "edge crates/alpha/src/lib.rs ext:crates.io:thiserror use"
has "a declared dependency no source names still shows" "$facts" "edge crates/beta/src/lib.rs ext:crates.io:unused manifest"
has "a renamed dependency resolves by the name the source uses" "$facts" \
  "edge crates/gamma/src/lib.rs crates/alpha/src/util.rs use"
lacks "a renamed dependency the source uses is not reported unnamed" "$facts" "^edge crates/gamma/src/lib.rs .* manifest$"
has "a comment marker inside a string does not hide the code after it" "$facts" \
  "edge crates/alpha/src/strings.rs crates/alpha/src/util.rs use"
has "a brace inside a string does not end an inline test module" "$facts" \
  "edge crates/alpha/src/strings.rs crates/alpha/src/loop_b.rs test"
has "a raw string's braces do not end an inline test module" "$facts" \
  "edge crates/alpha/src/raw.rs crates/alpha/src/loop_b.rs test"
has "code after an inline test module with a raw string reads as production" "$facts" \
  "edge crates/alpha/src/raw.rs crates/alpha/src/util.rs use"
has "a production loop is a cycle" "$facts" "cycle crates/alpha/src/first.rs crates/alpha/src/second.rs"
lacks "a loop closed only by test code is not a cycle" "$facts" "^cycle .*loop_a"
has "a relative TypeScript import resolves to the file" "$facts" "edge packages/client/src/index.ts packages/client/src/x.ts import"
has "a TypeScript package import is an npm node" "$facts" "edge packages/client/src/index.ts ext:npm:vitest import"
has "a local Python import resolves to the file" "$facts" "edge research/probe/run.py research/probe/helper.py import"
has "a declared Python import is a PyPI node" "$facts" "edge research/probe/run.py ext:pypi:numpy import"
lacks "an undeclared Python import (the standard library) is left out" "$facts" "ext:pypi:json"
has "a script naming another script is a reference" "$facts" "edge scripts/build.sh scripts/lint.sh ref"

if [[ -f $tmp/out/LICENSE-d3.txt ]] && grep -q "Mike Bostock" "$tmp/out/LICENSE-d3.txt"; then
  ok "D3's licence is published beside the page"
else
  bad "D3's licence is published beside the page"
fi
if out=$("$remote" "$tmp/out" 2>&1); then ok "the page loads nothing from the network"; else bad "the page loads nothing from the network" "$out"; fi

"$gen" "$root" "$tmp/again" >/dev/null
if cmp -s "$tmp/out/index.html" "$tmp/again/index.html"; then ok "two runs are byte-identical"; else bad "two runs are byte-identical"; fi
cp -r "$root" "$tmp/renamed-checkout"
"$gen" "$tmp/renamed-checkout" "$tmp/renamed-out" >/dev/null
if cmp -s "$tmp/out/index.html" "$tmp/renamed-out/index.html"; then
  ok "the same commit in a differently named directory is byte-identical"
else
  bad "the same commit in a differently named directory is byte-identical"
fi

root=$(new_repo broken)
write "$root" crates/beta/Cargo.toml "[package" "name = beta"
git -C "$root" add -A
status=0
out=$("$gen" "$root" "$tmp/broken-out" 2>&1) || status=$?
if [[ $status -eq 1 && $out == *"crates/beta/Cargo.toml"* && ! -e $tmp/broken-out ]]; then
  ok "an unparseable manifest fails, naming the file, with nothing written"
else
  bad "an unparseable manifest fails, naming the file, with nothing written (exit $status)" "$out"
fi

root=$(new_repo misshapen)
write "$root" packages/client/package.json "[]"
git -C "$root" add -A
status=0
out=$("$gen" "$root" "$tmp/misshapen-out" 2>&1) || status=$?
if [[ $status -eq 1 && $out == "gen-depgraph: packages/client/package.json"* && ! -e $tmp/misshapen-out ]]; then
  ok "a manifest that parses but is not an object fails, naming the file, with nothing written"
else
  bad "a manifest that parses but is not an object fails, naming the file, with nothing written (exit $status)" "$out"
fi

echo "gen-depgraph tests: $pass passed, $fail failed"
((fail == 0))
