#!/usr/bin/env bash
# Tests for scripts/check.sh (T173), using --dry-run so nothing is built.
#
# What matters is that the one entry point runs every gate CI runs, with the flags
# that make them gates (-D warnings, --locked, --frozen-lockfile), and that a typo
# in a step name fails instead of silently checking nothing.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
check="$here/../check.sh"
pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

# expect_plan <case> <args> <required substring>...
expect_plan() {
  local name=$1 out argv=()
  read -ra argv <<<"$2"
  shift 2
  if ! out=$("$check" --dry-run "${argv[@]}" 2>&1); then bad "$name (exit nonzero)" "$out"; return; fi
  for want in "$@"; do
    [[ $out == *"$want"* ]] || { bad "$name: missing '$want'" "$out"; return; }
  done
  ok "$name"
}

expect_plan "default runs every gate" "" \
  "cargo fmt --check" "cargo clippy --workspace --all-targets --locked -- -D warnings" \
  "cargo test --workspace --locked" "scripts/check-boundary.sh" "cargo deny check" \
  "cargo build --workspace --target wasm32-unknown-unknown --locked" \
  "cargo clippy --workspace --all-targets --target wasm32-unknown-unknown --locked -- -D warnings" \
  "cargo test --workspace --target wasm32-unknown-unknown --locked" \
  "pnpm install --frozen-lockfile" "pnpm typecheck" "pnpm test"
expect_plan "boundary step runs every self-test before the check" "boundary" \
  "scripts/tests/check-boundary.test.sh" "scripts/check-boundary.sh"
expect_plan "engine group is boundary, fmt, clippy, test" "engine" \
  "scripts/check-boundary.sh" "cargo fmt --check" "-- -D warnings" "cargo test --workspace --locked"
expect_plan "setup installs the pinned tools" "--setup" \
  "cargo install cargo-deny --version" "cargo install wasm-bindgen-cli --version"

out=$("$check" --dry-run engine 2>&1)
if [[ $out != *"cargo deny check"* && $out != *"pnpm"* ]]; then
  ok "engine group runs nothing else"
else
  bad "engine group runs nothing else" "$out"
fi

status=0
out=$("$check" --dry-run fmtt 2>&1) || status=$?
if [[ $status -eq 2 && $out == *"unknown step"* ]]; then
  ok "unknown step fails with exit 2"
else
  bad "unknown step fails with exit 2 (got $status)" "$out"
fi

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
