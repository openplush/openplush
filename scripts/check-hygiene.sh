#!/usr/bin/env bash
# Code hygiene (AGENTS.md, Engineering rules and Comments and prose):
#   - a module under crates/*/src is at most 400 lines; test modules (tests.rs,
#     proptests.rs) and integration tests are exempt;
#   - code and config carry no task, FR, decision, or spec-section references:
#     those rot when the spec moves, and traceability lives in commits and tasks.md.
# Usage: scripts/check-hygiene.sh [root]   (default: the repository)
# Tests: scripts/tests/check-hygiene.test.sh
set -euo pipefail
export LC_ALL=C

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"

max_lines=400
para=$'\xc2\xa7'
refs="\\b(T[0-9]{3}[a-z]?|FR-[0-9]{3}|D-[0-9]{3})\\b|\\bsection [0-9]|$para"

violations=()

# find_in <dir> <find predicates...>: files under dir, build output pruned.
find_in() {
  local dir=$1
  shift
  [[ -d $dir ]] || return 0
  find "$dir" \( -name target -o -name node_modules \) -prune -o -type f \( "$@" \) -print
}

while IFS= read -r f; do
  n=$(wc -l <"$f")
  ((n <= max_lines)) || violations+=("${f#./}: $n lines (max $max_lines); split by responsibility")
done < <(find_in crates -path '*/src/*' -name '*.rs' ! -name tests.rs ! -name proptests.rs | sort)

while IFS=: read -r f n text; do
  violations+=("${f#./}:$n: spec or task reference; say the why instead: ${text#"${text%%[![:space:]]*}"}")
done < <({
  find_in crates -name '*.rs' -o -name '*.toml'
  find . -maxdepth 1 -type f -name '*.toml'
  find_in scripts -name '*.sh'
  find_in .github/workflows -name '*.yml' -o -name '*.yaml'
} | sort | xargs -r grep -nHE "$refs" || true)

if ((${#violations[@]})); then
  printf 'hygiene: %s\n' "${violations[@]}" >&2
  exit 1
fi
echo "hygiene: ok"
