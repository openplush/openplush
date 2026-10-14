#!/usr/bin/env bash
# One entry point for every gate (T173). CI calls this too, so a contributor runs
# exactly what CI runs.
#
# Usage:
#   scripts/check.sh --setup          install the pinned tools (cargo-deny, the
#                                     wasm-bindgen-cli matching Cargo.lock, the toolchain)
#   scripts/check.sh                  run every gate
#   scripts/check.sh <step|group>...  run some: boundary fmt clippy test deny wasm shell,
#                                     or engine (= boundary fmt clippy test), or all
#   --dry-run                         print the commands instead of running them
#
# Needs: rustup, Node 22+ with pnpm, bash >= 4.3, jq, python3 >= 3.11.
# Tests: scripts/tests/check.test.sh
set -euo pipefail

# Keep in step with .github/workflows/engine.yml (scripts/tests/check.test.sh checks).
DENY_VERSION=0.20.2

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

dry=0
setup=0
steps=()
for arg in "$@"; do
  case $arg in
    --dry-run) dry=1 ;;
    --setup) setup=1 ;;
    engine) steps+=(boundary fmt clippy test) ;;
    all) steps+=(boundary fmt clippy test deny wasm shell) ;;
    boundary | fmt | clippy | test | deny | wasm | shell) steps+=("$arg") ;;
    *)
      echo "check: unknown step '$arg' (steps: boundary fmt clippy test deny wasm shell; groups: engine all)" >&2
      exit 2
      ;;
  esac
done
((setup || ${#steps[@]})) || steps=(boundary fmt clippy test deny wasm shell)

# run <command...>: print it; run it unless --dry-run.
run() {
  echo "+ $*"
  ((dry)) || "$@"
}

group() {
  if [[ ${GITHUB_ACTIONS:-} == true ]]; then echo "::group::$1"; else echo "== $1"; fi
}
endgroup() { [[ ${GITHUB_ACTIONS:-} == true ]] && echo "::endgroup::" || true; }

die() {
  echo "check: $*" >&2
  exit 1
}

# The wasm-bindgen version Cargo.lock pins (empty if none yet); the test runner
# must match it exactly. More than one locked version is an error.
wasm_bindgen_version() {
  cargo metadata --format-version 1 --locked |
    jq -r '[.packages[] | select(.name == "wasm-bindgen") | .version]
      | if length > 1 then error("expected at most one wasm-bindgen in Cargo.lock, got \(length)") else (.[0] // "") end'
}

installed_version() { # installed_version <binary> -> second word of `<binary> --version`, or empty
  command -v "$1" >/dev/null && "$1" --version 2>/dev/null | awk '{print $2; exit}' || true
}

if ((setup)); then
  group setup
  run rustup toolchain install
  if ((dry)) || [[ $(installed_version cargo-deny) != "$DENY_VERSION" ]]; then
    run cargo install cargo-deny --version "$DENY_VERSION" --locked
  fi
  wb=$(wasm_bindgen_version)
  if [[ -z $wb ]]; then
    echo "check: no wasm-bindgen in Cargo.lock yet; skipping wasm-bindgen-cli"
  elif ((dry)) || [[ $(installed_version wasm-bindgen-test-runner) != "$wb" ]]; then
    run cargo install wasm-bindgen-cli --version "$wb" --locked
  fi
  endgroup
fi

for step in "${steps[@]}"; do
  group "$step"
  case $step in
    boundary)
      # Every self-test first, so a broken checker cannot pass everything.
      for t in scripts/tests/*.test.sh; do run "$t"; done
      run scripts/check-boundary.sh
      ;;
    fmt) run cargo fmt --check ;;
    clippy) run cargo clippy --workspace --all-targets --locked -- -D warnings ;;
    test) run cargo test --workspace --locked ;;
    deny)
      ((dry)) || command -v cargo-deny >/dev/null || die "cargo-deny not installed; run scripts/check.sh --setup"
      run cargo deny check
      ;;
    wasm)
      run cargo build --workspace --target wasm32-unknown-unknown --locked
      run cargo clippy --workspace --all-targets --target wasm32-unknown-unknown --locked -- -D warnings
      if ! ((dry)); then
        want=$(wasm_bindgen_version)
        have=$(installed_version wasm-bindgen-test-runner)
        [[ -z $want || $have == "$want" ]] ||
          die "wasm-bindgen-test-runner ${have:-missing}, Cargo.lock needs $want; run scripts/check.sh --setup"
      fi
      # Only #[wasm_bindgen_test] tests run here; see CONTRIBUTING.
      run cargo test --workspace --target wasm32-unknown-unknown --locked
      ;;
    shell)
      run pnpm install --frozen-lockfile
      run pnpm typecheck
      run pnpm test
      if [[ -f apps/web/package.json ]]; then
        run pnpm --filter web --fail-if-no-match exec playwright install --with-deps chromium
        run pnpm --filter web --fail-if-no-match exec playwright test
      fi
      ;;
  esac
  endgroup
done
((dry)) || echo "check: ok (${steps[*]:-setup})"
