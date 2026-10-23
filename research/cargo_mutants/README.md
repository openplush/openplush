# Spike 001 T204: how much do the flatten and pattern tests catch, by mutation testing?

**Status (2026-10-23): closed. Both crates ran in full; data and a recommendation in [results.md](results.md); adoption is a later decision.**

Non-shipping research. There is no code here: the spike runs an installed tool against two engine crates and records what it reports. Nothing in `crates/`, `apps/`, or `packages/` depends on it, and `scripts/check.sh` never runs it.

## The question

001 T204: if `cargo-mutants` changes the source of `openplush-flatten` and `openplush-pattern` one small edit at a time, how many of those edits do the crates' own tests fail on, which ones survive, and how long does a run take? The answer feeds a later decision on whether to adopt it per PR, nightly, or not at all. Each surviving mutant becomes its own test task; this spike fixes none of them.

## Method

| Step | What | Why |
|---|---|---|
| Tool | `cargo-mutants` 27.1.0, installed with `cargo install --locked` into a scratch root, not through `scripts/check.sh --setup` | The spike measures the tool before the repo pins it |
| Scope | `-p openplush-flatten`, then `-p openplush-pattern`, one after the other on the same machine; every mutant cargo-mutants generates for the package, no exclusions | A full run per crate, so the counts are the crate's, not a sample |
| Tests | cargo-mutants' default: `cargo test` of the mutated package only (its unit, integration and doc tests) | A mutant caught only by another crate's tests counts as missed here |
| Settings | `--timeout-multiplier 3 -j 2`; the timeout is 3 times the unmutated baseline's test time | Two build slots on a 4-core machine; a mutant that hangs (an infinite loop) is a timeout, not a miss |
| Output | `--output` outside the repo; `outcomes.json` summarised into [results.json](results.json) | The repo keeps the summary, not cargo-mutants' 1 GB scratch copies |

A mutant is **caught** when the package's tests fail, **missed** when they pass, **timeout** when they run past the limit, and **unviable** when the mutant does not compile (it says nothing about the tests).

## Versions and machine

cargo-mutants 27.1.0; rustc 1.97.0 (2d8144b78 2026-07-07), the repo's pinned toolchain. Machine: a cloud container, 4 vCPUs (`nproc`), Intel(R) Xeon(R) Processor @ 2.10GHz, 15 GiB RAM, Linux 6.18. Wall times are on that machine and are not comparable to a CI runner without a rerun.

## Run it

From the repo root, at the commit named in results.md:

```sh
cargo install --locked cargo-mutants@27.1.0 --root <tools dir>
export PATH=<tools dir>/bin:$PATH
cargo mutants -p openplush-flatten --timeout-multiplier 3 -j 2 --output <dir outside the repo>
cargo mutants -p openplush-pattern --timeout-multiplier 3 -j 2 --output <another dir>
```

Each run copies the workspace into `$TMPDIR` once per job (about 1.1 GB each with build output, so about 2.2 GB at `-j 2`) and deletes the copies on exit. `--list` prints the mutants without building. To split a run, add `--shard k/n`; the shards together cover every mutant once.

## `results.json`

| Field | Meaning |
|---|---|
| `tool`, `rustc`, `machine` | Versions and machine as above; `machine` has `nproc`, `cpu`, `mem_gib`, `os` |
| `commit` | The `main` commit the runs mutated |
| `recheck` | Where the missed mutants were rechecked by hand on a later commit: `commit`, `crate`, `what` |
| `settings` | The cargo-mutants flags used, as a string |
| `crates[]` | One object per package, in run order |
| `crates[].package` | Cargo package name |
| `crates[].runs[]` | Each cargo-mutants invocation for the package: `started_utc`, `finished_utc` (ISO 8601 UTC, to the second), `wall_seconds` (including the baseline build and test), and a `note` when a run was cut off or resumed with `--iterate` |
| `crates[].fraction_run` | `"1/1"` for a full run; `"k/n"` names the shards that ran |
| `crates[].counts` | `generated`, `caught`, `missed`, `timeout`, `unviable`, each mutant counted once by its last outcome; the last four sum to `generated` |
| `crates[].missed[]` | `file`, `line`, `column`, `function`, `mutation` (cargo-mutants' text), `test_guess` (which test should catch it; a guess, not checked) |
| `crates[].timeout[]` | Same fields without `test_guess` |

Lists are sorted by file, line, column, then mutation text.
