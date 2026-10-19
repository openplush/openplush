#!/usr/bin/env bash
# Tests for scripts/pr-tier.py: one case per ADR 0014 rule, each a PR file
# listing on stdin and the tier it must get.
#
# What matters: a C path anywhere wins, a rename counts both names, the tick
# exception holds only for pure ticks, a tier: label is never lowered, and an
# incomplete file listing is C rather than a guess.
set -euo pipefail
# expect runs at the end of a pipeline and counts in this shell.
shopt -s lastpipe

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tier="$here/../pr-tier.py"
pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

# pr <title> <head_ref> <labels json>: an input document, from the files array on stdin.
pr() {
  jq -c --arg title "$1" --arg head "$2" --argjson labels "$3" \
    '{title: $title, head_ref: $head, labels: $labels, changed_files: length, files: .}'
}
# f <path> [status] [previous]: one changed file.
f() { jq -nc --arg p "$1" --arg s "${2:-modified}" --arg prev "${3:-}" '{filename: $p, status: $s} + (if $prev == "" then {} else {previous_filename: $prev} end)'; }
files() { jq -sc . ; }

# expect <case> <jq expression that must be true>, the input document on stdin.
expect() {
  local name=$1 want=$2 out
  if ! out=$(python3 "$tier" 2>&1); then bad "$name (exit nonzero)" "$out"; return; fi
  if [[ $(jq -r "$want" <<<"$out") == true ]]; then ok "$name"; else bad "$name: expected $want" "$out"; fi
}

a_title="feat(web): show the layout"
tick_title="docs(tasks): tick H007"

f crates/openplush-geom/src/lib.rs | files | pr "$a_title" 001/10-x '[]' | expect "a crates/ path is C" '.tier == "C" and .computed == "C"'
f docs/explanation/old.md renamed crates/openplush-geom/src/old.rs | files | pr "$a_title" 001/10-x '[]' | expect "a rename out of crates/ is C (the old name counts)" '.tier == "C"'
f crates/openplush-geom/src/new.rs renamed docs/new.md | files | pr "$a_title" 001/10-x '[]' | expect "a rename into crates/ is C (the new name counts)" '.tier == "C"'
f Cargo.lock | files | pr "$a_title" 001/10-x '[]' | expect "a lockfile is C" '.tier == "C"'
f apps/web/package.json | files | pr "$a_title" 001/10-x '[]' | expect "a nested manifest is C" '.tier == "C"'
f .cargo/config.toml | files | pr "$a_title" 001/10-x '[]' | expect "a .cargo/ config is C" '.tier == "C"'
f .github/workflows/review.yml | files | pr "chore(ci): tweak" chore/review-tweak '[]' | expect "a chore editing review.yml is C, and the review skips it" '.tier == "C" and .review_exempt'
f README.md | files | pr "chore: readme" chore/readme '[]' | expect "a chore editing only the README is A" '.tier == "A" and .review_exempt'
{ f docs/explanation/seams.md; f docs/reference/cli.md added; } | files | pr "docs: explain seams" 001/10-x '[]' | expect "a docs-only PR is A, and the review skips it" '.tier == "A" and .review_exempt'
f docs/adr/0099-x.md added | files | pr "docs(adr): decide" 001/10-x '[]' | expect "docs/adr/ is C, not docs" '.tier == "C"'
f apps/web/src/main.ts | files | pr "$a_title" 001/10-x '[]' | expect "an apps/ source change is A, and the review runs" '.tier == "A" and (.review_exempt | not)'
f fixtures/golden/cube.svg | files | pr "$a_title" 001/10-x '[]' | expect "a golden file is B" '.tier == "B"'
f apps/web/src/__snapshots__/view.snap | files | pr "$a_title" 001/10-x '[]' | expect "a .snap file is B" '.tier == "B"'
f fixtures/expected/cube.pdf | files | pr "$a_title" 001/10-x '[]' | expect "an expected PDF fixture is B" '.tier == "B"'
{ f apps/web/src/main.ts; f fixtures/golden/cube.svg; } | files | pr "$a_title" 001/10-x '[]' | expect "a mixed A+B PR is B" '.tier == "B"'
{ f fixtures/golden/cube.svg; f crates/openplush-geom/src/lib.rs; } | files | pr "$a_title" 001/10-x '[]' | expect "a mixed B+C PR is C" '.tier == "C"'
f apps/AGENTS.md | files | pr "$a_title" 001/10-x '[]' | expect "any AGENTS.md is C" '.tier == "C"'
f clippy.toml | files | pr "chore: relax a lint" chore/lints '[]' | expect "clippy.toml is C (the determinism lints)" '.tier == "C"'
f rustfmt.toml | files | pr "chore: fmt" chore/fmt '[]' | expect "rustfmt.toml is C" '.tier == "C"'
f .gitattributes | files | pr "chore: attributes" chore/attrs '[]' | expect ".gitattributes is C (eol and binary keep fixtures byte-identical)" '.tier == "C"'
f LICENSE-MIT | files | pr "chore: license" chore/license '[]' | expect "a LICENSE-* file is C" '.tier == "C"'
f docs/LICENSE-NOTES.md | files | pr "docs: license notes" 001/10-x '[]' | expect "a LICENSE-* name below the root is not the project's terms" '.tier == "A"'

# Tick PRs: the patch is what GitHub's files API returns (hunks only).
tasks=specs/001-pattern-pipeline/tasks.md
old='- [ ] H007 [P] `crates/openplush-geom/src/lib.rs` — vectors'
tick_patch() { printf '@@ -10,3 +10,3 @@ ## Phase 1\n ctx\n-%s\n+%s\n ctx\n' "$1" "$2"; }
tick_file() { jq -nc --arg p "$tasks" --arg patch "$1" '{filename: $p, status: "modified", patch: $patch}'; }
noted="${old/\[ \]/[x]} — *merged in #241: as \`Vec3\`*"

tick_file "$(tick_patch "$old" "$noted")" | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick-only tasks.md PR is A, and the review skips it" '.tier == "A" and .review_exempt'
tick_file "$(printf '%s-%s\n+%s\n' "$(tick_patch "$old" "$noted")" '- [ ] H008 old scope' '- [ ] H008 new scope')" | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick plus a re-scoped task is C" '.tier == "C"'
tick_file "$(tick_patch "$old" "${old/\[ \]/[x]}")" | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick without an as-built note is C" '.tier == "C"'
tick_file "$(tick_patch "$old" "${noted/vectors/vector}")" | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick that edits the task text is C" '.tier == "C"'
tick_file "$(tick_patch "${old/\[P\]/human:}" "${noted/\[P\]/human:}")" | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick of a human: task is C" '.tier == "C"'
tick_file "$(printf '@@ -10,2 +10,3 @@\n ctx\n+- [ ] H009 new task\n ctx\n')" | files | pr "$tick_title" 001/11-tick '[]' | expect "an added task line in a tick PR is C" '.tier == "C"'
{ tick_file "$(tick_patch "$old" "$noted")"; f docs/reference/cli.md; } | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick PR touching a second file is C" '.tier == "C"'
f "$tasks" | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick PR whose tasks.md patch is missing is C" '.tier == "C"'
tick_file "$(tick_patch "$old" "$noted")" | files | pr "docs(tasks): note" 001/11-tick '[]' | expect "a tick diff under a non-tick title is C" '.tier == "C"'
f docs/reference/cli.md | files | pr "$tick_title" 001/11-tick '[]' | expect "a tick title on a docs-only diff is judged by its paths" '.tier == "A"'

# Labels: never lowered, raised when the diff says so.
f apps/web/src/main.ts | files | pr "$a_title" 001/10-x '["tier:C"]' | expect "an existing tier:C label on an A diff stays C" '.tier == "C" and .computed == "A" and .label == "C" and (.relabel | not)'
f crates/openplush-geom/src/lib.rs | files | pr "$a_title" 001/10-x '["tier:A", "chore"]' | expect "a tier:A label on a C diff is raised" '.tier == "C" and .computed == "C" and .label == "A" and .relabel'
f apps/web/src/main.ts | files | pr "$a_title" 001/10-x '[]' | expect "no tier label on an A diff asks for one" '.label == null and .relabel'

# An incomplete listing is never guessed from.
jq -nc '[range(3001) | {filename: "docs/\(.).md", status: "added"}]' | pr "$a_title" 001/10-x '[]' | expect "more than 3000 files is C" '.tier == "C"'
f docs/x.md | files | pr "$a_title" 001/10-x '[]' | jq -c '.changed_files = 2' | expect "a listing shorter than changed_files is C" '.tier == "C"'
echo '[]' | pr "$a_title" 001/10-x '[]' | expect "an empty diff is A with the reason stated" '.tier == "A" and (.reasons | length) > 0'

if out=$(python3 "$tier" <<<'{"title": "x"}' 2>&1); then bad "a missing field fails" "$out"; else ok "a missing field fails"; fi
if out=$(python3 "$tier" <<<'not json' 2>&1); then bad "invalid JSON fails" "$out"; else ok "invalid JSON fails"; fi

echo "pr-tier tests: $pass passed, $fail failed"
((fail == 0))
