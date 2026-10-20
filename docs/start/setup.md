# How to: set up and run every check

This page gets a fresh machine from nothing to a passing run of every check OpenPlush has, in three commands. Do it before your first change, so you know your machine gives the same answers as CI.

## You will

Have the project cloned, its pinned tools installed, and a run of every gate ending in `check: ok`.

## Before you start

Install these yourself; the setup command installs everything else:

- **git**, **curl** and **tar**.
- **[rustup](https://rustup.rs)**. You don't pick a Rust version: `rust-toolchain.toml` pins it, and that version is also the oldest Rust OpenPlush supports. It comes with the `wasm32-unknown-unknown` target, rustfmt and clippy, and rustup installs exactly that.
- **Node 22 or newer**, and **pnpm 10**. Run `corepack enable` once and pnpm follows the version `package.json` pins.
- **bash 4.3 or newer**. macOS ships bash 3.2, so install a newer one there.
- **jq**, and **python3 3.11 or newer**.

The machine must be Linux or macOS on x86_64 or arm64, the platforms the workflow linter is pinned for. Every step below needs network access: setup downloads and builds tools, and the `shell` check installs npm packages and asks the npm registry for their licences.

## Steps

1. Clone the repository and enter it:

   ```sh
   git clone https://github.com/openplush/openplush && cd openplush
   ```

2. Install the pinned tools:

   ```sh
   scripts/check.sh --setup
   ```

   This installs the Rust toolchain, cargo-deny, mdBook, lychee, the WebAssembly test runner matching the `wasm-bindgen` version in `Cargo.lock`, and actionlint. Most are built from source with `cargo install`, so the first run takes a while; a later run skips every tool already at its pinned version.

3. Run every gate:

   ```sh
   scripts/check.sh all docs
   ```

   `all` is every step CI runs: the crate boundary and the checkers' own tests, hygiene, formatting, clippy, the Rust tests, the generated TypeScript types, the workflow linter, a wasm32 test build, the dependency licence check, the WebAssembly tests, and the TypeScript install, licence check, typecheck and tests. `docs` builds this book and checks its navigation. Plain `scripts/check.sh` runs `all` without `docs`.

   Every CI workflow calls this same script, so a step that passes on your machine passes in CI.

## Check it worked

The run ends with this line, and the command exits with status 0:

```text
check: ok (boundary hygiene fmt clippy test types workflows wasm-check deny wasm shell docs)
```

A missing or wrong-version tool stops the run with a message naming it and telling you to run `scripts/check.sh --setup`. To rerun only what failed, name its step, such as `scripts/check.sh clippy`; `scripts/check.sh --dry-run all` prints the exact commands each step runs.

While you work on the engine, `scripts/check.sh engine` is the fast subset: boundary, hygiene, formatting, clippy, the tests, the generated TypeScript types (it fails if `packages/engine-client/src/generated/` differs from a fresh export), the workflow linter and the wasm32 test build, which compiles every test for wasm32 without running one and so needs no test runner. What each check enforces, and why, is in [the checks reference](../reference/checks.md).
