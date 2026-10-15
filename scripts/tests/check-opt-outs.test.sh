#!/usr/bin/env bash
# Tests for scripts/check-opt-outs.sh: each case builds a throwaway tree and asserts
# whether the check accepts or rejects it.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-opt-outs.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

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

lib=crates/openplush-mesh/src/lib.rs

# new_case <name> [<lib.rs source>]: a tree with one crate and an allowlist holding
# only a comment.
new_case() {
  local root="$tmp/$1"
  mkdir -p "$root/crates/openplush-mesh/src" "$root/scripts"
  printf '%s\n' "${2:-pub fn f() {}}" >"$root/$lib"
  printf '# path lint reason\n\n' >"$root/scripts/disallowed-opt-outs.txt"
  echo "$root"
}

# list <root> <entry>: append an allowlist entry.
list() { printf '%s\n' "$2" >>"$1/scripts/disallowed-opt-outs.txt"; }

allow_map='#[allow(clippy::disallowed_types)]
pub fn f() -> usize { std::collections::HashMap::<u8, u8>::new().len() }'

r=$(new_case clean)
expect pass "no opt-outs, empty allowlist" "$r"

r=$(new_case no-allowlist)
rm "$r/scripts/disallowed-opt-outs.txt"
expect fail "a missing allowlist" "$r" "disallowed-opt-outs.txt"

r=$(new_case unlisted "$allow_map")
expect fail "an unlisted opt-out" "$r" "$lib: clippy::disallowed_types"

r=$(new_case listed "$allow_map")
list "$r" "$lib clippy::disallowed_types lookup only; never iterated"
expect pass "a listed opt-out" "$r"

r=$(new_case no-reason "$allow_map")
list "$r" "$lib clippy::disallowed_types"
expect fail "an entry without a reason" "$r" "reason"

r=$(new_case stale)
list "$r" "$lib clippy::disallowed_types lookup only; never iterated"
expect fail "a stale entry" "$r" "stale"

r=$(new_case stale-moved "$allow_map")
list "$r" "crates/openplush-mesh/src/old.rs clippy::disallowed_types lookup only"
expect fail "an entry naming the wrong file" "$r" "crates/openplush-mesh/src/old.rs"

r=$(new_case wrong-lint "$allow_map")
list "$r" "$lib clippy::disallowed_methods lookup only"
expect fail "an entry naming the wrong lint" "$r" "$lib: clippy::disallowed_types"

r=$(new_case two-sites "$allow_map
#[allow(clippy::disallowed_types)]
pub fn g() -> usize { std::collections::HashSet::<u8>::new().len() }")
list "$r" "$lib clippy::disallowed_types lookup only"
expect fail "two opt-outs, one entry" "$r" "$lib: clippy::disallowed_types"
list "$r" "$lib clippy::disallowed_types membership test only"
expect pass "two opt-outs, two entries" "$r"

# Forms that hide from a line-by-line scan, or reach the lint without naming it.
r=$(new_case multi-line '#[expect(
    clippy
        ::disallowed_methods,
    reason = "x"
)]
pub fn f() {}')
expect fail "a multi-line attribute" "$r" "$lib: clippy::disallowed_methods"

r=$(new_case crate-level '#![allow(clippy::disallowed_types)]
pub fn f() {}')
expect fail "a crate-level #![allow]" "$r" "$lib: clippy::disallowed_types"

r=$(new_case cfg-attr '#[cfg_attr(test, allow(clippy::disallowed_methods))]
pub fn f() {}')
expect fail "an opt-out behind cfg_attr" "$r" "$lib: clippy::disallowed_methods"

r=$(new_case old-name '#[allow(clippy::disallowed_method)]
pub fn f() {}')
expect fail "the lint's old singular name" "$r" "$lib: clippy::disallowed_methods"

for group in clippy::style clippy::all; do
  r=$(new_case "group-${group#clippy::}" "#[allow(unused, $group)]
pub fn f() {}")
  expect fail "the $group group, which contains both lints" "$r" "$lib: $group"
done

r=$(new_case warnings '#![allow(
    warnings
)]
pub fn f() {}')
expect fail "allow(warnings)" "$r" "$lib: warnings"

r=$(new_case warnings-elsewhere 'pub fn f(warnings: Option<u8>) -> u8 { warnings.expect("no warnings") }')
expect pass "the word warnings outside a lint attribute" "$r"

r=$(new_case cargo-lints)
printf '[package]\nname = "x"\n\n[lints.clippy]\ndisallowed_methods = "allow"\n' \
  >"$r/crates/openplush-mesh/Cargo.toml"
expect fail "a crate's Cargo.toml [lints] table" "$r" "crates/openplush-mesh/Cargo.toml: clippy::disallowed_methods"

r=$(new_case cargo-workspace-lints)
printf '[workspace.lints.clippy]\nstyle = { level = "allow", priority = -1 }\n' >"$r/Cargo.toml"
expect fail "the workspace [lints] table" "$r" "Cargo.toml: clippy::style"

r=$(new_case cargo-not-lints)
printf '[package]\nname = "x"\n\n[features]\nall = []\nwarnings = []\n' >"$r/crates/openplush-mesh/Cargo.toml"
expect pass "keys named all or warnings outside a lints table" "$r"

r=$(new_case cargo-config)
mkdir -p "$r/.cargo"
printf '[build]\nrustflags = ["-A", "clippy::disallowed_types", "--cap-lints", "warn"]\n' >"$r/.cargo/config.toml"
expect fail "rustflags in .cargo/config.toml" "$r" ".cargo/config.toml: clippy::disallowed_types"
expect fail "--cap-lints in .cargo/config.toml" "$r" ".cargo/config.toml: cap-lints"

r=$(new_case cargo-config-warnings)
mkdir -p "$r/.cargo"
printf '[build]\nrustflags = ["-Awarnings"]\n' >"$r/.cargo/config.toml"
expect fail "-Awarnings in .cargo/config.toml" "$r" ".cargo/config.toml: warnings"

r=$(new_case bindings)
mkdir -p "$r/bindings/openplush-wasm/src"
printf '%s\n' "$allow_map" >"$r/bindings/openplush-wasm/src/lib.rs"
expect fail "an opt-out in bindings/" "$r" "bindings/openplush-wasm/src/lib.rs: clippy::disallowed_types"

r=$(new_case build-output)
mkdir -p "$r/target/debug/build/x/.cargo"
printf '%s\n' "$allow_map" >"$r/target/debug/build/x/gen.rs"
printf 'x = 1\n' >"$r/target/debug/build/x/clippy.toml"
printf '[env]\nCLIPPY_CONF_DIR = "x"\n' >"$r/target/debug/build/x/.cargo/config.toml"
expect pass "the top-level target/ is out of scope" "$r"

r=$(new_case crate-target)
mkdir -p "$r/crates/openplush-mesh/target"
printf '%s\n' "$allow_map" >"$r/crates/openplush-mesh/target/gen.rs"
expect fail "a target/ inside a crate is in scope" "$r" "crates/openplush-mesh/target/gen.rs: clippy::disallowed_types"

# Bypasses that drop every ban at once, so no allowlist entry can excuse them.
r=$(new_case known-configs)
for f in clippy.toml crates/clippy.toml crates/openplush-testkit/clippy.toml; do
  mkdir -p "$r/$(dirname "$f")"
  printf 'too-many-lines-threshold = 60\n' >"$r/$f"
done
expect pass "the three known clippy.toml files" "$r"

for f in crates/openplush-mesh/clippy.toml crates/openplush-mesh/.clippy.toml .clippy.toml bindings/x/clippy.toml; do
  r=$(new_case "stray-${f//\//-}")
  mkdir -p "$r/$(dirname "$f")"
  printf 'too-many-lines-threshold = 60\n' >"$r/$f"
  list "$r" "$f clippy::disallowed_types it is listed"
  expect fail "another clippy config, even listed: $f" "$r" "$f: a clippy config other than"
done

r=$(new_case conf-dir-cargo-config)
mkdir -p "$r/.cargo"
printf '[env]\nCLIPPY_CONF_DIR = { value = "x", relative = true }\n' >"$r/.cargo/config.toml"
expect fail "CLIPPY_CONF_DIR in .cargo/config.toml" "$r" ".cargo/config.toml: CLIPPY_CONF_DIR"

r=$(new_case conf-dir-cargo-toml)
printf '[package]\nname = "x"\n# CLIPPY_CONF_DIR\n' >"$r/crates/openplush-mesh/Cargo.toml"
expect fail "CLIPPY_CONF_DIR in a Cargo.toml" "$r" "crates/openplush-mesh/Cargo.toml: CLIPPY_CONF_DIR"

# Cargo.toml forms a table parser would have to know about.
cargo_case() { # cargo_case <name> <Cargo.toml text> <expected lint>
  local r
  r=$(new_case "$1")
  printf '%s\n' "$2" >"$r/crates/openplush-mesh/Cargo.toml"
  expect fail "Cargo.toml: $1" "$r" "crates/openplush-mesh/Cargo.toml: $3"
}
cargo_case "an inline table" $'[lints]\nclippy = { disallowed_methods = "allow" }' clippy::disallowed_methods
cargo_case "a spaced header" $'[ lints.clippy ]\ndisallowed_types = "allow"' clippy::disallowed_types
cargo_case "a quoted key" $'[lints.clippy]\n"disallowed_methods" = "allow"' clippy::disallowed_methods
cargo_case "a hyphenated key" $'[lints.clippy]\ndisallowed-types = "allow"' clippy::disallowed_types
cargo_case "a quoted group key" $'[lints.clippy]\n"style" = "allow"' clippy::style
cargo_case "[lints.rust] warnings" $'[lints.rust]\nwarnings = "allow"' warnings

r=$(new_case macro 'macro_rules! quiet {
    ($lint:ident) => {
        #[allow(clippy::$lint)]
        pub fn f() -> usize { std::collections::HashMap::<u8, u8>::new().len() }
    };
}
quiet!(disallowed_types);')
expect fail "a macro that builds the attribute from a bare lint name" "$r" "$lib: clippy::disallowed_types"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
