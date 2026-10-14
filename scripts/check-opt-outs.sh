#!/usr/bin/env bash
# Opt-outs of the clippy.toml bans (the determinism and boundary rules) are
# reviewed: each one in crates/ or bindings/ needs an entry, with its reason, in
# scripts/disallowed-opt-outs.txt, and each entry must still match an opt-out.
#
# An opt-out is any mention of clippy::disallowed_types or disallowed_methods
# (old singular names too), of a group containing them (clippy::style, clippy::all),
# warnings inside an allow or expect, one of those keys in a Cargo.toml lints
# table, or any of them or --cap-lints in .cargo/config rustflags. Rust files are
# scanned whole, so multi-line and crate-level attributes count, and so does a
# mention in a comment.
# Usage: scripts/check-opt-outs.sh [root]   (default: the repository)
# Tests: scripts/tests/check-opt-outs.test.sh
set -euo pipefail
export LC_ALL=C

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"
allowlist=scripts/disallowed-opt-outs.txt

die() {
  echo "opt-outs: $*" >&2
  exit 1
}
[[ -f $allowlist ]] || die "$allowlist is missing"

# find_in <dir> <find predicates...>: files under dir, build output pruned.
find_in() {
  local dir=$1
  shift
  [[ -d $dir ]] || return 0
  find "$dir" \( -name target -o -name node_modules \) -prune -o -type f \( "$@" \) -print
}

# normalize: lint names as written -> canonical names, one per line.
normalize() { tr -d '[:space:]' | sed -E 's/(disallowed_type|disallowed_method)$/\1s/'; }

# rust_lints <file>: one lint name per opt-out in a Rust file.
rust_lints() {
  local flat
  flat=$(tr '\n' ' ' <"$1")
  grep -oE 'clippy[[:space:]]*::[[:space:]]*(disallowed_(types?|methods?)|style|all)\b' <<<"$flat" || true
  # Lint names come before any reason, so the text up to the first ")" holds them all.
  grep -oE '\b(allow|expect)[[:space:]]*\([^)]*' <<<"$flat" |
    sed -E 's/"([^"\\]|\\.)*"//g' | grep -oE '\bwarnings\b' || true
}

# rustflags_lints <.cargo/config>: one name per lint flag that reaches the bans.
rustflags_lints() {
  grep -oE 'clippy[[:space:]]*::[[:space:]]*(disallowed_(types?|methods?)|style|all)\b|warnings\b|cap-lints' "$1" || true
}

# cargo_lints <Cargo.toml>: one lint name per matching key in a lints table.
cargo_lints() {
  awk '
    /^[[:space:]]*\[/ { lints = ($0 ~ /^[[:space:]]*\[(workspace\.)?lints[].]/) }
    lints && /^[[:space:]]*((clippy|rust)\.)?(disallowed_(types?|methods?)|style|all|warnings)[[:space:]]*=/ {
      key = $0
      sub(/^[[:space:]]*((clippy|rust)\.)?/, "", key)
      sub(/[[:space:]]*=.*/, "", key)
      print (key == "warnings" ? key : "clippy::" key)
    }' "$1"
}

found=$(
  while IFS= read -r f; do
    rust_lints "$f" | while IFS= read -r lint; do echo "$f $(normalize <<<"$lint")"; done
  done < <(find_in crates -name '*.rs'; find_in bindings -name '*.rs')
  while IFS= read -r f; do
    cargo_lints "$f" | while IFS= read -r lint; do echo "$f $(normalize <<<"$lint")"; done
  done < <(find . -maxdepth 1 -name Cargo.toml | sed 's|^\./||'; find_in crates -name Cargo.toml; find_in bindings -name Cargo.toml)
  while IFS= read -r f; do
    rustflags_lints "$f" | while IFS= read -r lint; do echo "$f $(normalize <<<"$lint")"; done
  done < <(find_in .cargo -name 'config*' | sed 's|^\./||')
)

violations=()
listed=""
n=0
while IFS= read -r line || [[ -n $line ]]; do
  n=$((n + 1))
  [[ $line =~ ^[[:space:]]*(#|$) ]] && continue
  read -r path lint reason <<<"$line"
  [[ -n ${reason:-} ]] || { violations+=("$allowlist:$n: entry has no reason: $line"); continue; }
  listed+="$path $lint"$'\n'
done <"$allowlist"

while IFS= read -r entry; do
  [[ -n $entry ]] && violations+=("${entry%% *}: ${entry#* } opt-out is not in $allowlist; add it with the reason, or remove it")
done < <(comm -23 <(sort <<<"$found") <(sort <<<"$listed"))
while IFS= read -r entry; do
  [[ -n $entry ]] && violations+=("$allowlist: stale entry, no matching opt-out: $entry")
done < <(comm -13 <(sort <<<"$found") <(sort <<<"$listed"))

if ((${#violations[@]})); then
  printf 'opt-outs: %s\n' "${violations[@]}" >&2
  exit 1
fi
echo "opt-outs: ok ($(grep -c . <<<"$found" || true) listed)"
