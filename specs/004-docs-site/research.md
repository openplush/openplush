# Research: A Documentation Site for Contributors and Makers

**Feature**: `004-docs-site` | **Date**: 2026-10-20 | **Status**: Active

Confidence tags: `[solid]` verified, `[likely]` reasoned inference, `[speculative]` guess, `[verify]` must be checked by the task that depends on it. Versions are as of 2026-10-20.

## R1. Which site generator

| Fact | Tag |
|---|---|
| mdBook 0.5.4 (2026-07-06) is MPL-2.0, one binary, 18 direct dependencies. 0.5.0 removed Google Analytics, bundles fonts locally, embeds icons as inline SVG, and turns on admonitions and definition lists. Unknown config keys and missing preprocessors are errors | [solid] crates.io, mdBook CHANGELOG |
| Two mdBook builds of the same input, one from another directory, were byte-identical. Asset names carry content hashes; the search index is built through a `BTreeMap` | [solid] measured |
| Search is client-side and loads only when opened | [solid] |
| Zola relicensed to EUPL-1.2 in 0.22.0 (2026-01-09): outside the allow-list | [solid] Zola CHANGELOG |
| MkDocs core has not released since 1.6.1 (2024-08-30). Material for MkDocs entered maintenance mode 2025-11-05 with about 12 months of fixes, so end of life around 2026-11-05; it loads Google Fonts by default | [solid] PyPI; [likely] for the end-of-life date |
| Zensical (Material's successor) is 0.0.71, "Alpha", and adds a Python toolchain | [solid] PyPI |
| Starlight is pre-1.0 and Astro's CLI sends telemetry unless disabled. Docusaurus has no offline search built in. VitePress stable has been idle since 2025-08 | [solid] |

**Decision**: mdBook (D-001).

## R2. How mdBook treats this repo's docs

| Fact | Tag |
|---|---|
| mdBook will not build without `SUMMARY.md`. `create-missing` defaults to true and silently creates empty pages for missing entries | [solid] tested |
| Every SUMMARY generator on crates.io (`mdbook-autosummary`, `mdbook-auto-gen-summary`, `mdbook-fs-summary`) is still on mdBook 0.4 and incompatible with 0.5 | [solid] |
| Well-regarded contributor guides (rustc-dev-guide, the Servo book) order their sidebars by a contributor's path, not by folder | [solid] |
| The `index` preprocessor renames `README.md` chapters to `index.html`, but body links are rewritten `.md` → `.html` with no existence check: `how-to/README.md` becomes a dead `how-to/README.html`, and `../AGENTS.md` becomes a dead `../AGENTS.html`. No warning is printed | [solid] tested |
| `docs/` has 37 relative links that leave `docs/` (to `specs/`, `research/`, the constitution, `AGENTS.md`, `crates/AGENTS.md`, `.github/`) | [solid] audit |
| Disabling the default `index` preprocessor makes README links consistent: on mdBook 0.5.4, `[build] use-default-preprocessors = false` plus `[preprocessor.links]` writes `README.html`, links to `README.md` resolve, and `{{#include}}` still works. `[preprocessor] index = false` is rejected | [solid] tested 2026-10-20: dead relative hrefs in the built book 86 → 0 |
| `mdbook-linkcheck2` 0.13 supports 0.5 but checks Markdown sources, so it misses the README breakage. lychee 0.24 (Apache-2.0 OR MIT) with `--offline` checks the built HTML | [likely] |
| A ```` ```rust ```` block is a playground block by default and `book.js` then POSTs to `play.rust-lang.org` on page load; `runnable = false` stops it. MathJax loads from cdnjs but is off by default | [solid] tested |
| The default theme's Source Code Pro font is OFL-1.1; Open Sans is Apache-2.0 | [solid] |

**Decisions**: hand-curated SUMMARY with a completeness check (D-004); links rewritten and lychee on HTML (D-005); network-free config and check (D-006).

## R3. Content: what a junior developer meets today

| Fact | Tag |
|---|---|
| `docs/tutorials/` holds only a placeholder; `how-to/` and `explanation/` have one page each | [solid] audit |
| No glossary exists anywhere. Domain terms (half-edge, chart, χ = 1, seam parity, allowance, nap, dart) and process-ID prefixes (D-, T, FR-, SC-, US, S, R, H, U, Q) are undefined | [solid] audit |
| ADR 0005 carries 53 ID references; `how-to/pull-requests.md` 21 | [solid] audit |
| Project status is stated in at least four places that disagree (README, CONTRIBUTING, `specs/README.md`, `reference/crates.md`) | [solid] audit |
| The geometry gotchas, the crate ownership table and the merge-tier table live only in agent files | [solid] audit |
| `docs/README.md` promises a page on why seam lengths are pinned that does not exist | [solid] audit |
| Newcomers' main barriers: finding where to start, missing prior knowledge, documentation, technical setup. An orientation portal helped with the first but not with setup, so the setup page must actually work | [solid] Steinmacher et al. 2015, 2016 |
| A short architecture page with a codemap is the most valuable newcomer page; describe only what rarely changes | [likely] matklad, "ARCHITECTURE.md" |
| Diátaxis' author warns against empty quadrant sections | [solid] diataxis how-to-use |
| Agent context files raised agent cost and step count with no significant gain in success; keep them to what the codebase does not already say | [likely] Gloaguen et al., arXiv 2602.11988 (search summary) |

**Decisions**: journey-ordered contributor book (spec FR-020); glossary and status page first (Phase 3); facts move out of agent files (D-009).

## R4. API docs and publishing

| Fact | Tag |
|---|---|
| `cargo doc --workspace --no-deps` writes no root `index.html` on stable; `--enable-index-page` is nightly-only | [solid] tested on 1.97.0 |
| rustdoc output uses relative paths, loads nothing remote, and was byte-identical across two builds; 13 MB, 670 files, one dotfile (`.lock`) | [solid] measured |
| rustdoc bundles FiraSans, FiraMono, SourceSerif4, SourceCodePro and NanumBarunGothic, all OFL-1.1, with no stable option to omit them | [solid] |
| Current Pages actions: `upload-pages-artifact@v5`, `deploy-pages@v5`. The deploy job needs `pages: write` and `id-token: write`; the `github-pages` environment can be restricted to `main` | [solid] |
| GitHub's native PR previews for Pages are alpha and not public; `rossjrw/pr-preview-action` needs branch-based Pages and fails for forks | [solid] |
| `Swatinem/rust-cache` is LGPL-3.0 and already pinned in `engine.yml`, `wasm.yml`, `review.yml` | [solid] |
| `taiki-e/install-action` installs a pinned mdbook with checksum verification (its `manifests/mdbook.json` lists 0.5.4 for Linux, macOS and Windows) | [solid] checked 2026-10-20 |
| `taiki-e/install-action` does not offer lychee: it is absent from `TOOLS.md` and `manifests/lychee.json` does not exist on `main`. `scripts/check.sh --setup` installs it with `cargo install --locked`, as it does `cargo-deny` | [solid] checked 2026-10-20 |

**Decisions**: rustdoc under `/dev/api/` with a generated index, gated on D-007; artifact previews (D-012).

## Sources

The full research report and notes, with links, were produced in the session that wrote this feature and are summarised here. Primary sources: mdBook v0.5.4 source and guide; rustdoc book; `actions/deploy-pages` and `actions/upload-pages-artifact` READMEs; crates.io, PyPI and npm registries; Zola CHANGELOG; Material for MkDocs blog (2025-11-05); diataxis.fr; Steinmacher et al. (ICSE 2016); matklad's ARCHITECTURE.md post; rustc-dev-guide and Servo book SUMMARY files.
