# Checks: what each one enforces

This page lists the rules behind the checks in `scripts/check.sh` that are not obvious from their names: the workspace lints, how a lint is opted out of, the check for copied task counts, the generated status counts, the tests that must also run on wasm32, and the dependency licence check. [Set up and run every check](../start/setup.md) says how to run them.

## Entries

In the order `scripts/check.sh` runs them.

### Lint opt-outs (`hygiene`)

An opt-out of the `disallowed_types` or `disallowed_methods` lint needs a line in [`scripts/disallowed-opt-outs.txt`](https://github.com/openplush/openplush/blob/main/scripts/disallowed-opt-outs.txt) with its reason, so a reviewer sees every one in one place. `scripts/check-opt-outs.sh` fails on:

- an opt-out with no line in that file, or a line whose opt-out is gone;
- any `clippy.toml` or `.clippy.toml` besides the three known ones (the root, `crates/` and `crates/openplush-testkit/`), because clippy reads only the nearest one and another would drop every ban;
- `CLIPPY_CONF_DIR` in a `Cargo.toml` or `.cargo/config*`.

It scans the files git tracks or would add, so ignored trees such as build output are skipped, and counts every textual mention of either lint name in a `.rs` file under `crates/` or `bindings/`, a `Cargo.toml` or a `.cargo/config*`, plus `clippy::style`, `clippy::all` and `warnings` lint levels. The script's header lists the exact patterns and what they miss. It is a tripwire for honest mistakes, not a sandbox.

### Copied task counts (`hygiene`)

A task count copied into prose is wrong the next time a task is ticked, and a review of that tick never opens the page. The counts live on the [status page](../start/status.md#by-feature), generated; every other page links there. `scripts/check-docs-facts.sh` reads `README.md`, `CONTRIBUTING.md`, `specs/README.md` and every `.md` and `.inc` file under `docs/`, except the generated `docs/start/status-counts.inc`, and fails, naming each file and line, on:

- a number followed by "task", "tasks", "work item" or "work items". A zero-padded number is a feature, as in "003 tasks U008", and never matches;
- "N of M" on a line that says "task", "ticked" or "work item", or in a table whose header row says so. Elsewhere, as in a count of pattern pieces, it passes.

A number counts only when it stands alone, so dates (YYYY-MM-DD), version numbers and paper sizes such as A4 never match. The fix is a link to the status page instead of the number.

It also warns, without failing, on each line outside the status page that says "not built", "not yet" or "planned", and prints how many there are: what is built is the status page's to say, so each one is a line to check by hand.

### Generated status counts (`status`)

The task counts on the [status page](../start/status.md#by-feature) are generated, because counts copied by hand drift each time a task is ticked. `scripts/gen-status.py` reads every `specs/*/tasks.md` and writes `docs/start/status-counts.inc`, which the page includes; the `status` step renders the counts again and fails, with a diff, when the committed file differs. After ticking a task, run `scripts/gen-status.py` and commit the include with the tick.

A task line starts `- [ ] ` or `- [x] ` and then a task ID. The generator fails, rather than count around it, on a line that starts with a checkbox but is not a task line (indented, `[X]`, a stray space in the box, another bullet or no ID), and on a `tasks.md` with no task lines.

### Workspace lints (`clippy`)

Clippy runs with `-D warnings`, so every warning blocks a merge. The workspace lints (`[workspace.lints]` in `Cargo.toml`, and `clippy.toml`) flag only what this project has decided is a bug:

| Lint | Why |
|---|---|
| `unsafe_code = "forbid"` | The [constitution](https://github.com/openplush/openplush/blob/main/.specify/memory/constitution.md) forbids unsafe code in the engine. An exception needs an ADR. |
| `missing_docs` (`openplush-pipeline` only) | The public engine API documents every item. Elsewhere a doc comment is one line saying what the signature doesn't. |
| `unreachable_pub` | An item is `pub(crate)` unless another crate calls it. |
| `too_many_lines` (60), `too_many_arguments` (5) | Small units. A function that predates the limit carries `#[expect(clippy::too_many_lines, reason = …)]` until it is split. |
| `unwrap_used`, `expect_used`, `panic` | No panics on data that comes from the user: a panic cannot be caught on wasm32 ([Code rules, WASM constraints](code-rules.md#wasm-constraints)). Allowed in `#[test]` functions and `#[cfg(test)]` modules. Code outside those, such as helpers in `tests/*.rs` and all of `openplush-testkit`, returns `Result` and uses `?`. |
| `disallowed_types`: `HashMap`, `HashSet` | Their iteration order is unspecified, which breaks byte-identical output. |
| `disallowed_methods`: `sort_unstable*`, `select_nth_unstable*` | Equal elements end in an unspecified order, which has differed between x86_64 and wasm32 and between toolchains. Sort stably with a total order (`total_cmp`). |
| `todo`, `unimplemented`, `dbg_macro` | Placeholders and debug output don't merge. |

A lint that is wrong for one line is allowed on that item with `#[allow(clippy::…)]` and a comment saying why. For `unwrap` or `expect` the comment is `// INVARIANT: …`, naming the invariant that makes it safe. An opt-out of `disallowed_types` or `disallowed_methods` also needs its line in the opt-out list above. A lint is never turned off workspace-wide to get one pull request green; that needs its own pull request and a reason.

### Tests on wasm32 (`wasm`)

`cargo test --target wasm32-unknown-unknown` runs only tests marked `#[wasm_bindgen_test]`. A plain `#[test]` is skipped there, with "no tests to run!" and exit status 0, and `#[wasm_bindgen_test]` alone compiles to nothing natively. A test that must run on both targets carries both attributes, as `crates/openplush-geom/tests/wasm_determinism.rs` does:

```rust
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn my_test() { /* … */ }
```

Its crate needs `[target.'cfg(target_arch = "wasm32")'.dev-dependencies] wasm-bindgen-test.workspace = true`.

Such a file is named `tests/wasm_<topic>.rs`. The `wasm` step treats every file with that name as a canary and fails if it runs no test on wasm32, which is what happens, silently, when the attribute goes missing.

### Dependency licences (`deny`, `shell`)

Every dependency, dev dependencies included, must carry a licence on the allowlist in `deny.toml`. A licence not on it needs an ADR; there is no per-package exception.

- **Crates**: `deny` runs `cargo deny`.
- **npm packages**: `shell` runs `scripts/check-js-licenses.sh` right after `pnpm install`. It reads the licence each package declares in its own `package.json`, never pnpm's licence column, because pnpm guesses a licence from the LICENSE text when the field is missing. pnpm installs only the platform packages that match your machine, so for every other package in `pnpm-lock.yaml` the check asks the registry (`pnpm view`) for its declared licence, and like `pnpm install` it needs registry access.

How an npm licence expression is judged:

| Expression | Passes when |
|---|---|
| `A OR B` | any alternative is allowed |
| `A AND B` | every part is allowed |
| `X WITH exception` | never on its own; only as an `OR` alternative to an allowed licence |
| `AND` mixed with `OR` | never: stricter than `cargo deny` on purpose, until a real dependency needs it |

IDs are case-sensitive. A missing licence, `SEE LICENSE IN …`, `UNLICENSED` and a `licenses` array of more than one entry all fail.
