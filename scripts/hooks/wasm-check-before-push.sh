#!/usr/bin/env bash
# Claude Code PreToolUse hook for Bash (.claude/settings.json): before an agent's
# `git push`, run scripts/check.sh wasm-check, so a target that fails only on
# wasm32 shows here and not as red CI. Exit 2 blocks the push; stderr says why.
# Tests: scripts/tests/wasm-check-before-push.test.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
command=$(jq -r '.tool_input.command // ""')
[[ $command =~ (^|[^[:alnum:]_-])git([[:space:]]+-C[[:space:]]+[^[:space:]]+)?[[:space:]]+push([[:space:]]|$) ]] || exit 0
if ! out=$("$root/scripts/check.sh" wasm-check 2>&1); then
  tail -n 40 <<<"$out" >&2
  echo "push blocked: the wasm32 check failed; fix it and rerun scripts/check.sh wasm-check" >&2
  exit 2
fi
