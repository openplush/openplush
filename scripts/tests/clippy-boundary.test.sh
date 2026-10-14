#!/usr/bin/env bash
# Tests for the compiler-resolved boundary layer: crates/clippy.toml and
# crates/openplush-testkit/clippy.toml.
#
# Each case copies the repository's real clippy configs into a throwaway workspace,
# adds one probe crate, and runs clippy with -D warnings. The point is the forms
# scripts/check-boundary.sh cannot see (aliases, renamed std): clippy resolves paths,
# so `use std::time;` then `time::Instant::now()` is still std::time::Instant::now.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$here/../.."
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export CARGO_TARGET_DIR="$tmp/target" # shared, so each case only recompiles the probe

pass=0
fail=0

# expect <pass|fail> <case name> <crate dir name> <lib.rs source> [<lint text the output must contain>]
expect() {
  local want=$1 name=$2 crate=$3 src=$4 pattern=${5:-}
  local root="$tmp/ws-$((pass + fail))"
  mkdir -p "$root/crates/$crate/src"
  cp "$repo/clippy.toml" "$root/clippy.toml"
  cp "$repo/crates/clippy.toml" "$root/crates/clippy.toml"
  [[ -f $repo/crates/$crate/clippy.toml ]] && cp "$repo/crates/$crate/clippy.toml" "$root/crates/$crate/clippy.toml"
  printf '[workspace]\nresolver = "3"\nmembers = ["crates/%s"]\n' "$crate" >"$root/Cargo.toml"
  printf '[package]\nname = "%s"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n' "$crate" \
    >"$root/crates/$crate/Cargo.toml"
  printf '%s\n' "$src" >"$root/crates/$crate/src/lib.rs"
  local out status=0
  out=$(cd "$root" && cargo clippy --quiet --offline -- -D warnings 2>&1) || status=$?
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

geom=openplush-geom

# Aliased forms the text scan cannot see.
expect fail "use std::time; then time::Instant::now()" $geom \
  'use std::time; pub fn f() -> time::Instant { time::Instant::now() }' "\`std::time::Instant\`"
expect fail "use std as s; then s::fs::read" $geom \
  'use std as s; pub fn f() -> usize { s::fs::read("x").map(|v| v.len()).unwrap_or(0) }' "\`std::fs::read\`"
expect fail "glob import of std::time" $geom \
  'use std::time::*; pub fn f() { let _ = SystemTime::now(); }' "\`std::time::SystemTime\`"
expect fail "fs through a type: File::open" $geom \
  'pub fn f() -> bool { std::fs::File::open("x").is_ok() }' "\`std::fs::File\`"
expect fail "fs: read_to_string" $geom \
  'pub fn f() -> usize { std::fs::read_to_string("x").map(|s| s.len()).unwrap_or(0) }' "\`std::fs::read_to_string\`"
expect fail "threads: thread::spawn" $geom \
  'pub fn f() { let _ = std::thread::spawn(|| ()); }' "\`std::thread::spawn\`"
expect fail "network: TcpStream" $geom \
  'pub fn f() -> bool { std::net::TcpStream::connect("x:1").is_ok() }' "\`std::net::TcpStream\`"
expect fail "processes: Command::new" $geom \
  'pub fn f() { let _ = std::process::Command::new("x"); }' "\`std::process::Command\`"
expect fail "processes: process::exit" $geom \
  'pub fn f() { std::process::exit(1) }' "\`std::process::exit\`"

# Plain-std forms found by review round 2: no std::fs:: path written out, so only
# the compiler layer can see them.
expect fail "fs through Path::exists" $geom \
  'pub fn f() -> bool { std::path::Path::new("x").exists() }' "\`std::path::Path::exists\`"
expect fail "fs through Path::read_dir" $geom \
  'pub fn f() -> bool { std::path::Path::new("/").read_dir().is_ok() }' "\`std::path::Path::read_dir\`"
expect fail "fs through std::fs::exists" $geom \
  'use std as s; pub fn f() -> bool { s::fs::exists("x").unwrap_or(false) }' "\`std::fs::exists\`"
expect fail "clock through UNIX_EPOCH.elapsed()" $geom \
  'use std::time::UNIX_EPOCH; pub fn f() -> bool { UNIX_EPOCH.elapsed().is_ok() }' "\`std::time::SystemTime::elapsed\`"
expect fail "randomness through RandomState" $geom \
  'use std::hash::BuildHasher; pub fn f() -> u64 { std::hash::RandomState::new().hash_one(0u8) }' "\`std::hash::RandomState"
expect fail "network through ToSocketAddrs" $geom \
  'use std as s; use s::net::ToSocketAddrs; pub fn f() -> bool { "example.com:80".to_socket_addrs().is_ok() }' "to_socket_addrs\`"

# The root rules still apply under crates/ (clippy reads only the nearest file).
expect fail "HashMap is still banned in crates/" $geom \
  'pub fn f() -> usize { std::collections::HashMap::<u8, u8>::new().len() }' "\`std::collections::HashMap\`"

# Allowed.
expect pass "Duration is not a clock read" $geom \
  'pub fn f() -> std::time::Duration { std::time::Duration::from_millis(5) }'
expect pass "plain arithmetic" $geom 'pub fn f(a: f64, b: f64) -> f64 { a.mul_add(b, 1.0) }'
expect pass "testkit may read fixture files" openplush-testkit \
  'pub fn load() -> usize { std::fs::read("fixture.glb").map(|v| v.len()).unwrap_or(0) }'
expect fail "testkit still may not use HashMap" openplush-testkit \
  'pub fn f() -> usize { std::collections::HashMap::<u8, u8>::new().len() }' "\`std::collections::HashMap\`"

# Size limits (AGENTS.md, Small units): each clippy.toml sets them, since clippy
# reads only the nearest one. The defaults (100 lines, 7 arguments) would pass
# both fail cases.
long_fn() { # long_fn <body lines>
  printf '#![warn(clippy::too_many_lines)]\npub fn f(x: &mut u32) {\n'
  for ((i = 1; i <= $1; i++)); do printf '    *x = x.wrapping_add(%d);\n' "$i"; done
  printf '}\n'
}
for crate in $geom openplush-testkit; do
  expect pass "$crate: a 50-line function" "$crate" "$(long_fn 50)"
  expect fail "$crate: a 70-line function" "$crate" "$(long_fn 70)" "too_many_lines"
  expect pass "$crate: five parameters" "$crate" 'pub fn f(_: u8, _: u8, _: u8, _: u8, _: u8) {}'
  expect fail "$crate: six parameters" "$crate" 'pub fn f(_: u8, _: u8, _: u8, _: u8, _: u8, _: u8) {}' "too_many_arguments"
done

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
