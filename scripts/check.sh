#!/usr/bin/env bash
# One entry point for every gate. CI calls this too, so a contributor runs
# exactly what CI runs.
#
# Usage:
#   scripts/check.sh --setup          install the pinned tools (cargo-deny, mdbook, lychee, the
#                                     wasm-bindgen-cli matching Cargo.lock, actionlint, the toolchain)
#   scripts/check.sh --setup <steps>  install only what those steps need, then run them
#   scripts/check.sh                  run every gate but docs
#   scripts/check.sh <step|group>...  run some: boundary hygiene fmt clippy test types workflows wasm-check deny wasm shell docs,
#                                     or engine (= boundary hygiene fmt clippy test types workflows wasm-check),
#                                     or all (every step but docs)
#   --dry-run                         print the commands instead of running them
#
# Needs: rustup, Node 22+ with pnpm, bash >= 4.3, jq, python3 >= 3.11.
# Tests: scripts/tests/check.test.sh
set -euo pipefail

# Keep DENY_VERSION and ACTIONLINT_VERSION in step with .github/workflows/engine.yml,
# and MDBOOK_VERSION with .github/workflows/docs.yml (scripts/tests/check.test.sh checks).
DENY_VERSION=0.20.2
MDBOOK_VERSION=0.5.4
LYCHEE_VERSION=0.24.2
ACTIONLINT_VERSION=1.7.9
# sha256 of each release tarball, from the release's checksums file.
ACTIONLINT_SHA256=(
  linux_amd64 233b280d05e100837f4af1433c7b40a5dcb306e3aa68fb4f17f8a7f45a7df7b4
  linux_arm64 6b82a3b8c808bf1bcd39a95aced22fc1a026eef08ede410f81e274af8deadbbc
  darwin_amd64 f89a910e90e536f60df7c504160247db01dd67cab6f08c064c1c397b76c91a79
  darwin_arm64 855e49e823fc68c6371fd6967e359cde11912d8d44fed343283c8e6e943bd789
)

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

dry=0
setup=0
steps=()
for arg in "$@"; do
  case $arg in
    --dry-run) dry=1 ;;
    --setup) setup=1 ;;
    engine) steps+=(boundary hygiene fmt clippy test types workflows wasm-check) ;;
    all) steps+=(boundary hygiene fmt clippy test types workflows wasm-check deny wasm shell) ;;
    boundary | hygiene | fmt | clippy | test | types | workflows | wasm-check | deny | wasm | shell | docs) steps+=("$arg") ;;
    *)
      echo "check: unknown step '$arg' (steps: boundary hygiene fmt clippy test types workflows wasm-check deny wasm shell docs; groups: engine all)" >&2
      exit 2
      ;;
  esac
done
((setup || ${#steps[@]})) || steps=(boundary hygiene fmt clippy test types workflows wasm-check deny wasm shell)

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

# canary_ran: reads a wasm test run's output on stdin; true if it reports at
# least one passed test. "no tests to run!" and "0 passed" are false. Keep it on
# one line: scripts/tests/check.test.sh extracts and tests it.
canary_ran() { grep -Eq 'test result: ok\. [1-9][0-9]* passed'; }

installed_version() { # installed_version <binary> -> second word of `<binary> --version`, or empty
  command -v "$1" >/dev/null && "$1" --version 2>/dev/null | awk '{print $2; exit}' || true
}
actionlint_version() { command -v actionlint >/dev/null && actionlint -version 2>/dev/null | head -n 1 || true; }

# actionlint ships as a Go binary, so --setup installs the pinned release into
# cargo's bin directory (already on PATH here) after checking its checksum.
install_actionlint() {
  local os arch platform sha="" i url tmp bin
  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  case $(uname -m) in
    x86_64 | amd64) arch=amd64 ;;
    aarch64 | arm64) arch=arm64 ;;
    *) die "no pinned actionlint build for $(uname -m)" ;;
  esac
  platform="${os}_${arch}"
  for ((i = 0; i < ${#ACTIONLINT_SHA256[@]}; i += 2)); do
    [[ ${ACTIONLINT_SHA256[i]} == "$platform" ]] && sha=${ACTIONLINT_SHA256[i + 1]}
  done
  [[ -n $sha ]] || die "no pinned actionlint build for $platform"
  url="https://github.com/rhysd/actionlint/releases/download/v$ACTIONLINT_VERSION/actionlint_${ACTIONLINT_VERSION}_$platform.tar.gz"
  bin="${CARGO_HOME:-$HOME/.cargo}/bin"
  tmp=$(mktemp -d)
  run curl -fsSL -o "$tmp/actionlint.tar.gz" "$url"
  if ! ((dry)); then
    if command -v sha256sum >/dev/null; then got=$(sha256sum "$tmp/actionlint.tar.gz"); else got=$(shasum -a 256 "$tmp/actionlint.tar.gz"); fi
    [[ ${got%% *} == "$sha" ]] || die "actionlint download checksum mismatch: got ${got%% *}, pinned $sha"
    mkdir -p "$bin"
  fi
  run tar -xzf "$tmp/actionlint.tar.gz" -C "$bin" actionlint
  rm -rf "$tmp"
}

# needs <step>: whether setup must install that step's tool (every tool when no
# step is named).
needs() { ((${#steps[@]} == 0)) || [[ " ${steps[*]} " == *" $1 "* ]]; }

if ((setup)); then
  group setup
  run rustup toolchain install
  if needs deny && { ((dry)) || [[ $(installed_version cargo-deny) != "$DENY_VERSION" ]]; }; then
    run cargo install cargo-deny --version "$DENY_VERSION" --locked
  fi
  if needs docs; then
    if ((dry)) || [[ $(installed_version mdbook) != "v$MDBOOK_VERSION" ]]; then
      run cargo install mdbook --version "$MDBOOK_VERSION" --locked
    fi
    if ((dry)) || [[ $(installed_version lychee) != "$LYCHEE_VERSION" ]]; then
      run cargo install lychee --version "$LYCHEE_VERSION" --locked
    fi
  fi
  if needs wasm; then
    wb=$(wasm_bindgen_version)
    if [[ -z $wb ]]; then
      echo "check: no wasm-bindgen in Cargo.lock yet; skipping wasm-bindgen-cli"
    elif ((dry)) || [[ $(installed_version wasm-bindgen-test-runner) != "$wb" ]]; then
      run cargo install wasm-bindgen-cli --version "$wb" --locked
    fi
  fi
  if needs workflows && { ((dry)) || [[ $(actionlint_version) != "$ACTIONLINT_VERSION" ]]; }; then
    install_actionlint
  fi
  endgroup
fi

for step in "${steps[@]}"; do
  group "$step"
  case $step in
    boundary)
      # Every self-test first, so a broken checker cannot pass everything.
      for t in scripts/tests/*.test.sh; do
        # Needs mdbook and lychee, which only the docs step requires; it runs there.
        [[ $t == scripts/tests/check-docs-links.test.sh ]] && continue
        run "$t"
      done
      run scripts/check-boundary.sh
      ;;
    hygiene)
      run scripts/check-hygiene.sh
      run scripts/check-opt-outs.sh
      ;;
    fmt) run cargo fmt --check ;;
    clippy) run cargo clippy --workspace --all-targets --locked -- -D warnings ;;
    test) run cargo test --workspace --locked ;;
    types) run scripts/gen-ts-types.sh --check ;;
    workflows)
      # Without shellcheck and pyflakes, which CI runners have and most machines do not.
      if ! ((dry)); then
        have=$(actionlint_version)
        [[ $have == "$ACTIONLINT_VERSION" ]] ||
          die "actionlint ${have:-missing}, check.sh pins $ACTIONLINT_VERSION; run scripts/check.sh --setup"
      fi
      run actionlint -shellcheck= -pyflakes= .github/workflows/*.yml
      ;;
    wasm-check)
      # usize is 32 bits on wasm32, so code and tests that build natively can fail
      # there: a shift past u32::MAX is a hard error only codegen reports, which
      # cargo check and clippy never reach. Builds, runs nothing: no runner needed.
      run cargo test --workspace --target wasm32-unknown-unknown --no-run --locked
      ;;
    deny)
      ((dry)) || command -v cargo-deny >/dev/null || die "cargo-deny not installed; run scripts/check.sh --setup"
      run cargo deny --locked check
      ;;
    wasm)
      # Check the runner first: failing after a full build and clippy wastes minutes.
      if ! ((dry)); then
        want=$(wasm_bindgen_version)
        have=$(installed_version wasm-bindgen-test-runner)
        [[ -z $want || $have == "$want" ]] ||
          die "wasm-bindgen-test-runner ${have:-missing}, Cargo.lock needs $want; run scripts/check.sh --setup"
      fi
      run cargo build --workspace --target wasm32-unknown-unknown --locked
      run cargo clippy --workspace --all-targets --target wasm32-unknown-unknown --locked -- -D warnings
      # Only #[wasm_bindgen_test] tests run here; see CONTRIBUTING.
      run cargo test --workspace --target wasm32-unknown-unknown --locked
      # Each wasm canary (crates/*/tests/wasm_*.rs) must actually run on wasm32.
      # Without its #[wasm_bindgen_test] attribute, wasm32 reports "no tests to
      # run!" and exits 0, so the line above would pass with nothing checked.
      for canary in crates/*/tests/wasm_*.rs; do
        [[ -e $canary ]] || continue
        crate=$(basename "$(dirname "$(dirname "$canary")")")
        name=$(basename "$canary" .rs)
        echo "+ cargo test -p $crate --test $name --target wasm32-unknown-unknown --locked (must run at least 1 test)"
        if ! ((dry)); then
          out=$(cargo test -p "$crate" --test "$name" --target wasm32-unknown-unknown --locked 2>&1) ||
            { echo "$out"; die "wasm canary $name failed"; }
          canary_ran <<<"$out" ||
            { echo "$out"; die "wasm canary $name ran no tests on wasm32 (missing #[wasm_bindgen_test]?)"; }
        fi
      done
      ;;
    shell)
      run pnpm install --frozen-lockfile
      run scripts/check-js-licenses.sh
      run pnpm typecheck
      run pnpm test
      if [[ -f apps/web/package.json ]]; then
        run pnpm --filter web --fail-if-no-match exec playwright install --with-deps chromium
        run pnpm --filter web --fail-if-no-match exec playwright test
      fi
      ;;
    docs)
      if ! ((dry)); then
        have=$(installed_version mdbook)
        [[ $have == "v$MDBOOK_VERSION" ]] ||
          die "mdbook ${have:-missing}, check.sh pins v$MDBOOK_VERSION; run scripts/check.sh --setup docs"
        have=$(installed_version lychee)
        [[ $have == "$LYCHEE_VERSION" ]] ||
          die "lychee ${have:-missing}, check.sh pins $LYCHEE_VERSION; run scripts/check.sh --setup docs"
      fi
      run scripts/tests/check-docs-links.test.sh
      run mdbook build
      # mdBook copies every non-Markdown file under docs/ into the book, so the
      # landing page lands there too, with its dev/ link dead; it belongs at the root.
      run rm -r target/site/dev/site
      run cp docs/site/index.html target/site/index.html
      run scripts/check-docs-summary.sh
      run scripts/check-docs-remote.sh target/site
      run scripts/check-docs-links.sh target/site
      ;;
  esac
  endgroup
done
((dry)) || echo "check: ok (${steps[*]:-setup})"
