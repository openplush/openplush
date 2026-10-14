#!/usr/bin/env bash
# Engine boundary check (T016). Constitution I's enforcement: without it, the
# boundary between engine, bindings, and shell is a suggestion.
#
# Fails when:
#   1. An engine crate (crates/*) reaches wasm-bindgen, web-sys, or js-sys through
#      normal or build dependencies, directly or transitively. Dev-dependencies are
#      allowed: wasm-bindgen-test needs them and they never ship.
#   2. Engine source (crates/*/src, except openplush-testkit) names std::fs,
#      std::thread, std::net, std::process, std::time::Instant, or
#      std::time::SystemTime. AGENTS.md: no filesystem, clock, or threads.
#   3. A dependency edge between workspace crates is not downward in the DAG from
#      specs/001-pattern-pipeline/plan.md, or a crate is missing from that DAG.
#   4. A crate takes openplush-testkit as a normal or build dependency.
#   5. A binding (bindings/*) depends on a workspace crate other than
#      openplush-pipeline and openplush-project.
#
# Usage: scripts/check-boundary.sh [workspace-root]   (default: this repository)
# Tests: scripts/tests/check-boundary.test.sh
#
# Known limits: the source scan is textual. It ignores // comments but not block
# comments or strings, and it misses a `use std::{...}` group split across lines
# or a renamed `std`. It is a tripwire for honest mistakes, not a sandbox.
set -euo pipefail

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
command -v jq >/dev/null || { echo "check-boundary: jq is required" >&2; exit 2; }

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
  [openplush-export]="openplush-pattern"
  [openplush-project]="openplush-mesh openplush-seams openplush-pattern"
  [openplush-pipeline]="openplush-io openplush-decal openplush-layout openplush-export openplush-project"
  [openplush-testkit]="openplush-pipeline"
)
binding_may_use=" openplush-pipeline openplush-project "
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

meta=$(cargo metadata --format-version 1 --manifest-path "$root/Cargo.toml")

# One tab-separated fact per line: member, edge, forbidden.
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
         reach($id)[] | $pk[.].name | select(IN("wasm-bindgen", "web-sys", "js-sys"))
         | "forbidden\t\($n)\t\(.)"
       else empty end)
' <<<"$meta")

violations=()

while IFS=$'\t' read -r kind a b c; do
  case $kind in
    member)
      name=$a area=$b dir=$c
      if [[ $area == crates && ! -v "parents[$name]" ]]; then
        violations+=("$name is not in the DAG table in scripts/check-boundary.sh (plan.md's crate DAG; a new crate needs its place there)")
      fi
      if [[ $area == crates && $name != "$testkit" && -d $dir/src ]]; then
        while IFS= read -r hit; do
          [[ -n $hit ]] && violations+=("${hit#"$root"/} (engine crates never touch the filesystem, threads, clock, network, or processes)")
        done < <(
          # shellcheck disable=SC2016  # $0 and $1 below are awk's, not the shell's
          find "$dir/src" -name '*.rs' -print0 |
            xargs -0 -r awk '{ line = $0; sub(/\/\/.*/, "", line) }
              line ~ /std::(fs|thread|net|process)([^A-Za-z0-9_]|$)/ ||
              line ~ /std::time::(Instant|SystemTime)/ ||
              line ~ /std::\{[^}]*(^|[^A-Za-z0-9_])(fs|thread|net|process)([^A-Za-z0-9_]|$)/ ||
              line ~ /std::time::\{[^}]*(Instant|SystemTime)/ {
                printf "%s:%d: %s\n", FILENAME, FNR, $0 }'
        )
      fi
      ;;
    edge)
      from=$a area=$b to=$c
      if [[ $to == "$testkit" ]]; then
        violations+=("$from -> $to: openplush-testkit is a dev-dependency only")
      elif [[ $area == bindings ]]; then
        [[ $binding_may_use == *" $to "* ]] ||
          violations+=("$from -> $to: bindings depend on openplush-pipeline and openplush-project only")
      elif [[ $area == crates && -v "parents[$from]" ]]; then
        [[ $(below "$from") == *" $to "* ]] ||
          violations+=("$from -> $to: not downward in the crate DAG (plan.md); a sideways or upward edge needs an ADR")
      fi
      ;;
    forbidden)
      violations+=("$a reaches $b through normal or build dependencies (engine crates never touch browser APIs)")
      ;;
  esac
done <<<"$facts"

if ((${#violations[@]})); then
  printf 'boundary: %s\n' "${violations[@]}" >&2
  echo "check-boundary: ${#violations[@]} violation(s)" >&2
  exit 1
fi
echo "check-boundary: ok"
