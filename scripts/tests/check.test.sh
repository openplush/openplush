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
  "cargo test --workspace --locked" "scripts/gen-ts-types.sh --check" "scripts/check-boundary.sh" "scripts/check-hygiene.sh" \
  "cargo deny --locked check" \
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
expect_plan "hygiene step checks lint opt-outs and the ADR lifecycle" "hygiene" \
  "scripts/check-hygiene.sh" "scripts/check-opt-outs.sh" "scripts/check-adr-lifecycle.sh"
expect_plan "engine group is boundary, hygiene, fmt, clippy, test, types, workflows, wasm-check" "engine" \
  "scripts/check-boundary.sh" "scripts/check-hygiene.sh" "scripts/check-opt-outs.sh" "cargo fmt --check" "-- -D warnings" "cargo test --workspace --locked" \
  "scripts/gen-ts-types.sh --check" "actionlint" \
  "cargo test --workspace --target wasm32-unknown-unknown --no-run --locked"
# The status page's task counts are generated when the book is built and never
# committed, so no tick on main can leave a copy stale.
out=$("$check" --dry-run docs 2>&1)
gen=$(grep -nxF "+ scripts/gen-status.py" <<<"$out" | cut -d: -f1 || true)
book=$(grep -nxF "+ mdbook build" <<<"$out" | cut -d: -f1 || true)
if [[ -n $gen && -n $book && $gen -lt $book ]]; then ok "docs step generates the status counts before building the book"; else bad "docs step generates the status counts before building the book" "$out"; fi
status=0
out=$("$check" --dry-run status 2>&1) || status=$?
if [[ $status -eq 2 ]]; then ok "status is not a step: the counts are never committed"; else bad "status is not a step (exit $status)" "$out"; fi
root="$here/../.."
if git -C "$root" check-ignore -q docs/start/status-counts.inc && ! git -C "$root" ls-files --error-unmatch docs/start/status-counts.inc >/dev/null 2>&1; then
  ok "the status counts include is ignored and untracked"
else
  bad "the status counts include is ignored and untracked"
fi
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
# mdBook copies every non-Markdown file under docs/, so the landing page also
# lands inside the book, where its dev/ link is dead; the step drops that copy.
expect_plan "docs step puts the landing page at the site root, not in the book" "docs" \
  "cp docs/site/index.html target/site/index.html" "rm -r target/site/dev/site"
expect_plan "docs step runs the links self-test" "docs" "scripts/tests/check-docs-links.test.sh"
expect_plan "docs step builds the maker book and checks it for IDs and code" "docs" \
  "mdbook build docs/user" "rm target/site/guide/book.toml" "scripts/check-docs-plain.sh"
# The whole site, landing page included, not just the book under dev/.
out=$("$check" --dry-run docs 2>&1)
for cmd in "scripts/check-docs-remote.sh target/site" "scripts/check-docs-links.sh target/site"; do
  if grep -qxF "+ $cmd" <<<"$out"; then ok "docs step runs $cmd"; else bad "docs step runs $cmd" "$out"; fi
done
# The links self-test builds a book with mdbook and runs lychee, which CI's engine
# setup does not install, so only the docs step runs it.
out=$("$check" --dry-run boundary 2>&1)
if [[ $out == *"scripts/tests/check-docs-remote.test.sh"* && $out != *"scripts/tests/check-docs-links.test.sh"* ]]; then
  ok "boundary step leaves the links self-test to the docs step"
else
  bad "boundary step leaves the links self-test to the docs step" "$out"
fi
# The docs step needs mdbook, which CI's engine setup does not install.
for args in "" all engine; do
  out=$("$check" --dry-run $args 2>&1)
  if [[ $out != *"mdbook build"* && $out != *"scripts/check-docs-summary.sh"* && $out != *"scripts/check-docs-remote.sh"* && $out != *"scripts/check-docs-links.sh"* ]]; then
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
# The docs workflow installs the mdbook check.sh pins, and lychee through check.sh.
docs_yml="$here/../../.github/workflows/docs.yml"
if [[ -f $docs_yml ]] && grep -q "tool: mdbook@$mdbook\b" "$docs_yml"; then
  ok "docs.yml installs mdbook $mdbook, as check.sh pins"
else
  bad "docs.yml installs mdbook $mdbook, as check.sh pins" "$(grep -n 'mdbook@' "$docs_yml" 2>&1)"
fi
if [[ -f $docs_yml ]] && grep -q 'scripts/check.sh --setup docs' "$docs_yml"; then
  ok "docs.yml installs the docs tools with check.sh --setup docs"
else
  bad "docs.yml installs the docs tools with check.sh --setup docs" "$(grep -n 'check.sh' "$docs_yml" 2>&1)"
fi
# The determinism job builds, moves the site aside, builds again, then diffs:
# --setup docs installs the tools and runs the docs step, so it is the first build. The first build
# moves outside target/, which rust-cache restores, so a stale copy can never stand in for it.
want_runs=$'scripts/check.sh --setup docs\nmv target/site "$RUNNER_TEMP/site-1"\nscripts/check.sh docs\ndiff -r "$RUNNER_TEMP/site-1" target/site'
got_runs=$([[ -f $docs_yml ]] && awk '/^  determinism:$/ {on = 1; next} on && /^  [a-z-]+:$/ {on = 0} on' "$docs_yml" | sed -n 's/^ *- run: //p' | grep -E '^(scripts/check.sh|mv |diff )')
if [[ $got_runs == "$want_runs" ]]; then
  ok "docs.yml builds the site twice and diffs the builds"
else
  bad "docs.yml builds the site twice and diffs the builds" "$got_runs"
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
