#!/usr/bin/env bash
# Tests for scripts/sync-labels.sh (T019), using --dry-run so nothing touches GitHub.
#
# What matters: one command per label in labels.json, every one aimed at an
# explicit repository, and a typo in an argument fails instead of being ignored.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
sync="$here/../sync-labels.sh"
labels="$here/../../.github/labels.json"
pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

out=$("$sync" --dry-run)
want=$(jq length "$labels")
got=$(grep -c '^gh label create ' <<<"$out" || true)
if [[ $got == "$want" ]]; then ok "one command per label ($want)"; else bad "expected $want commands, got $got" "$out"; fi

if grep -v -- '--repo openplush/openplush ' <<<"$out" | grep -q .; then
  bad "every command names the default repo" "$out"
else
  ok "every command names the default repo"
fi

out=$("$sync" --dry-run --repo someone/fork)
if grep -q -- '--repo someone/fork ' <<<"$out"; then ok "--repo overrides the target"; else bad "--repo overrides the target" "$out"; fi

if grep -q -- '--force$' <<<"$out"; then ok "updates existing labels (--force)"; else bad "updates existing labels (--force)" "$out"; fi

if out=$("$sync" --dry-run --bogus 2>&1); then bad "unknown argument fails" "$out"; else ok "unknown argument fails"; fi
if out=$("$sync" --dry-run --repo 2>&1); then bad "--repo without a value fails" "$out"; else ok "--repo without a value fails"; fi

echo "sync-labels tests: $pass passed, $fail failed"
((fail == 0))
