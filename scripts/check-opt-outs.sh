#!/usr/bin/env bash
# Opt-outs of the clippy.toml bans (the determinism and boundary rules) are
# reviewed: each one needs an entry, with its reason, in
# scripts/disallowed-opt-outs.txt, and each entry must still match an opt-out.
#
# Fails when:
#   1. A clippy.toml or .clippy.toml exists other than the three known ones (root,
#      crates/, crates/openplush-testkit/). Clippy reads only the nearest one, so
#      another would drop every ban. No entry excuses it.
#   2. CLIPPY_CONF_DIR appears in any Cargo.toml or .cargo/config*. No entry
#      excuses it.
#   3. An opt-out is not listed, or an entry matches no opt-out. Each of these
#      counts as one opt-out, wherever it appears, comments and strings included:
#      - disallowed_types or disallowed_methods (any spelling: singular,
#        hyphenated, bare) in a .rs under crates/ or bindings/, any Cargo.toml,
#        or any .cargo/config*;
#      - clippy::style or clippy::all (groups containing both lints) in those files;
#      - warnings inside an allow or expect in a .rs file;
#      - a style, all, or warnings key set to a non-list in a Cargo.toml;
#      - warnings or cap-lints in a .cargo/config*.
# In a git checkout the scan covers the files git tracks or would add, so ignored
# trees (build output, a worktree nested in the checkout) are skipped. Outside one
# it covers every file. Either way scripts/tests/ is skipped.
#
# Known limits: the scan is textual, not a parser. It misses a group lint built
# by a macro from a bare `style` or `all`, lint flags passed through RUSTFLAGS
# or the command line, and anything spelled to dodge the patterns above. It is a
# tripwire for honest mistakes, not a sandbox; review of the clippy configs and
# scripts/check.sh is the layer behind it.
# Usage: scripts/check-opt-outs.sh [root]   (default: the repository)
# Tests: scripts/tests/check-opt-outs.test.sh
set -euo pipefail
export LC_ALL=C

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"
allowlist=scripts/disallowed-opt-outs.txt
known_configs=" clippy.toml crates/clippy.toml crates/openplush-testkit/clippy.toml "

[[ -f $allowlist ]] || { echo "opt-outs: $allowlist is missing" >&2; exit 1; }

# repo_files <extended regex>: repository files whose relative path matches, sorted.
repo_files() {
  local files prefix
  # An empty prefix means $PWD is the checkout's top; comparing paths breaks in
  # Git Bash, where git prints C:/... and $PWD is /c/....
  if prefix=$(git rev-parse --show-prefix 2>/dev/null) && [[ -z $prefix ]]; then
    files=$(git ls-files --cached --others --exclude-standard)
  else
    files=$(find . \( -path ./target -o -path ./.git -o -path ./node_modules \) -prune -o -type f -print |
      sed 's|^\./||')
  fi
  { grep -E "$1" <<<"$files" || true; } | { grep -v '^scripts/tests/' || true; } | sort -u
}

lint_re='disallowed[-_](type|method)s?|clippy[[:space:]]*::[[:space:]]*(style|all)\b'

# canon <name as written>: the lint's canonical name.
canon() {
  case $1 in
    disallowed?type*) echo clippy::disallowed_types ;;
    disallowed?method*) echo clippy::disallowed_methods ;;
    style | all) echo "clippy::$1" ;;
    clippy*) tr -d '[:space:]' <<<"$1" ;;
    *) echo "$1" ;;
  esac
}

# rust_lints <file>: one name per opt-out in a Rust file, scanned whole.
rust_lints() {
  local flat
  flat=$(tr '\n' ' ' <"$1")
  grep -oE "$lint_re" <<<"$flat" || true
  # Lint names come before any reason, so the text up to the first ")" holds them all.
  grep -oE '\b(allow|expect)[[:space:]]*\([^)]*' <<<"$flat" |
    sed -E 's/"([^"\\]|\\.)*"//g' | grep -oE '\bwarnings\b' || true
}

# config_lints <file>: one name per opt-out in a Cargo.toml or .cargo/config.
config_lints() {
  grep -oE "$lint_re|cap-lints" "$1" || true
  # A lint level is never a list; a feature (all = [...]) is.
  local keys='style|all'
  if [[ $1 == *.cargo/config* ]]; then grep -oE 'warnings\b' "$1" || true; else keys+='|warnings'; fi
  grep -oE "\\b($keys)[\"']?[[:space:]]*=[[:space:]]*[^[:space:][]" "$1" | grep -oE '^[a-z]+' || true
}

violations=()
while IFS= read -r f; do
  [[ $known_configs == *" $f "* ]] ||
    violations+=("$f: a clippy config other than the three known ones drops every ban; remove it")
done < <(repo_files '(^|/)\.?clippy\.toml$')

configs=$(repo_files '(^|/)Cargo\.toml$|(^|/)\.cargo/config')
while IFS= read -r f; do
  if [[ -n $f ]] && grep -q CLIPPY_CONF_DIR "$f"; then
    violations+=("$f: CLIPPY_CONF_DIR points clippy at another config; remove it")
  fi
done <<<"$configs"

found=$(
  while IFS= read -r f; do
    rust_lints "$f" | while IFS= read -r lint; do echo "$f $(canon "$lint")"; done
  done < <(repo_files '^(crates|bindings)/.*\.rs$')
  while IFS= read -r f; do
    [[ -z $f ]] || config_lints "$f" | while IFS= read -r lint; do echo "$f $(canon "$lint")"; done
  done <<<"$configs"
)

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
