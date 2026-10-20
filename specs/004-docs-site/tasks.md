---
description: "Task list for 004 docs site"
---

# Tasks: A Documentation Site for Contributors and Makers

**Input**: Design documents in `/specs/004-docs-site/`
**Prerequisites**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md), [quickstart.md](./quickstart.md)
**Generated**: 2026-10-20

**Tests first where there is code**: every check under `scripts/` lands after a failing test under `scripts/tests/`, as `AGENTS.md` requires. Pages have no test commit; their gate is the site checks plus review.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: which user story it serves (US1 to US5 in spec.md)
- `needs:ID[,ID]`: unticked tasks this one waits on, where the phase order alone does not say so
- `blocked:W-0NN`: a proposed decision this one waits on; only the maintainer clears it
- `human:XX`: needs a person; no agent picks it up
- Task IDs are `W0NN`, distinct from 001's `T`, 002's `H` and 003's `U`. This feature's decisions are `W-0NN`, as in plan.md.

## PR packaging

Same policy as 001–003: one intent per PR, test commit before implementation commit, interdependent intents stacked ([ADR 0013](../../docs/adr/0013-intent-sized-pull-requests.md)). Branches are `004/NN-slug`; 01 is this spec.

### Batch these

| PR | Tasks |
|---|---|
| `004/NN-authoring-standards` | W002, W003 (the guide and the templates it describes) |
| `004/NN-docs-build-gate` | W006, W007, W008 |
| `004/NN-docs-network-gate` | W010, W011 |
| `004/NN-onboarding-path` | W018–W022 |
| `004/NN-engine-concepts` | W026–W029, or one PR each if review prefers |
| `004/NN-maker-guide` | W034, W035 |

### Isolate these

| Task | Why |
|---|---|
| **W001** ADR | A decision the maintainer accepts before the code that relies on it |
| **W004, W005** docs skill and `AGENTS.md` | They change how every agent writes docs; a reviewer should read them alone |
| **W009** link rewrite | Touches many pages mechanically; a focused diff is easy to verify |
| **W024** facts out of agent files | Changes agent behaviour; never folded into a content PR |

---

## Phase 0: Decisions

- [x] W001 `docs/adr/0017-docs-site.md`: mdBook for both books (W-001); OFL-1.1 accepted for font files in the published docs site only (W-007); CI-only tooling outside Constitution IX, ratifying `Swatinem/rust-cache` (W-008); a licence record for the `cargo install`ed tools (mdbook MPL-2.0, lychee Apache-2.0 OR MIT). Considered options from research R1 and R4. `human:merge`. Also records `wasm-bindgen-cli`; mdBook's inline Font Awesome icons are CC BY 4.0 (found in W008), so W-007 covers them too

## Phase 1: Authoring standards (US4)

- [x] W002 [US4] `docs/how-to/write-docs.md`: the writing guide (W-011). Sections in the order an author needs them: pick the audience (contributor, maker), pick the kind (tutorial, how-to, explanation, reference, maker task, FAQ entry) with one line on each, copy the template, place it in the right `SUMMARY.md`, write, run `scripts/check.sh docs`. Rules: contributor pages open with their purpose, process IDs only as links or a closing "Decisions" line, every term in the glossary; maker pages use maker words, no code, IDs or crate names; no empty sections; status only on the status page; a change to public or maker-facing behaviour updates its page in the same PR
- [x] W003 [P] [US4] `docs/_templates/{tutorial,how-to,explanation,reference,maker-task,faq-entry}.md`: one per kind, each a skeleton with headings and a one-line instruction under each, nothing else. Tutorial and how-to open with "You will" and "Before you start". The ADR template stays at `docs/adr/0000-template.md` and the guide links to it. needs:W002
- [x] W004 [US4] `.claude/skills/docs/SKILL.md`: procedure only (W-011). Steps: classify audience and kind, copy the template, add the SUMMARY entry, write, run the docs checks, ask `docs-reviewer` for a public-surface change. Every rule is a link into `docs/how-to/write-docs.md`; the skill states none. needs:W002,W003. As built: the review on opening runs `docs-reviewer` on any change under `docs/`, so step 7 tells agents not to run it locally (`steward`)
- [x] W005 [US4] `AGENTS.md` "Specs and docs" and `.claude/agents/docs-reviewer.md`: add the maker-guide trigger (a maker-facing change updates `docs/user/`) and point to the writing guide and the docs skill. No rule restated. needs:W004

## Phase 2: Site infrastructure (US2, US5)

- [x] W006 [US2] `scripts/check.sh`: `MDBOOK_VERSION` and `LYCHEE_VERSION` pinned beside `DENY_VERSION`; `--setup` installs both. Confirm whether `taiki-e/install-action` offers lychee (research R4 `[verify]`); fall back to `cargo install --locked`. As built: install-action has no lychee (research R4), so `--setup` installs both with `cargo install --locked`, and only when the docs tools are needed; the pins are not mirrored in `engine.yml`, which installs neither
- [x] W007 [P] [US2] `scripts/tests/check-docs-summary.test.sh`: failing tests for the navigation check on a fixture tree. Fails naming a page in no `SUMMARY.md`; fails on a page in both books; ignores `docs/_templates/`; assigns `docs/user/**` to the maker book and the rest to the contributor book; passes on a complete tree
- [x] W008 [US2] `scripts/check-docs-summary.sh`, `book.toml` (`src = "docs"`, `create-missing = false`, `edit-url-template`, `git-repository-url`), `docs/SUMMARY.md` over today's pages in path order with empty sections left out (W-004), and a `docs` step in `scripts/check.sh` that builds the book and runs the check. Makes W007 pass. needs:W006,W007. As built: `scripts/check.sh docs` is not in the default, `all` or `engine` runs, since the plan's CI-cost question (plan.md, Open questions) is still open; `edit-url-template` uses mdBook's `{path}`, which on 0.5.4 already starts with `docs/`; the `<placeholders>` in ADR 0016 produce mdBook warnings only; Rust code blocks link to the playground until W011
- [x] W009 [US2] Rewrite every relative link in `docs/` that leaves `docs/` (37 at audit) to `https://github.com/openplush/openplush/blob/main/…`, and make README links resolve: disable the `index` preprocessor if that works (research R2 `[verify]`), else a small repo-owned preprocessor with its own failing test first. needs:W008. As built: 42 links at recount, directories to `/tree/main/…`; `use-default-preprocessors = false` with `links` re-enabled works on mdBook 0.5.4 (research R2), so no repo-owned preprocessor
- [x] W010 [P] [US2] `scripts/tests/check-docs-remote.test.sh`: failing tests for the network check on fixture HTML and CSS. Fails on an `http(s)://` `script src`, `link href`, `img src`, CSS `url()` and `@import`; passes on plain `<a href>` links to other sites and on relative references. As built: also fails on protocol-relative `//host/...` references and on `url()` or `@import` in an HTML `style` attribute or element, and on a directory with no HTML; passes on `data:` URIs; the `docs` step's case in `scripts/tests/check.test.sh` landed here too
- [x] W011 [US2] `scripts/check-docs-remote.sh` run on the built site by the `docs` step; `book.toml` sets `[output.html.playground] runnable = false` and no MathJax (W-006). Makes W010 pass. needs:W008,W010. As built: the check reads HTML and CSS only; `book.js` still names `play.rust-lang.org`, but fetches it only for `.playground` blocks, which `runnable = false` stops mdBook emitting; MathJax is off by default on mdBook 0.5.4, so `book.toml` has no key for it (a test book with `$$x^2$$` built with no `mathjax` reference). The check also covers `srcset`, `poster`, `<object data>` and SVG `href`/`xlink:href`. It scans the assembled `target/site`, landing page included
- [x] W012 [US2] `scripts/tests/check-docs-links.test.sh` then the gate: the test builds a fixture book with a dead link and a dead anchor and expects failure; the `docs` step runs `lychee --offline --include-fragments` over `target/site/` (W-005). needs:W009. As built: `scripts/check-docs-links.sh <dir>` wraps lychee with `--root-dir` set to the site directory, without which mdBook's `404.html` (`<base href="/">`) fails the check; the `docs` step runs it on the assembled `target/site`, landing page included; the self-test needs mdbook and lychee, so the `docs` step runs it and the `boundary` step skips it
- [x] W013 [US2] `docs/site/index.html`: landing page with "For contributors" (to `dev/`) and, hidden until W036, "For makers" (to `guide/`). Self-contained, no external resources; the `docs` step assembles `target/site/`. needs:W008. As built: "For makers" is an HTML comment, not a `hidden` element, since lychee checks hidden links and `guide/` is not built yet; W036 uncomments it. mdBook copies every non-Markdown file under `src`, so the `docs` step deletes the copy at `target/site/dev/site/` and copies the page to `target/site/index.html`; the navigation check reads only `.md` files, so it needs no rule for the page
- [x] W014 [US2] `.github/workflows/docs.yml`: on `pull_request` and `push` to `main`, run `scripts/check.sh docs`; on PRs upload `target/site/` as a 7-day artifact (W-012); on `main`, remove `target/site/guide/` (W-003) and deploy with job-level `pages: write` and `id-token: write`, environment `github-pages`, `concurrency: pages` without cancel. Every action SHA-pinned; never `pull_request_target`. needs:W011,W012,W013. As built: mdbook comes from `taiki-e/install-action` as a checksummed binary, at the version `scripts/tests/check.test.sh` holds equal to `MDBOOK_VERSION`; `scripts/check.sh --setup docs` then builds only lychee. `book.toml` sets no `site-url`: it only changes mdBook's `dev/404.html` `<base href>`, Pages serves a custom 404 only from the site root, and `/openplush/dev/` makes lychee fail on the base href itself, since the local site root has no `/openplush/` prefix
- [ ] W015 [US2] Enable Pages with Source "GitHub Actions" and restrict the `github-pages` environment to `main`. `human:settings` needs:W014
- [ ] W016 [P] [US5] `scripts/tests/assemble-docs-api.test.sh` then `scripts/assemble-docs-api.sh`: `cargo doc --workspace --no-deps --locked` copied to `target/site/dev/api/` without `.lock`, plus a generated `api/index.html` listing every workspace crate in name order; the test fails on a missing crate or a leftover dotfile. `docs/SUMMARY.md` links to it. needs:W012
- [x] W017 [US2] `docs.yml` determinism job: build the site twice into separate directories and compare byte for byte (SC-007). Workflow only, not the local `docs` step, to keep local runs fast. needs:W014. As built: the job runs `scripts/check.sh docs`, moves `target/site` to `target/site-1`, runs it again and fails on any `diff -r "$RUNNER_TEMP/site-1" target/site`; it repeats the build job's setup steps rather than sharing them, as the other workflows do, and `scripts/tests/check.test.sh` asserts the diff is there. The site is the mdBook book plus the landing page; rustdoc stays out until W016's decision (W-007). Locally, two builds in one tree and a build from another directory were identical

## Phase 3: Onboarding path (US1) 🎯 MVP

- [x] W018 [US1] `docs/start/status.md`: the one status page (FR-015), by capability (import, repair, seams, flatten, layout, export, app) and feature. Replace every other status statement with a link: `README.md`, `CONTRIBUTING.md`, `specs/README.md`, `docs/reference/crates.md` — *2026-10-20: dated 2026-10-20; also rows for pattern and end to end, and a known-gaps list that took over the stale holes in `specs/README.md`, whose Implementation column went. Review the same day: 004's row brought up to date; a spec-and-plan column took over `specs/README.md`'s stale Status table (179 tasks for 189, none for 003's 71), which became a feature list; the open 001 defaults (three, not five) counted on the status page only. `docs/explanation/architecture.md`'s Status column (it still calls the flatten solver planned), `docs/reference/README.md` and `docs/reference/cli.md` still state status: a follow-up*
- [x] W019 [P] [US1] `docs/start/README.md` "Start here": what OpenPlush is in three sentences, who each part is for, the path through the contributor book, where to ask — *2026-10-20: the path links only pages that exist (no first-change step until W033); "where to ask" is GitHub issues through the issue forms, since blank issues are off and no Discussions link exists in the repo. Review the same day: the lanes table, a copy of `CONTRIBUTING.md`'s, became a link to it, and the task-readiness rule moved to the glossary, which states it in full*
- [x] W020 [P] [US1] `docs/start/setup.md`: prerequisites and the three commands to a passing run of every gate (SC-001), verified by running them in a clean container and saying so in the PR — *2026-10-20: the third command is `scripts/check.sh all docs`, since plain `scripts/check.sh` leaves out `docs`. Verified from nothing: a fresh `git clone`, empty `CARGO_HOME` and `RUSTUP_HOME`, and no pinned tool on `PATH` (Node, pnpm, jq and python3 preinstalled, as the page's prerequisites say). `scripts/check.sh --setup` installed the toolchain, mdBook, lychee, cargo-deny, the wasm test runner and actionlint in 9 min 45 s, and `scripts/check.sh all docs` ended `check: ok` in 6 min 17 s. rustup's own installer was blocked by the network proxy, so its binary was copied in and `rustup default stable` set, as the installer would*
- [x] W021 [P] [US1] `docs/reference/glossary.md`: geometry terms, sewing terms, process terms, and an ID legend for every prefix in use (D-, T, H, U, W, FR-, SC-, US, S-, R, Q, W-) — *2026-10-20: the legend also has H-, P, §, ADR and the constitution's numerals, and splits the letters used twice: H is 002's tasks (H0NN) and spikes (H1–H5), Q is a clarification in 001 and 002 but a spike in 003, S is 001's spikes; also opening and constitution as terms*
- [x] W022 [US1] `docs/README.md` becomes the contributor book's landing page: who it is for and the path, replacing the Diátaxis table. Remove the promise of a page that does not exist, or point it at W027. needs:W018,W019,W020,W021 — *2026-10-20: the path itself stays on Start here, its one home; the landing page says who the book is for, sends newcomers there, and lists the sections by reader need, with the flattening page as the explanation example. The tutorial example that promised a missing page is gone*
- [ ] W023 [US1] Cold-reader test (SC-002): someone new to the repo answers the five US1 questions from the site alone within 30 minutes; gaps become tasks. `human:reader` needs:W022

## Phase 4: Facts out of agent files (US4)

- [x] W024 [US4] Move to `docs/` (W-009): the geometry gotchas (`crates/AGENTS.md`) into `docs/explanation/meshes.md`; the crate ownership table into `docs/reference/crates/`; the merge-tier table (steward skill) into `docs/how-to/review-and-merge.md`; the rationale behind the code rules into `docs/reference/code-rules.md`. The agent files keep each directive as one line plus a link. The commands table stays in `AGENTS.md` behind an anchor and `docs/reference/commands.md` includes it (verify the out-of-root include; else make the page canonical and link from `AGENTS.md`). needs:W009 — *2026-10-20: W-009 accepted by the maintainer the same day. The ownership table went to `docs/reference/crates.md#which-crate-owns-what`, not a `crates/` directory, since W031 is the task that splits that page into one; the reasons behind every rule in `crates/AGENTS.md` became `docs/reference/code-rules.md`, under the same headings, with the lint reasons left in `docs/reference/checks.md` and linked; the out-of-root `{{#include ../../AGENTS.md:commands}}` works on mdBook 0.5.4, with `<!-- ANCHOR -->` comments around the table, so `docs/reference/commands.md` is that include. The tier table became the how-to's `#tiers`, and the glossary, the pull-request how-to and the agents page link to it instead of listing paths. Left as found: `steward`'s Reviews section still says only the maintainer asks for a rerun, which ADR 0018 changed*
- [x] W025 [P] [US4] `docs/explanation/agents.md`: how agents work in this repo in plain words (Spec Kit flow, factory, steward, the merge gate) with links to the skills. As built: also the review on opening and standing authorizations; no glossary links, since the glossary (W021) is not on `main` yet; the tier table is left in `steward`, which W024 moves

## Phase 5: How the engine works (US1)

- [x] W026 [P] [US1] `docs/explanation/pipeline.md`: one model followed through every stage, the crate that owns each, what can fail there (FR-022). As built: follows the capsule fixture; with no glossary or status page on main, terms are plain prose and unbuilt stages link to the specs index's status table
- [x] W027 [P] [US1] `docs/explanation/flattening.md`: what flattening does and why seam lengths are pinned exactly
- [x] W028 [P] [US1] `docs/explanation/seams.md`: seams, charts, darts and openings, and why a chart must be a disk. As built: also covers the seam graph, the sewing map, mandatory seams and the fold check, since the chart stage refuses on them
- [x] W029 [P] [US1] `docs/explanation/sewing.md`: sewing concepts for programmers (allowance, grain, nap, notches). As built: notches and grain arrows are not built, so their sections say so and link to status

## Phase 6: Reference and tutorial

- [x] W030 `CONTRIBUTING.md` becomes a short pointer into the site; its human content moves to `docs/start/` and `docs/how-to/`. Assumes the plan's open question resolves to a pointer. needs:W018,W020 — *2026-10-20: W-014 accepted the pointer. The lanes table moved to Start here (`#the-lanes`, every link repointed); Spec Kit setup became `docs/how-to/author-specs.md`; the lints, opt-outs, wasm32 canaries and licence rules became `docs/reference/checks.md`, a reference page because the writing guide puts lookup facts there; the `engine` subset joined setup, and Conventional Commits, answering 🔴 findings and running reviewers locally joined the pull-request how-to and the agents page. The PR rules, review tiers and tests-first rules were already there and were dropped. Left pointing at CONTRIBUTING: the `scripts/check.sh` wasm comment and the `.github/` issue forms, which it still links on to*
- [ ] W031 [P] Split `docs/reference/crates.md` into `docs/reference/crates/<crate>.md`, one readable page each, with a short index. needs:W024
- [x] W032 [P] `docs/how-to/pull-requests.md` rewritten to the writing guide (IDs as links, purpose first); `docs/adr/README.md` gains "how to read an ADR". ADR bodies are records and are not edited. As built: kept as a how-to on the how-to template, its reasoning inline under each step; task IDs became links to `tasks.md` or plain descriptions, and rules stated in `AGENTS.md` or `steward` are linked, not restated. The page is still partly explanation; splitting the reasoning into `docs/explanation/` is a follow-up
- [x] W033 [US1] `docs/tutorials/first-change.md`: a code-level first change, test first, on crates that exist today. Assumes the plan's open question resolves to "now" — *2026-10-20: W-015 resolved it to now. The exercise adds `Mm::checked_add` to `openplush-geom`'s `units`, with an integration test for the sum and for overflow as `GeomError::InvalidLength`; every command and output was run on `main` and the change is thrown away, not committed. The start page's path gains the step*

## Phase 7: Maker guide (US3)

- [x] W034 [US3] `scripts/tests/check-docs-plain.test.sh` then `scripts/check-docs-plain.sh` (SC-006): fails on a process ID or a code span naming a crate or Rust/TS identifier in `docs/user/`. Then `docs/user/book.toml`, `docs/user/SUMMARY.md` and `docs/user/README.md` (what OpenPlush does, for a maker), built and checked by the `docs` step. needs:W008. As built: every code span and fenced block fails, not only one naming a crate or identifier, since a maker page has no reason for code (FR-030); the ID patterns are the glossary legend's prefixes, bounded so dates, paper sizes and counts pass; the check reads the Markdown, so it covers the navigation too. `docs/user/book.toml` mirrors the contributor book's settings with `src = "."` and builds to `target/site/guide/`, which the remote and link checks already cover and `docs.yml` already drops on `main`; `docs/how-to/README.md` stopped promising app guides
- [x] W035 [US3] `docs/user/faq.md`: questions every new maker asks, answered only where the answer exists in the code today (cost and licence, the model never leaves your computer, which files open); other questions wait for their feature. needs:W034. As built: three entries (cost and licence, the model stays on the device, OBJ, STL and PLY open); fabric and printing wait for the presets and for an app that exports
- [ ] W036 [US3] Publish the maker guide at the first public release: lift W-003, unhide the landing link. `human:release`

## Dependencies and execution order

- W001 first. W016 waits on W-007, which W001 decides.
- Phase 1 before any new page, so pages start from the templates.
- Phase 2's W006 → W008 → W009 → W012 → W014 is the critical path to a published site.
- Phases 3, 5 and 7 run in parallel once W008 is in. Phase 4 waits on W009 so moved content lands with working links.

## Notes

- A page added by another feature (a maker-facing change in 001–003) follows the writing guide and needs no task here.
- Stale-status fixes found along the way go in as their own `chore/` PRs, not into this feature.
