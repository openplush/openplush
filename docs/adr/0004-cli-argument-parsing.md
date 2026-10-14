---
status: Proposed
date: 2026-10-14
deciders: maintainer (accepts in review of the T038 PR); proposed by the implementing agent
---

# Use `clap` to parse the CLI's arguments

## Context and problem statement

T038 builds `bindings/openplush-cli`, the headless shell that Constitution II makes the reference path ("if it can't run headless, it isn't done") and that golden and determinism tests will run through. It needs subcommands, typed flags, and generated `--help`. This is the project's first command-line parsing dependency, so AGENTS.md asks for an ADR. The plan's tree already names `clap` for this binding.

## Decision drivers

* Constitution II: every capability reachable as file-in, file-out; a usage mistake must fail loudly, never fall back to a default.
* Constitution IX: license on the allowlist; the dependency must pass `cargo deny`.
* Constitution I: the CLI is a shell, so this dependency must not reach `crates/` (the boundary check enforces it).
* Contributor burden: argument parsing is not this project's problem to solve.

## Considered options

* `clap` 4 with the `derive` feature.
* `lexopt` or `pico-args` (minimal, hand-rolled subcommands and help).
* Parsing `std::env::args` by hand.

## Decision outcome

Chosen option: **`clap` 4 with `derive`**, because typed arguments (`ValueEnum`, value parsers) turn a bad flag into a usage error at the edge, and `--help` stays generated from the same definition that parses. MIT OR Apache-2.0 [solid]. It builds for `wasm32-unknown-unknown` with the workspace (checked by `scripts/check.sh wasm`) [solid], though the CLI never runs there.

### Consequences

* Good, because each flag's type, help text and validation live in one definition, and `Command::debug_assert` checks it in a unit test.
* Good, because usage errors have a conventional exit code (2), separate from engine failures (1).
* Bad, because `clap` with `derive` is the heaviest option in compile time and binary size [likely]. That matters little for a native binary that is not shipped to browsers.

### Confirmation

`cargo deny check` passes with `clap` in the lock file, and `scripts/check-boundary.sh` keeps it out of `crates/`. `bindings/openplush-cli/tests/cli.rs` pins the usage-error exit code.

## Pros and cons of the options

### `lexopt` / `pico-args`

* Good, because they are tiny and dependency-free.
* Bad, because subcommands, help text and value validation are hand-written, and that is a second copy of the surface to keep in sync.

### By hand

* Good, because it adds no dependency.
* Bad, because of the same drift as above, with more code to review.

## More information

The surface follows `specs/001-pattern-pipeline/quickstart.md`. As built: [reference/cli.md](../reference/cli.md).
