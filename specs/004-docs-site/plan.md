# Implementation Plan: A Documentation Site for Contributors and Makers

**Branch**: `004-docs-site` | **Date**: 2026-10-20 | **Status**: Draft
**Spec**: [spec.md](./spec.md) · **Research**: [research.md](./research.md) · **Quickstart**: [quickstart.md](./quickstart.md)

**Input**: Feature specification from `/specs/004-docs-site/spec.md`

## Summary

Publish `docs/` as a static site with two parts: a contributor guide ordered as a newcomer's path, and a maker guide (user guide and FAQ) that is built from day one but published only at the first release. Back it with one writing guide, one template per page kind, and a `docs` skill, so every author produces the same shape. Then move the human-facing facts that live only in agent files into `docs/`, and write the onboarding and concept pages a junior developer needs.

**Technically this is two mdBook books, the workspace rustdoc, three checks and one workflow.** No crate, no runtime code, no change to the engine or the app.

## Technical Context

| Field | Value |
|---|---|
| Tools | mdBook 0.5.x (MPL-2.0), lychee (Apache-2.0 OR MIT), rustdoc from the pinned toolchain. All pinned exactly; installed by `scripts/check.sh --setup` and by `cargo install --locked`, since `taiki-e/install-action` has no lychee (research R4) |
| Hosting | GitHub Pages, deployed by Actions from `main`; default address `https://openplush.github.io/openplush/` |
| Testing | Shell checks under `scripts/` with their tests as `scripts/tests/*.test.sh`, run by `scripts/check.sh docs` and the `shell` step |
| Constraints | No remote resources (Constitution VIII); byte-identical builds (V); licences on the allow-list or ADR-accepted (IX) |
| Scope | ~25 existing pages; ~20 new contributor pages; a maker guide skeleton plus FAQ |

No `data-model.md` or `contracts/`: the feature adds no types and no public engine surface.

## Constitution Check

| Principle | Status |
|---|---|
| I Engine/Shell separation | ✅ Nothing under `crates/` or `apps/` changes |
| II CLI parity | ✅ Not applicable |
| III Invariants / no silent wrong output | ✅ Dead links, unlisted pages and remote references fail the build instead of shipping (FR-010–FR-013) |
| IV Test-first | ✅ Each check lands as a failing test under `scripts/tests/` before the check itself |
| V Determinism | ✅ mdBook and rustdoc output measured byte-identical across builds (research R1, R4); SC-007 keeps it checked |
| VI Units | ✅ Not applicable |
| VII Explicit repair | ✅ Not applicable |
| VIII Local-first, no telemetry | ⚠️ mdBook's playground and MathJax defaults make network calls; both are turned off and a check forbids remote references (D-006) |
| IX Licences | ⚠️ rustdoc and mdBook bundle OFL-1.1 fonts (research R4); D-007 proposes accepting OFL for the docs site only, through an ADR. `Swatinem/rust-cache` (LGPL-3.0) is already in three workflows; D-008 proposes recording CI-only tooling as outside IX |

The two ⚠️ rows are resolved by the ADR (T001) before any task that depends on them; those tasks carry `blocked:D-007`.

## Project Structure

### Documentation (this feature)

```text
specs/004-docs-site/
├── spec.md
├── plan.md                    # this file
├── research.md                # tool comparison, mdBook behaviour, rustdoc and Pages, content audit
├── quickstart.md
├── checklists/requirements.md
└── tasks.md                   # T001–T036
```

### Repository changes

```text
book.toml                      # contributor book: src = "docs", build-dir = "target/site/dev"
docs/SUMMARY.md                # contributor navigation, hand-curated, journey-ordered (D-004)
docs/start/                    # NEW: start here, status, setup
docs/explanation/              # + pipeline, meshes, seams, flattening, sewing, agents
docs/how-to/write-docs.md      # NEW: the writing guide, the one home for doc rules (D-011)
docs/reference/glossary.md     # NEW: terms and ID legend
docs/reference/crates/         # crates.md split one page per crate
docs/_templates/               # NEW: one template per page kind; excluded from navigation by rule
docs/user/                     # NEW: maker guide, its own book
docs/user/book.toml            #   src = ".", build-dir = "../../target/site/guide"
docs/user/SUMMARY.md
docs/site/index.html           # NEW: landing page routing to /dev/ and (after release) /guide/
scripts/check-docs-*.sh        # NEW: summary, remote-reference and plain-language checks
scripts/assemble-docs-api.sh   # NEW: rustdoc copy and crate index
scripts/tests/*-docs-*.test.sh # their tests
scripts/check.sh               # + docs step; + MDBOOK_VERSION, LYCHEE_VERSION pins
.github/workflows/docs.yml     # NEW: build on PR (artifact preview), deploy on main
.claude/skills/docs/SKILL.md   # NEW: procedure; links to the writing guide
AGENTS.md                      # Specs and docs: + maker-guide trigger, + docs skill pointer
```

**Structure decision**: two books, not one book with two parts (D-002). A maker who opens the guide should never see a sidebar full of crate names, and mdBook gives each book one sidebar. The maker book lives under `docs/user/` so the contributor book's `src = "docs"` keeps today's paths; the summary check assigns `docs/user/**` to the maker book and everything else to the contributor book.

## Architecture

### Published layout

```text
https://openplush.github.io/openplush/
├── index.html        landing page: "For makers" (after release) · "For contributors"
├── dev/              contributor book
│   └── api/          workspace rustdoc + generated crate index
└── guide/            maker book (published from the first release; D-003)
```

The URLs are fixed from the first deploy, so the release adds `/guide/` without moving anything.

### Build

```text
mdbook build                 ─▶ target/site/dev/
mdbook build docs/user       ─▶ target/site/guide/
cargo doc --no-deps --locked, one crate at a time ─▶ copy to target/site/dev/api/, drop .lock, write api/index.html
cp docs/site/index.html      ─▶ target/site/
checks: summary completeness (sources) · remote references (output) · plain language (maker sources) · lychee --offline (output)
```

The deploy job removes `target/site/guide/` before upload until D-003 is lifted, so the checks always see the whole site but only the contributor part ships.

As built, the rustdoc is documented one crate at a time, dependencies first: [Checks, Generated API pages](../../docs/reference/checks.md#generated-api-pages-docs).

### Writing standards (summary; the writing guide is their home)

- **Audience first, then kind.** Every page belongs to one part and one Diátaxis kind; the template carries both.
- **Contributor pages**: open with what the page is for. A process ID never stands in for an explanation; it appears as a link or in a closing "Decisions" line, and the glossary explains its prefix.
- **Maker pages**: the maker's words (pattern piece, seam allowance, fabric), no code, no IDs, no crate names; each message page says what happened and what to do.
- **No empty sections**: a section enters the navigation with its first real page.

### Agent files and human docs

`docs/` holds every fact a human needs. Agent files keep directives (stop conditions, never-do rules, procedures) and link to `docs/` for the facts. One exception: the commands table stays in `AGENTS.md`, because agents read it every turn, and the site includes it by anchor (D-009). The docs skill is procedure only and links into the writing guide.

## Phasing

| Phase | What | Gate |
|---|---|---|
| 0 | ADR for tools and licences | Maintainer merges |
| 1 | Writing guide, templates, docs skill | Maintainer merges (skill, `AGENTS.md`) |
| 2 | Site infrastructure: books, checks, workflow, Pages | Maintainer merges; maintainer enables Pages |
| 3 | Onboarding path: start here, status, setup, glossary | Auto-merge tier for `docs/`-only |
| 4 | Facts out of agent files into `docs/` | Maintainer merges (agent files) |
| 5 | Engine concepts | Auto-merge tier |
| 6 | Reference reshape | Mixed |
| 7 | Maker guide skeleton and FAQ | Auto-merge tier; publishing is a maintainer decision |

Phases 3, 5 and 7 can run in parallel once Phase 1 and T008 are in.

## Decision Ledger

| Date | ID | Decision | Status |
|---|---|---|---|
| 2026-10-20 | D-001 | **mdBook 0.5.x** for both books. The only candidate on the licence allow-list that is a single binary, privacy-clean by default and actively maintained; Zola is EUPL-1.2 since 0.22, Material for MkDocs reaches end of life 2026-11, Zensical is alpha, Starlight's build sends telemetry by default (research R1) | Accepted in [ADR 0017](../../docs/adr/0017-docs-site.md), 2026-10-20 |
| 2026-10-20 | D-002 | **Two books, one site**: contributor book at `/dev/`, maker book at `/guide/`, a static landing page at `/`. Separate sidebars keep the maker guide free of contributor vocabulary | Proposed |
| 2026-10-20 | D-003 | **The maker guide is built and checked from its first page but published only at the first public release.** Pages describe what exists on `main`; publishing waits until that is released (spec FR-005, FR-032); lifting this is a maintainer decision | Proposed |
| 2026-10-20 | D-004 | **Hand-curated `SUMMARY.md` per book, journey-ordered**, with a check that fails on any page in neither navigation, and `create-missing = false`. A generated sidebar would sort alphabetically or need a second file for the order (research R2) | Proposed |
| 2026-10-20 | D-005 | **Links leaving the published tree become GitHub URLs**, rewritten once; the 37 existing ones are fixed in one task. `lychee --offline --include-fragments` checks the built HTML, not the Markdown, because mdBook rewrites `.md` to `.html` without checking (research R2) | Proposed |
| 2026-10-20 | D-006 | **No network on page load**: `[output.html.playground] runnable = false`, no MathJax, no remote `additional-*`; a check fails on any `http(s)://` resource reference (`src`, `href` on `link`, CSS `url()`, `@import`) in the built output | Proposed |
| 2026-10-20 | D-007 | **OFL-1.1 accepted for font files, and CC BY 4.0 for mdBook's inlined Font Awesome icons, in the published docs site only**, not the app, the WASM package or exported PDFs. rustdoc bundles OFL fonts with no stable way to drop them, so `/api/` needs this; ADR 0008's PDF font question stays separate | Accepted in [ADR 0017](../../docs/adr/0017-docs-site.md), 2026-10-20. Blocks T016 |
| 2026-10-20 | D-008 | **CI-only tooling is outside Constitution IX**, which governs what the project ships (ADR 0011's reading). Ratifies `Swatinem/rust-cache` (LGPL-3.0), already pinned in three workflows. The ADR also records the licences of `cargo install`ed tools, which `cargo deny` never sees | Accepted in [ADR 0017](../../docs/adr/0017-docs-site.md), 2026-10-20 |
| 2026-10-20 | D-009 | **`docs/` is the home of every fact a human needs; agent files keep directives and link.** Exception: the commands table stays in `AGENTS.md` and the site includes it by anchor | Accepted (maintainer, 2026-10-20) |
| 2026-10-20 | D-010 | **Maker-facing message explanations live in the maker guide.** `docs/reference/` keeps the exact strings and codes for contributors and links to the maker explanation, so each fact has one home | Proposed |
| 2026-10-20 | D-011 | **The writing guide (`docs/how-to/write-docs.md`) is the one home for documentation rules.** Templates carry structure; the docs skill carries steps and links to the guide | Proposed |
| 2026-10-20 | D-012 | **PR previews are a downloadable workflow artifact**, not a live host. GitHub's native Pages previews are not public, and branch-based preview actions need write tokens and fail for forks (research R4) | Proposed |
| 2026-10-20 | D-013 | **No diagrams tool until a page needs one.** `mdbook-mermaid` adds ~2.7 MB per page; the first diagram task decides between it and committed SVG | Proposed |
| 2026-10-20 | D-014 | **CONTRIBUTING.md is a short pointer into the site** (Bevy's model): GitHub shows it on new issues and PRs, so it keeps only what a first-time contributor needs at that moment and links the site for the rest; two full homes would drift | Accepted (standing authorization, 2026-10-20) |
| 2026-10-20 | D-015 | **The first-change tutorial is written now**, on crates that exist, rather than after the pipeline runs end to end: a newcomer needs a code path today, and a tutorial built on stable leaf crates survives later stages landing | Accepted (standing authorization, 2026-10-20) |
| 2026-10-20 | D-016 | **Copied facts are generated, not hand-kept**: an in-repo generator writes them at build time, into an include the `docs` step creates before building the book and git ignores. Not `mdbook-cmdrun`, which renders a failing command's error into the page instead of failing the build and runs arbitrary commands at build time. Not a committed include with a diff check either: every tick anywhere left it stale and made it a conflict point between pull requests | Accepted (maintainer, 2026-10-21: build-time) |
| 2026-10-20 | D-017 | **A lint that meets a decision agents cannot make reports it through a reviewed exceptions file**, each line with its reason, and fails when the exception goes stale; it never changes a decision's status to get green | Accepted (standing authorization, 2026-10-21; grant on the 004 ledger #271) |
| 2026-10-20 | D-018 | **`docs/SUMMARY.md` stays hand-curated** (D-004's reading order), **but each folder's index list is generated** from its pages, since parallel docs PRs conflict on those lists ([#320](https://github.com/openplush/openplush/issues/320)). The decision-ledger layout is decided in 005 | Accepted (maintainer, 2026-10-21) |
| 2026-10-23 | D-019 | **The dependency graph is an interactive page generated at build time**, the first diagram D-013 waited for. An in-repo Python script (standard library only, like `scripts/gen-indexes.py`) reads `git ls-files` and writes one self-contained page to `target/site/dev/depgraph/` after the book builds, so it cannot go stale and is never committed (D-016). D3 7.9.0 (ISC, on the allow-list) is vendored under `scripts/depgraph/` and inlined, with its licence published beside the page, so the page loads nothing from the network (D-006). Cost, measured on 2026-10-23: 537 nodes and 1,787 edges, about 650 KB for the one page (370 KB of data, 280 KB of D3), under a second to generate, and two runs byte-identical. Rejected for this page: `mdbook-mermaid`, about 2.7 MB on every page and no search or filter at 500 nodes; a committed SVG, stale on the next crate. D-013's choice for small diagrams inside a page stays open | Proposed |

### Trade-offs accepted

- **A hand-curated sidebar can lag a new page.** The summary check turns that into a failed build, not a hidden page.
- **The maker guide is written before anyone reads it.** It keeps maker-facing features explainable and avoids a catch-up project at release.
- **Two books mean two `book.toml` files.** Shared settings are duplicated in two files; a third book would be the point to generate them.

### Open questions

- **CI cost**: does the `docs` step join the default `scripts/check.sh` run and the required checks, or stay a separate non-required workflow? `cargo doc` adds build time.
- **CONTRIBUTING.md**: resolved 2026-10-20 by D-014: a short pointer into the site, as T030 assumed.
- **First tutorial**: resolved 2026-10-20 by D-015: a code-level tutorial on existing crates now, as T033 assumed.
