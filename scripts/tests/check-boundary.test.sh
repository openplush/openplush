#!/usr/bin/env bash
# Tests for scripts/check-boundary.sh (T016).
#
# Each case builds a throwaway Cargo workspace with path-only dependencies, so no
# network is needed, and asserts whether the boundary check accepts or rejects it.
# Forbidden third-party crates (web-sys, js-sys, wasm-bindgen) are faked as local
# path crates with those names: the check matches by package name.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check-boundary.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

pass=0
fail=0

# crate <root> <dir> <name> [<deps toml>] [<dev-deps toml>] [<src extra>] [<extra toml>]
# Workspace members inherit the workspace lints, as AGENTS.md requires; crates under
# vendor/ stand in for third-party crates and are not members. NO_LINTS=1 omits it.
crate() {
  local root=$1 dir=$2 name=$3 deps=${4:-} devdeps=${5:-} src=${6:-} extra=${7:-}
  local lints=$'[lints]\nworkspace = true'
  [[ $dir == vendor/* || ${NO_LINTS:-} == 1 ]] && lints=""
  mkdir -p "$root/$dir/src"
  cat >"$root/$dir/Cargo.toml" <<TOML
[package]
name = "$name"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
$deps

[dev-dependencies]
$devdeps

$extra

$lints
TOML
  printf '%s\n' "$src" >"$root/$dir/src/lib.rs"
}

# workspace <root> <member>...
workspace() {
  local root=$1
  shift
  local members
  members=$(printf '"%s",' "$@")
  mkdir -p "$root"
  printf '[workspace]\nresolver = "2"\nmembers = [%s]\nexclude = ["vendor"]\n\n[workspace.lints.rust]\nunsafe_code = "forbid"\n' \
    "${members%,}" >"$root/Cargo.toml"
}

# expect <pass|fail> <case name> <root> [<pattern the failure output must contain>]
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

new_case() {
  local root="$tmp/$1"
  mkdir -p "$root"
  echo "$root"
}

# 1. Downward edges along the DAG are allowed.
r=$(new_case clean)
workspace "$r" crates/openplush-geom crates/openplush-mesh crates/openplush-flatten crates/openplush-seams crates/openplush-fields
crate "$r" crates/openplush-geom openplush-geom
crate "$r" crates/openplush-mesh openplush-mesh 'openplush-geom = { path = "../openplush-geom" }'
crate "$r" crates/openplush-seams openplush-seams 'openplush-mesh = { path = "../openplush-mesh" }'
crate "$r" crates/openplush-fields openplush-fields 'openplush-mesh = { path = "../openplush-mesh" }'
crate "$r" crates/openplush-flatten openplush-flatten 'openplush-seams = { path = "../openplush-seams" }
openplush-fields = { path = "../openplush-fields" }
openplush-geom = { path = "../openplush-geom" }'
expect pass "downward DAG edges, including skipping a level" "$r"

# 2. Upward edge.
r=$(new_case upward)
workspace "$r" crates/openplush-geom crates/openplush-mesh
crate "$r" crates/openplush-mesh openplush-mesh
crate "$r" crates/openplush-geom openplush-geom 'openplush-mesh = { path = "../openplush-mesh" }'
expect fail "upward edge geom -> mesh" "$r" "openplush-geom -> openplush-mesh"

# 3. Sideways edge between siblings.
r=$(new_case sideways)
workspace "$r" crates/openplush-layout crates/openplush-export
crate "$r" crates/openplush-layout openplush-layout
crate "$r" crates/openplush-export openplush-export 'openplush-layout = { path = "../openplush-layout" }'
expect fail "sideways edge export -> layout" "$r" "openplush-export -> openplush-layout"

# 4. A crate the DAG table doesn't know.
r=$(new_case unknown)
workspace "$r" crates/openplush-mystery
crate "$r" crates/openplush-mystery openplush-mystery
expect fail "crate missing from the DAG table" "$r" "openplush-mystery"

# 5. Direct browser dependency.
r=$(new_case direct-web)
workspace "$r" crates/openplush-geom
crate "$r" vendor/web-sys web-sys
crate "$r" crates/openplush-geom openplush-geom 'web-sys = { path = "../../vendor/web-sys" }'
expect fail "direct dependency on web-sys" "$r" "web-sys"

# 6. Transitive browser dependency through an innocent-looking crate.
r=$(new_case transitive-web)
workspace "$r" crates/openplush-geom
crate "$r" vendor/js-sys js-sys
crate "$r" vendor/helper helper 'js-sys = { path = "../js-sys" }'
crate "$r" crates/openplush-geom openplush-geom 'helper = { path = "../../vendor/helper" }'
expect fail "transitive dependency on js-sys" "$r" "js-sys"

# 7. wasm-bindgen as a dev-dependency (wasm-bindgen-test needs it) is allowed.
r=$(new_case dev-wasm)
workspace "$r" crates/openplush-geom
crate "$r" vendor/wasm-bindgen wasm-bindgen
crate "$r" crates/openplush-geom openplush-geom '' 'wasm-bindgen = { path = "../../vendor/wasm-bindgen" }'
expect pass "wasm-bindgen as a dev-dependency" "$r"

# 8. Bindings may use only pipeline and project.
r=$(new_case binding-bad)
workspace "$r" crates/openplush-geom crates/openplush-mesh bindings/openplush-cli
crate "$r" crates/openplush-geom openplush-geom
crate "$r" crates/openplush-mesh openplush-mesh 'openplush-geom = { path = "../openplush-geom" }'
crate "$r" bindings/openplush-cli openplush-cli 'openplush-mesh = { path = "../../crates/openplush-mesh" }'
expect fail "binding depends on an inner engine crate" "$r" "openplush-cli -> openplush-mesh"

r=$(new_case binding-ok)
workspace "$r" crates/openplush-pipeline crates/openplush-project bindings/openplush-wasm
crate "$r" crates/openplush-pipeline openplush-pipeline
crate "$r" crates/openplush-project openplush-project
crate "$r" vendor/wasm-bindgen wasm-bindgen
crate "$r" bindings/openplush-wasm openplush-wasm 'openplush-pipeline = { path = "../../crates/openplush-pipeline" }
openplush-project = { path = "../../crates/openplush-project" }
wasm-bindgen = { path = "../../vendor/wasm-bindgen" }'
expect pass "binding uses pipeline, project, and wasm-bindgen" "$r"

# 9. testkit is a dev-dependency only.
r=$(new_case testkit-normal)
workspace "$r" crates/openplush-geom crates/openplush-testkit
crate "$r" crates/openplush-testkit openplush-testkit
crate "$r" crates/openplush-geom openplush-geom 'openplush-testkit = { path = "../openplush-testkit" }'
expect fail "testkit as a normal dependency" "$r" "openplush-testkit"

r=$(new_case testkit-dev)
workspace "$r" crates/openplush-geom crates/openplush-mesh crates/openplush-testkit
crate "$r" crates/openplush-geom openplush-geom
crate "$r" crates/openplush-testkit openplush-testkit 'openplush-geom = { path = "../openplush-geom" }'
crate "$r" crates/openplush-mesh openplush-mesh 'openplush-geom = { path = "../openplush-geom" }' 'openplush-testkit = { path = "../openplush-testkit" }'
expect pass "testkit as a dev-dependency" "$r"

# 10. Filesystem, threads, clock, network, processes in engine source.
for api in 'std::fs::read("x")' 'std::thread::spawn(|| ())' 'std::time::Instant::now()' 'std::time::SystemTime::now()' 'std::net::TcpStream::connect("x")' 'std::process::exit(1)'; do
  r=$(new_case "src-$(tr -c '[:lower:]' '-' <<<"$api" | cut -c1-24)")
  workspace "$r" crates/openplush-geom
  crate "$r" crates/openplush-geom openplush-geom '' '' "pub fn f() { let _ = $api; }"
  expect fail "engine source uses ${api%%(*}" "$r" "${api%%::*}::"
done

r=$(new_case src-use-group)
workspace "$r" crates/openplush-geom
crate "$r" crates/openplush-geom openplush-geom '' '' 'use std::{fmt, fs};'
expect fail "grouped import 'use std::{fmt, fs}'" "$r" "fs"

r=$(new_case src-testkit-fs)
workspace "$r" crates/openplush-testkit
crate "$r" crates/openplush-testkit openplush-testkit '' '' 'pub fn load() { let _ = std::fs::read("fixture.glb"); }'
expect pass "testkit may read fixture files" "$r"

r=$(new_case src-comment)
workspace "$r" crates/openplush-geom
crate "$r" crates/openplush-geom openplush-geom '' '' '//! Never use std::fs here; see crates/AGENTS.md.'
expect pass "a comment mentioning std::fs is not a use" "$r"

# 11. Grouped imports in rustfmt's canonical order: the banned item is often first.
for src in 'use std::{fs, io};' 'use std::{fs::File, path::Path};' 'use std::{net::TcpStream};' \
  'use std::{fmt, time::Instant};' 'use std::time::*;'; do
  r=$(new_case "grp-$(tr -c '[:lower:]' '-' <<<"$src" | cut -c1-28)")
  workspace "$r" crates/openplush-geom
  crate "$r" crates/openplush-geom openplush-geom '' '' "$src"
  expect fail "grouped or glob import: $src" "$r" "std::"
done
for src in 'use std::{fmt, io};' 'use std::{f64::consts::PI, ops::Add};' 'use std::time::Duration;' \
  'pub fn process(v: std::vec::Vec<u8>) -> usize { v.len() }' 'mod nostd { pub mod fs {} } use nostd::fs;'; do
  r=$(new_case "ok-$(tr -c '[:lower:]' '-' <<<"$src" | cut -c1-28)")
  workspace "$r" crates/openplush-geom
  crate "$r" crates/openplush-geom openplush-geom '' '' "$src"
  expect pass "allowed std use: $src" "$r"
done

# 12. A call after a string containing // is code, not a comment.
r=$(new_case src-string-slashes)
workspace "$r" crates/openplush-geom
crate "$r" crates/openplush-geom openplush-geom '' '' 'pub fn f() { let _ = ("//", std::fs::read("x")); }'
expect fail "std::fs after a string containing //" "$r" "std::fs"

# 13. Engine code outside src/ is still engine code.
r=$(new_case src-path-module)
workspace "$r" crates/openplush-geom
crate "$r" crates/openplush-geom openplush-geom '' '' '#[path = "../extra/io.rs"] mod io;'
mkdir -p "$r/crates/openplush-geom/extra"
echo 'pub fn f() { let _ = std::fs::read("x"); }' >"$r/crates/openplush-geom/extra/io.rs"
expect fail "#[path] module outside src/" "$r" "extra/io.rs"

r=$(new_case build-script)
workspace "$r" crates/openplush-geom
crate "$r" crates/openplush-geom openplush-geom
echo 'fn main() {}' >"$r/crates/openplush-geom/build.rs"
expect fail "engine crate with a build.rs" "$r" "build.rs"

# 14. Threads: rayon is banned on the engine path (crates/AGENTS.md), directly or not.
r=$(new_case rayon-direct)
workspace "$r" crates/openplush-geom
crate "$r" vendor/rayon rayon
crate "$r" crates/openplush-geom openplush-geom 'rayon = { path = "../../vendor/rayon" }'
expect fail "direct dependency on rayon" "$r" "rayon"

r=$(new_case rayon-transitive)
workspace "$r" crates/openplush-geom
crate "$r" vendor/rayon rayon
crate "$r" vendor/solver solver 'rayon = { path = "../rayon" }'
crate "$r" crates/openplush-geom openplush-geom 'solver = { path = "../../vendor/solver" }'
expect fail "transitive dependency on rayon" "$r" "rayon"

# 15. A browser crate behind a feature no member enables still ships if anyone enables it.
r=$(new_case optional-web)
workspace "$r" crates/openplush-geom
crate "$r" vendor/web-sys web-sys
crate "$r" crates/openplush-geom openplush-geom 'web-sys = { path = "../../vendor/web-sys", optional = true }' '' '' \
  $'[features]\nweb = ["dep:web-sys"]'
expect fail "optional web-sys behind an unused feature" "$r" "web-sys"

# 16. Every member lives under crates/ or bindings/ and inherits the workspace lints.
r=$(new_case member-elsewhere)
workspace "$r" crates/openplush-geom tools/xtask
crate "$r" crates/openplush-geom openplush-geom
crate "$r" tools/xtask xtask
expect fail "workspace member outside crates/ and bindings/" "$r" "xtask"

r=$(new_case no-workspace-lints)
workspace "$r" crates/openplush-geom
NO_LINTS=1 crate "$r" crates/openplush-geom openplush-geom
expect fail "crate that does not inherit the workspace lints" "$r" "lints"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
