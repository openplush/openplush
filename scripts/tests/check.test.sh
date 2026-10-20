#!/usr/bin/env bash
# Tests for scripts/check.sh, using --dry-run so nothing is built.
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
  "cargo test --workspace --locked" "scripts/gen-ts-types.sh --check" "scripts/check-boundary.sh" "scripts/check-hygiene.sh" "cargo deny --locked check" \
  "cargo build --workspace --target wasm32-unknown-unknown --locked" \
  "cargo clippy --workspace --all-targets --target wasm32-unknown-unknown --locked -- -D warnings" \
  "cargo test --workspace --target wasm32-unknown-unknown --locked" \
  "pnpm install --frozen-lockfile" "scripts/check-js-licenses.sh" "pnpm typecheck" "pnpm test"
expect_plan "wasm step requires every wasm canary to run at least one test" "wasm" \
  "--test wasm_determinism --target wasm32-unknown-unknown --locked (must run at least 1 test)" \
  "--test wasm_predicates --target wasm32-unknown-unknown --locked (must run at least 1 test)"
# The guard itself, on canned runner output (the plan cases above never run it).
eval "$(grep '^canary_ran()' "$check")"
for out in "test result: ok. 1 passed; 0 failed" "test result: ok. 10 passed; 0 failed"; do
  if canary_ran <<<"$out"; then ok "canary counts as run: $out"; else bad "canary counts as run: $out"; fi
done
for out in "no tests to run!" "test result: ok. 0 passed; 0 failed" "test result: FAILED. 1 passed; 1 failed"; do
  if canary_ran <<<"$out"; then bad "canary does not count as run: $out"; else ok "canary does not count as run: $out"; fi
done
expect_plan "boundary step runs every self-test before the check" "boundary" \
  "scripts/tests/check-boundary.test.sh" "scripts/check-boundary.sh"
expect_plan "hygiene step checks every lint opt-out against the allowlist" "hygiene" \
  "scripts/check-hygiene.sh" "scripts/check-opt-outs.sh"
expect_plan "engine group is boundary, hygiene, fmt, clippy, test, types, workflows, wasm-check" "engine" \
  "scripts/check-boundary.sh" "scripts/check-hygiene.sh" "scripts/check-opt-outs.sh" "cargo fmt --check" "-- -D warnings" "cargo test --workspace --locked" \
  "scripts/gen-ts-types.sh --check" "actionlint" \
  "cargo test --workspace --target wasm32-unknown-unknown --no-run --locked"
# usize is 32 bits on wasm32, so a test can build natively and not there: a shift
# past u32::MAX is a hard error that only codegen reports, which cargo check and
# clippy never reach. wasm-check builds every test binary for wasm32 without
# running one, so it needs no test runner and the engine group can carry it.
expect_plan "wasm-check builds every test binary for wasm32" "wasm-check" \
  "cargo test --workspace --target wasm32-unknown-unknown --no-run --locked"
# The lint must give the same answer on every machine, so the optional
# shellcheck and pyflakes passes (present on CI runners, not everywhere) are off.
expect_plan "workflows step lints the workflows with actionlint alone" "workflows" \
  "actionlint -shellcheck= -pyflakes= .github/workflows"
expect_plan "default runs the workflows lint" "" "actionlint -shellcheck= -pyflakes="
expect_plan "docs step builds the book and checks its navigation" "docs" \
  "mdbook build" "scripts/check-docs-summary.sh"
# The docs step stays out of the default and group runs until CI's cost is decided.
for args in "" all engine; do
  out=$("$check" --dry-run $args 2>&1)
  if [[ $out != *"mdbook build"* && $out != *"scripts/check-docs-summary.sh"* ]]; then
    ok "${args:-default} run leaves out the docs step"
  else
    bad "${args:-default} run leaves out the docs step" "$out"
  fi
done
expect_plan "setup installs the pinned cargo-deny" "--setup" "cargo install cargo-deny --version"
mdbook=$(sed -n 's/^MDBOOK_VERSION=//p' "$check")
lychee=$(sed -n 's/^LYCHEE_VERSION=//p' "$check")
expect_plan "setup installs the pinned mdbook and lychee" "--setup" \
  "cargo install mdbook --version $mdbook --locked" "cargo install lychee --version $lychee --locked"
actionlint=$(sed -n 's/^ACTIONLINT_VERSION=//p' "$check")
expect_plan "setup installs the pinned actionlint release" "--setup" \
  "actionlint/releases/download/v$actionlint/actionlint_${actionlint}_" "tar -xzf"
# The runner must match Cargo.lock exactly; with no wasm-bindgen locked, say so.
locked=$(cd "$here/../.." && cargo metadata --format-version 1 --locked |
  jq -r '[.packages[] | select(.name == "wasm-bindgen") | .version][0] // ""')
if [[ -n $locked ]]; then
  expect_plan "setup installs the runner Cargo.lock pins" "--setup" "cargo install wasm-bindgen-cli --version $locked --locked"
else
  expect_plan "setup skips the runner when nothing pins wasm-bindgen" "--setup" "no wasm-bindgen in Cargo.lock yet"
fi

# The cargo-deny version CI installs must be the one check.sh pins.
deny=$(sed -n 's/^DENY_VERSION=//p' "$check")
if grep -q "tool: cargo-deny@$deny\b" "$here/../../.github/workflows/engine.yml"; then
  ok "engine.yml installs cargo-deny $deny, as check.sh pins"
else
  bad "engine.yml installs cargo-deny $deny, as check.sh pins" "$(grep -n 'cargo-deny@' "$here/../../.github/workflows/engine.yml")"
fi

# CI installs actionlint through the same installer contributors use.
if grep -q 'scripts/check.sh --setup engine' "$here/../../.github/workflows/engine.yml"; then
  ok "engine.yml installs the engine tools with check.sh --setup engine"
else
  bad "engine.yml installs the engine tools with check.sh --setup engine" "$(grep -n 'check.sh' "$here/../../.github/workflows/engine.yml")"
fi
# With steps named, --setup installs only what they need.
expect_plan "setup for the engine group installs actionlint" "--setup engine" "actionlint_${actionlint}_" "actionlint -shellcheck="
out=$("$check" --dry-run --setup engine 2>&1)
if [[ $out != *"cargo install cargo-deny"* && $out != *"wasm-bindgen-cli"* ]]; then
  ok "setup for the engine group installs nothing for deny or wasm"
else
  bad "setup for the engine group installs nothing for deny or wasm" "$out"
fi
if [[ $out != *"cargo install mdbook"* && $out != *"cargo install lychee"* ]]; then
  ok "setup for the engine group installs nothing for docs"
else
  bad "setup for the engine group installs nothing for docs" "$out"
fi
expect_plan "setup for deny installs only cargo-deny" "--setup deny" "cargo install cargo-deny --version"
out=$("$check" --dry-run --setup deny 2>&1)
if [[ $out != *"actionlint_"* ]]; then ok "setup for deny skips actionlint"; else bad "setup for deny skips actionlint" "$out"; fi

out=$("$check" --dry-run engine 2>&1)
if [[ $out != *"cargo deny"* && $out != *"pnpm"* && $out != *"cargo test --workspace --target wasm32-unknown-unknown --locked"* ]]; then
  ok "engine group runs nothing else"
else
  bad "engine group runs nothing else" "$out"
fi

status=0
out=$("$check" --dry-run fmtt 2>&1) || status=$?
if [[ $status -eq 2 && $out == *"unknown step"* && $out == *" docs"* ]]; then
  ok "unknown step fails with exit 2"
else
  bad "unknown step fails with exit 2 (got $status)" "$out"
fi

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
