#!/usr/bin/env bash
# Tests for scripts/check-hygiene.sh: each case builds a throwaway tree and asserts
# whether the check accepts or rejects it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-hygiene.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

# Built by concatenation so this file does not trip the check it tests.
task="T""123"
fr="FR-""013"
decision="D-""020"
para=$'\xc2\xa7'"13a"
section="section"" 3"

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

# new_case <name>: a tree with one small clean crate.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root/crates/openplush-geom/src"
  printf '//! Vectors.\npub fn f() {}\n' >"$root/crates/openplush-geom/src/lib.rs"
  echo "$root"
}

# lines <n>: n lines of code.
lines() { for ((i = 0; i < $1; i++)); do echo "const _: u8 = 0;"; done; }

r=$(new_case clean)
expect pass "clean tree" "$r"

r=$(new_case at-cap)
lines 400 >"$r/crates/openplush-geom/src/vector.rs"
expect pass "a 400-line module" "$r"

r=$(new_case over-cap)
lines 401 >"$r/crates/openplush-geom/src/vector.rs"
expect fail "a 401-line module" "$r" "crates/openplush-geom/src/vector.rs"

r=$(new_case nested-over-cap)
mkdir -p "$r/crates/openplush-mesh/src/halfedge"
lines 401 >"$r/crates/openplush-mesh/src/halfedge/build.rs"
expect fail "a 401-line nested module" "$r" "halfedge/build.rs"

r=$(new_case test-modules)
lines 401 >"$r/crates/openplush-geom/src/tests.rs"
lines 401 >"$r/crates/openplush-geom/src/proptests.rs"
mkdir -p "$r/crates/openplush-geom/tests"
lines 401 >"$r/crates/openplush-geom/tests/vector.rs"
expect pass "long test modules and integration tests" "$r"

for ref in "$task" "$fr" "$decision" "$para" "$section"; do
  r=$(new_case "ref-rs-$ref")
  printf '/// Added for %s.\npub fn g() {}\n' "$ref" >>"$r/crates/openplush-geom/src/lib.rs"
  expect fail "reference $ref in a Rust comment" "$r" "crates/openplush-geom/src/lib.rs:3"
done

r=$(new_case ref-toml)
printf '[package]\nname = "x" # %s\n' "$task" >"$r/crates/openplush-geom/Cargo.toml"
expect fail "reference in a crate's Cargo.toml" "$r" "Cargo.toml:2"

r=$(new_case ref-root-toml)
printf '# %s\n' "$decision" >"$r/clippy.toml"
expect fail "reference in a root TOML file" "$r" "clippy.toml:1"

r=$(new_case ref-script)
mkdir -p "$r/scripts/tests"
printf '#!/usr/bin/env bash\n# %s\n' "$task" >"$r/scripts/tests/x.test.sh"
expect fail "reference in a script" "$r" "scripts/tests/x.test.sh:2"

r=$(new_case ref-workflow)
mkdir -p "$r/.github/workflows"
printf 'name: x # %s\n' "$fr" >"$r/.github/workflows/x.yml"
expect fail "reference in a workflow" "$r" ".github/workflows/x.yml:1"

r=$(new_case out-of-scope)
mkdir -p "$r/docs" "$r/specs" "$r/research/s7" "$r/crates/openplush-geom/target/debug"
printf '%s\n' "$task" >"$r/docs/README.md"
printf '%s\n' "$task" >"$r/specs/tasks.md"
printf '# %s\n' "$decision" >"$r/research/s7/run.py"
printf '// %s\n' "$task" >"$r/crates/openplush-geom/target/debug/gen.rs"
expect pass "docs, specs, research, and build output are out of scope" "$r"

r=$(new_case near-misses)
printf '// T12, TODO, T1234x, 0x7f, D-2, FR-1, sections 3, Ti123.\npub fn h() {}\n' \
  >>"$r/crates/openplush-geom/src/lib.rs"
expect pass "near-misses are not references" "$r"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
