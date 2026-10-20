#!/usr/bin/env bash
# Tests for scripts/hooks/wasm-check-before-push.sh, the agent's PreToolUse hook
# for Bash: only a `git push` runs the wasm32 check, and a failing check blocks
# the push (exit 2) with the reason on stderr.
#
# The hook finds the repository from its own path, so each case runs a copy of it
# in a scratch tree whose scripts/check.sh is a stub that logs its arguments.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
hook="$here/../hooks/wasm-check-before-push.sh"
pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

tree=$(mktemp -d)
trap 'rm -rf "$tree"' EXIT
mkdir -p "$tree/scripts/hooks"
cp "$hook" "$tree/scripts/hooks/"
# The stub exits with $STUB_STATUS and logs each call.
cat >"$tree/scripts/check.sh" <<'STUB'
#!/usr/bin/env bash
echo "$*" >>"$(dirname "$0")/../calls"
echo "stub check output"
exit "${STUB_STATUS:-0}"
STUB
chmod +x "$tree/scripts/check.sh" "$tree/scripts/hooks/wasm-check-before-push.sh"

# expect <case> <command> <want exit> <want calls>: run the hook on a Bash call.
expect() {
  local name=$1 command=$2 want_exit=$3 want_calls=$4 status=0 err calls
  rm -f "$tree/calls"
  err=$(jq -nc --arg c "$command" '{tool_name: "Bash", tool_input: {command: $c}}' |
    "$tree/scripts/hooks/wasm-check-before-push.sh" 2>&1 >/dev/null) || status=$?
  calls=$(cat "$tree/calls" 2>/dev/null || true)
  if [[ $status == "$want_exit" && $calls == "$want_calls" ]]; then
    ok "$name"
  else
    bad "$name (exit $status, calls '$calls')" "$err"
  fi
}

expect "a push runs the wasm32 check" "git push -u origin chore/x" 0 "wasm-check"
expect "a push from another directory runs it" "cd /repo && git -C /repo push origin main" 0 "wasm-check"
expect "a command that is not a push runs nothing" "git status --short" 0 ""
expect "a push named only in text runs nothing" "git log --grep push" 0 ""
STUB_STATUS=1 expect "a failing check blocks the push" "git push origin chore/x" 2 "wasm-check"
err=$(jq -nc '{tool_name: "Bash", tool_input: {command: "git push"}}' |
  STUB_STATUS=1 "$tree/scripts/hooks/wasm-check-before-push.sh" 2>&1 >/dev/null || true)
if [[ $err == *"stub check output"* && $err == *"scripts/check.sh wasm-check"* ]]; then
  ok "a blocked push says why and how to rerun the check"
else
  bad "a blocked push says why and how to rerun the check" "$err"
fi

echo "$pass passed, $fail failed"
((fail == 0))
