#!/usr/bin/env bash
# Engine boundary check. Constitution I's enforcement: without it, the
# boundary between engine, bindings, and shell is a suggestion.
#
# Fails when:
#   1. An engine crate (crates/*) reaches wasm-bindgen, web-sys, js-sys (browser
#      APIs), rayon (threads; crates/AGENTS.md) or getrandom (OS randomness, which
#      breaks determinism; seeded generators don't need it) through normal or build
#      dependencies, directly or transitively, with every feature enabled.
#      Dev-dependencies are allowed: wasm-bindgen-test needs them and they never ship.
#   2. Engine source (any .rs under crates/<crate>/ outside tests/, benches/,
#      examples/; openplush-testkit excepted) names std::fs, std::thread, std::net,
#      std::process, std::time::Instant, std::time::SystemTime, or std::time::*.
#      AGENTS.md: no filesystem, clock, or threads.
#   3. An engine crate has a build.rs (arbitrary code at build time).
#   4. A dependency edge between workspace crates is not downward in the DAG from
#      specs/001-pattern-pipeline/plan.md, or a crate is missing from that DAG.
#   5. A crate takes openplush-testkit as a normal or build dependency.
#   6. A binding (bindings/*) depends on a workspace crate other than
#      openplush-pipeline (Constitution I; the project API is re-exported).
#   7. A workspace member lives outside crates/ and bindings/, or does not inherit
#      the workspace lints ([lints] workspace = true), which carry forbid(unsafe_code).
#
# Usage: scripts/check-boundary.sh [workspace-root]   (default: this repository)
# Tests: scripts/tests/check-boundary.test.sh
# Needs: bash >= 4.3, jq, python3 >= 3.11, cargo.
#
# Known limits: the source scan is textual. It strips // comments but not block
# comments or strings, and it misses aliasing (`use std::time;` then
# `time::Instant`, `use std as s;`), nested groups (`use std::{io::{self, Read}, fs}`)
# and a `use` split across lines. It is a tripwire for honest mistakes, not a
# sandbox; crates/clippy.toml is the compiler-resolved layer behind it.
# Third-party crates are trusted apart from the dependency denylist above: neither
# layer reads dependency source, so a crate that spawns threads or reads the clock
# internally is caught only by dependency review (and an ADR for a new category).
set -euo pipefail
((BASH_VERSINFO[0] > 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 3))) ||
  { echo "check-boundary: bash >= 4.3 required (macOS: brew install bash)" >&2; exit 2; }

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
for tool in jq python3 cargo; do
  command -v "$tool" >/dev/null || { echo "check-boundary: $tool is required" >&2; exit 2; }
done

# The DAG from plan.md, as each crate's direct parents. A crate may depend on
# anything reachable downward from here. Adding an edge that is not downward
# (sideways between siblings, or upward) needs an ADR, then a change here.
declare -A parents=(
  [openplush-geom]=""
  [openplush-mesh]="openplush-geom"
  [openplush-io]="openplush-mesh"
  [openplush-seams]="openplush-mesh"
  [openplush-fields]="openplush-mesh"
  [openplush-flatten]="openplush-seams openplush-fields"
  [openplush-pattern]="openplush-flatten"
  [openplush-decal]="openplush-pattern"
  [openplush-layout]="openplush-pattern"
  [openplush-export]="openplush-layout"
  [openplush-project]="openplush-mesh openplush-seams openplush-pattern"
  [openplush-pipeline]="openplush-io openplush-decal openplush-layout openplush-export openplush-project"
  [openplush-testkit]="openplush-pipeline"
)
binding_may_use=" openplush-pipeline "  # the project API arrives as openplush_pipeline::project
testkit=openplush-testkit

# below <crate>: every crate reachable downward from <crate>, space-separated.
below() {
  local todo=() next=() seen=" " c
  read -ra todo <<<"${parents[$1]}"
  while ((${#todo[@]})); do
    c=${todo[0]}
    todo=("${todo[@]:1}")
    [[ $seen == *" $c "* ]] && continue
    seen+="$c "
    read -ra next <<<"${parents[$c]:-}"
    todo+=("${next[@]}")
  done
  echo "$seen"
}

# --all-features: a browser crate behind a feature nobody enables yet still ships the
# day someone does. --locked when there is a lockfile, as CI uses everywhere else.
locked=()
[[ -f $root/Cargo.lock ]] && locked=(--locked)
meta=$(cargo metadata --all-features "${locked[@]}" --format-version 1 --manifest-path "$root/Cargo.toml")

# One tab-separated fact per line: member, edge, forbidden, buildscript.
facts=$(jq -r '
  . as $m
  | ($m.packages | map({key: .id, value: .}) | from_entries) as $pk
  | ($m.resolve.nodes | map({key: .id, value: .}) | from_entries) as $nd
  | ($m.workspace_members | map({key: ., value: true}) | from_entries) as $isws
  | def area($id): $pk[$id].manifest_path | ltrimstr($m.workspace_root + "/") | split("/")[0];
    def nondev($id): [($nd[$id].deps // [])[] | select(any(.dep_kinds[]; .kind != "dev")) | .pkg];
    def reach($id):
      {seen: [], todo: nondev($id)}
      | until(.todo == [];
          .todo[0] as $h
          | .todo |= .[1:]
          | if any(.seen[]; . == $h) then . else .seen += [$h] | .todo += nondev($h) end)
      | .seen;
    $m.workspace_members[] as $id
    | $pk[$id].name as $n
    | area($id) as $a
    | "member\t\($n)\t\($a)\t\($pk[$id].manifest_path | rtrimstr("/Cargo.toml"))",
      (nondev($id)[] | select($isws[.]) | "edge\t\($n)\t\($a)\t\($pk[.].name)"),
      (if $a == "crates" then
         (reach($id)[] | $pk[.].name | select(IN("wasm-bindgen", "web-sys", "js-sys", "rayon", "rayon-core", "getrandom"))
          | "forbidden\t\($n)\t\(.)"),
         (if any($pk[$id].targets[]; .kind | index("custom-build")) then "buildscript\t\($n)" else empty end)
       else empty end)
' <<<"$meta")

violations=()

while IFS=$'\t' read -r kind a b c; do
  case $kind in
    member)
      name=$a area=$b dir=$c
      if [[ $area != crates && $area != bindings ]]; then
        violations+=("$name lives in $area/; every workspace member must be under crates/ or bindings/")
      fi
      python3 -c 'import sys, tomllib
d = tomllib.load(open(sys.argv[1], "rb"))
sys.exit(0 if d.get("lints", {}).get("workspace") is True else 1)' "$dir/Cargo.toml" ||
        violations+=("$name must set [lints] workspace = true (the workspace lints carry forbid(unsafe_code))")
      if [[ $area == crates && ! -v "parents[$name]" ]]; then
        violations+=("$name is not in the DAG table in scripts/check-boundary.sh (plan.md's crate DAG; a new crate needs its place there)")
      fi
      if [[ $area == crates && $name != "$testkit" && -d $dir ]]; then
        # Captured, not process-substituted, so a failing awk fails the script.
        # Every .rs outside tests/, benches/, examples/, plus any file a scanned file
        # pulls in with #[path = "..."], wherever it lives (a #[path] into tests/ is
        # still library code).
        files=()
        while IFS= read -r -d '' f; do files+=("$f"); done < <(find "$dir" -name '*.rs' -not -path "$dir/tests/*" \
          -not -path "$dir/benches/*" -not -path "$dir/examples/*" -not -path "$dir/target/*" -print0)
        declare -A seen=()
        for ((i = 0; i < ${#files[@]}; i++)); do
          seen[${files[i]}]=1
        done
        for ((i = 0; i < ${#files[@]}; i++)); do
          while IFS= read -r rel; do
            # Canonical path without GNU realpath -m (macOS has no -m).
            target_dir=$(cd "$(dirname "${files[i]}")/$(dirname "$rel")" 2>/dev/null && pwd -P) || continue
            target="$target_dir/$(basename "$rel")"
            if [[ -f $target && -z ${seen[$target]:-} ]]; then
              seen[$target]=1
              files+=("$target")
            fi
          done < <(grep -oE '#\[path *= *"[^"]+"\]' "${files[i]}" | sed -E 's/.*"([^"]+)".*/\1/' || true)
        done
        unset seen
        ((${#files[@]})) || continue
        # shellcheck disable=SC2016  # $0 below is awk's, not the shell's
        hits=$(awk '{ line = $0; sub(/(^|[[:space:]])\/\/.*/, "", line) }
            line ~ /(^|[^A-Za-z0-9_])std::(fs|thread|net|process)([^A-Za-z0-9_]|$)/ ||
            line ~ /(^|[^A-Za-z0-9_])std::time::(Instant|SystemTime|\*)/ ||
            line ~ /(^|[^A-Za-z0-9_])std::\{([^}]*[^A-Za-z0-9_])?(fs|thread|net|process)([^A-Za-z0-9_]|$)/ ||
            line ~ /(^|[^A-Za-z0-9_])std::time::\{[^}]*(Instant|SystemTime)/ ||
            line ~ /(^|[^A-Za-z0-9_])std::\{.*time::(\{[^}]*)?(Instant|SystemTime|\*)/ {
              printf "%s:%d: %s\n", FILENAME, FNR, $0 }' "${files[@]}")
        while IFS= read -r hit; do
          [[ -n $hit ]] && violations+=("${hit#"$root"/} (engine crates never touch the filesystem, threads, clock, network, or processes)")
        done <<<"$hits"
      fi
      ;;
    edge)
      from=$a area=$b to=$c
      if [[ $to == "$testkit" ]]; then
        violations+=("$from -> $to: openplush-testkit is a dev-dependency only")
      elif [[ $area == bindings ]]; then
        [[ $binding_may_use == *" $to "* ]] ||
          violations+=("$from -> $to: bindings depend on openplush-pipeline only; use its re-exports (Constitution I)")
      elif [[ $area == crates && -v "parents[$from]" ]]; then
        [[ $(below "$from") == *" $to "* ]] ||
          violations+=("$from -> $to: not downward in the crate DAG (plan.md); a sideways or upward edge needs an ADR")
      fi
      ;;
    forbidden)
      violations+=("$a reaches $b through normal or build dependencies (engine crates never touch browser APIs, threads, or OS randomness; crates/AGENTS.md)")
      ;;
    buildscript)
      violations+=("$a has a build.rs: engine crates have no build scripts (arbitrary code at build time)")
      ;;
  esac
done <<<"$facts"

if ((${#violations[@]})); then
  printf 'boundary: %s\n' "${violations[@]}" >&2
  echo "check-boundary: ${#violations[@]} violation(s)" >&2
  exit 1
fi
echo "check-boundary: ok"
