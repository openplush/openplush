# Checks: what each one enforces

<!-- index: what the lint, opt-out, wasm32 test and licence checks enforce, and why. -->

This page lists the rules behind the checks in `scripts/check.sh` that are not obvious from their names: the workspace lints, how a lint is opted out of, the tests that must also run on wasm32, the generated status counts, index lists and API pages, and the dependency licence check. It also covers the agent attribution check, which only CI runs, on pull requests. [Set up and run every check](../start/setup.md) says how to run them.

## Entries

In the order `scripts/check.sh` runs them, then the checks it does not run.

### Lint opt-outs (`hygiene`)

An opt-out of the `disallowed_types` or `disallowed_methods` lint needs a line in [`scripts/disallowed-opt-outs.txt`](https://github.com/openplush/openplush/blob/main/scripts/disallowed-opt-outs.txt) with its reason, so a reviewer sees every one in one place. `scripts/check-opt-outs.sh` fails on:

- an opt-out with no line in that file, or a line whose opt-out is gone;
- any `clippy.toml` or `.clippy.toml` besides the three known ones (the root, `crates/` and `crates/openplush-testkit/`), because clippy reads only the nearest one and another would drop every ban;
- `CLIPPY_CONF_DIR` in a `Cargo.toml` or `.cargo/config*`.

It scans the files git tracks or would add, so ignored trees such as build output are skipped, and counts every textual mention of either lint name in a `.rs` file under `crates/` or `bindings/`, a `Cargo.toml` or a `.cargo/config*`, plus `clippy::style`, `clippy::all` and `warnings` lint levels. The script's header lists the exact patterns and what they miss. It is a tripwire for honest mistakes, not a sandbox.

### Copied task counts (`hygiene`)

A task count copied into prose is wrong the next time a task is ticked, and a review of that tick never opens the page. The counts live on the [status page](../start/status.md#by-feature), generated; every other page links there. `scripts/check-docs-facts.sh` reads `README.md`, `CONTRIBUTING.md`, `specs/README.md` and every `.md` and `.inc` file under `docs/`, except the generated `docs/start/status-counts.inc` and the [generated index lists](#generated-index-lists-docs), whose text it already reads in the pages they copy, and fails, naming each file and line, on:

- a number followed by "task", "tasks", "work item" or "work items". A zero-padded number is a feature, as in "003 tasks T008", and never matches;
- "N of M" on a line that says "task", "ticked" or "work item", or in a table whose header row says so. Elsewhere, as in a count of pattern pieces, it passes.

A number counts only when it stands alone, so dates (YYYY-MM-DD), version numbers and paper sizes such as A4 never match. The fix is a link to the status page instead of the number.

It also warns, without failing, on each line outside the status page that says "not built", "not yet" or "planned", and prints how many there are: what is built is the status page's to say, so each one is a line to check by hand.

### ADR lifecycle (`hygiene`)

An ADR that another one amends or supersedes must say so, and the newer one must name it, so a reader who lands on either finds the other. `scripts/check-adr-lifecycle.sh` reads every `docs/adr/NNNN-*.md` and fails when:

- an Accepted ADR amends or supersedes one that is still Proposed;
- an Accepted ADR amends or supersedes one that does not record it, by a "Superseded by ADR-NNNN" status or a line naming the newer ADR with "amended" or "superseded";
- an ADR says another amends or supersedes it, and that other ADR does not name it;
- an ADR has no status, two ADRs share a number, or an amendment names an ADR that does not exist;
- a row of a feature plan's Decision Ledger has an ID that is not `D-` and three digits, the form every feature uses ([ADR 0021](../adr/0021-one-id-scheme.md)).

Each ADR failure names both ADRs and the fix. An ADR's status is the maintainer's decision and its body is a record, so a finding only the maintainer can settle is not fixed by editing the ADR: it gets a line, with its reason, in [`scripts/adr-lifecycle-exceptions.txt`](https://github.com/openplush/openplush/blob/main/scripts/adr-lifecycle-exceptions.txt), and a line whose finding is gone fails as stale. The script's header lists the phrasings it reads as an amendment ("Supersedes, in ADR 0014: …", "amended by", "Changes ADR 0006 …") and what it misses.

The check also prints, without failing, the Proposed rows of every feature plan's Decision Ledger, sorted, as a reminder of the decisions waiting on the maintainer.

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
| `disallowed_methods`: the `f64` and `f32` transcendentals (`sin` to `atanh`, `exp*`, `ln*`, `log*`, `powf`, `powi`, `cbrt`, `hypot`) | `std` hands them to the platform libm, whose last bits differ between native and wasm32. Call the `libm` crate instead. `sqrt` and `+ − × ÷` are correctly rounded and allowed. |
| `disallowed_methods`: `f64::mul_add`, `f32::mul_add` | One rounding style everywhere, so an `a*b + c` oracle stays bit-comparable. Fused multiply-add itself is deterministic; call `libm::fma` where fusing is the point. |
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

### Generated status counts (`docs`)

The task counts on the [status page](../start/status.md#by-feature) are generated, because counts copied by hand drift each time a task is ticked. Before it builds the book, the `docs` step runs `scripts/gen-status.py`, which reads every `specs/*/tasks.md` and writes `docs/start/status-counts.inc` for the page to include: the product features first, then, under their own heading, any whose front matter says `kind: process`. The file is git-ignored and never committed, so a tick needs no regeneration and no pull request conflicts on it. A bare `mdbook build` fails on the missing file; run `scripts/gen-status.py` first, or build through `scripts/check.sh docs`.

A task line starts `- [ ] ` or `- [x] ` and then a task ID: `T` and three digits, in every feature ([ADR 0021](../adr/0021-one-id-scheme.md)). The generator fails the build, rather than count around it, on a task line with another letter, naming it; on a line that starts with a checkbox but is not a task line (indented, `[X]`, a stray space in the box, another bullet or no ID), on a `tasks.md` with no task lines, and on a `kind:` other than `product` (the default) or `process`.

### Generated index lists (`docs`)

The page lists in the `README.md` of `docs/explanation/`, `docs/how-to/`, `docs/reference/` and `docs/tutorials/`, and the table in [`docs/adr/README.md`](../adr/README.md), are generated, because parallel docs pull requests each added a line to the same hand-kept list and conflicted on it. Right after `scripts/gen-status.py`, the `docs` step runs `scripts/gen-indexes.py`, which writes an `index.inc` beside each of those `README.md` files for it to include. Like the status counts, the files are git-ignored and never committed, and a bare `mdbook build` fails without them.

- **A folder's list** has one entry per page `docs/SUMMARY.md` lists in that folder, in `SUMMARY.md`'s order: the page's H1 as the link, a colon, then its index line, `<!-- index: <one line> -->`, which must be the first line after the H1. `SUMMARY.md` stays hand-kept, since its order is the reading order.
- **The ADR table** has one row per `docs/adr/NNNN-*.md` but the template, in number order: the number, the H1, and the `status:` from the front matter. The `README.md` keeps the table's header row.

The generator fails the build, naming the file and writing no include, on a listed page with no H1 or no index line, an ADR with no H1 or no `status:` in its front matter, and a folder `SUMMARY.md` lists no page in.

### Generated API pages (`docs`)

The site publishes the workspace rustdoc under `dev/api/`, beside the contributor book, as the [Rust API](api.md). Right after `mdbook build`, which empties `target/site/dev/`, the `docs` step runs `scripts/assemble-docs-api.sh`, which:

- cleans cargo's doc directory, so a crate gone from the workspace leaves no pages;
- documents each workspace crate with its own `cargo doc --no-deps --locked -p <crate>`, after the workspace crates it depends on, then in name order. rustdoc links an item of another workspace crate only if that crate's docs exist when it starts, so one parallel `cargo doc --workspace` linked differently from run to run, and two builds of the site must be identical;
- copies the doc directory to `target/site/dev/api/` without rustdoc's `.lock`, and writes `api/index.html`, which stable rustdoc does not: every workspace crate in name order, with no date.

It fails, leaving no `api/`, on a workspace crate rustdoc wrote no `index.html` for, on any dotfile besides `.lock`, which would otherwise be published unseen, and on a crate with more than one documented target, since the index links one page per crate.

rustdoc links a source range as `lib.rs.html#11-16`, an anchor only its script reads. The link check reads such a range on a `.rs.html` page as its first line, `#11`, which rustdoc gives an id; a range-shaped anchor on any other page still fails.

### Generated dependency graph (`docs`)

The site publishes an interactive [dependency graph](dependency-graph.md) under `dev/depgraph/`. After the API pages, the `docs` step runs `scripts/gen-depgraph.py`, which reads the files git tracks and writes one page, `depgraph/index.html`, with the graph's data and D3 inlined, and D3's ISC licence beside it as `LICENSE-d3.txt`. D3 7.9.0 is vendored as `scripts/depgraph/d3.min.js`, copied unchanged from the npm package `d3@7.9.0` (the package tarball's sha1, as npm publishes it, is `579e7acb3d749caf8860bd1741ae8d371070cd5d`; the file's own sha256 is `f2094bbf6141b359722c4fe454eb6c4b0f0e42cc10cc7af921fc158fceb86539`), so the page loads nothing from the network and the remote-reference check passes. Like the status counts, the page is never committed, so it cannot go stale.

The output is byte-identical for the same commit: edges are sorted, Python packages count as external only when a `requirements.txt` declares them (never by the running Python's own module list), and neither a date nor the checkout directory's name is written: the page's title is the last path segment of the root `package.json`'s name (so `@scope/name` gives `name`), else the last part of the Cargo workspace's repository URL, else `repository`. It fails, writing nothing, naming the file, on a manifest that does not parse or does not have the expected shape, or a tracked file that is missing or not UTF-8.

### Agent attribution (pull requests, CI only)

A pull request's own commits carry the contributor's git identity and no agent attribution, and its description carries no agent footer; the rule is in [Commits and PRs in `AGENTS.md`](https://github.com/openplush/openplush/blob/main/AGENTS.md#commits-and-prs). [`scripts/check-attribution.sh`](https://github.com/openplush/openplush/blob/main/scripts/check-attribution.sh) fails on, in the commits the pull request adds, merge commits included:

- an author or committer email `noreply@anthropic.com`, or the author name `Claude`;
- a `Co-Authored-By:` line naming Claude or an `anthropic.com` address (a person as co-author passes);
- a `Claude-Session:` line;
- a "Generated with" or "Generated by" footer that links to Claude Code, or a link to a Claude Code session, in a commit message or the description.

Each failure names the commit, by short SHA and subject, or `PR body`, and the line that matched. Footers are matched by their link and trailers by the start of their line, so a description that discusses them in prose passes.

It reads only the commits between the pull request's base and its head, so commits already on `main` are never reported, whoever wrote them: history is not rewritten.

It runs as the `attribution` step of the `fmt, clippy, test` job in `.github/workflows/engine.yml`, on pull request events only, so a failure blocks the merge gate. It reads the description when the step runs, so after removing a footer, re-run the job. `scripts/check.sh` does not run it, because a local run has no pull request. To check your branch's commits before you push:

```sh
scripts/check-attribution.sh origin/main HEAD
```

With `PR_BODY` set to the description, it checks that too.
