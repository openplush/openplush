---
status: Proposed
date: 2026-10-20
deciders: maintainer
---

# Build the docs site with mdBook; accept OFL-1.1 fonts in the published site only; CI-only tooling is outside Constitution IX

## Context and problem statement

Feature 004 publishes `docs/` as a static site: a contributor book, a maker guide, and the workspace rustdoc under `/dev/api/` ([spec](../../specs/004-docs-site/spec.md), [plan](../../specs/004-docs-site/plan.md)). Three questions need an answer before its tasks land. Which generator builds the books? May the site carry the fonts the generators bundle, which are OFL-1.1 and not on the allow-list? And does Constitution IX govern tools that only run in CI, such as `Swatinem/rust-cache` (LGPL-3.0), already pinned in three workflows, and the tools `cargo install` puts on a runner, which `cargo deny` never sees? This ADR records the plan's decisions W-001, W-007 and W-008 (task W001).

## Decision drivers

* Constitution IX: dependencies on the allow-list (MIT, Apache-2.0, BSD-2/3, ISC, Zlib, Unicode, MPL-2.0); nothing GPL, AGPL or LGPL in the shipped artifact.
* Constitution VIII: no network request on page load, so no remote fonts, scripts or stylesheets.
* Constitution V: the same commit builds a byte-identical site.
* Contributor burden: one more pinned binary, not a second language toolchain.
* Maintenance: a generator that is maintained today and will be next year.

## Considered options

Generator (W-001):
* A: mdBook 0.5.x
* B: Zola
* C: Material for MkDocs, or its successor Zensical
* D: Starlight, Docusaurus or VitePress

Fonts (W-007):
* E: accept OFL-1.1 for font files in the published docs site only
* F: strip or replace the bundled fonts
* G: publish no rustdoc

CI-only tooling (W-008):
* H: IX governs what the project ships; tooling that only runs in CI is outside it, and its licences are recorded
* I: IX covers CI tooling too; replace `Swatinem/rust-cache` and hold every CI tool to the allow-list

## Decision outcome

**Generator: A, mdBook 0.5.x**, for both books. It is the only candidate that is on the allow-list (MPL-2.0), a single binary, privacy-clean by default and actively maintained [solid: research R1]. Two builds of the same input were byte-identical [solid: measured, research R1].

**Fonts: E.** Font files licensed OFL-1.1 may appear in the published docs site and nowhere else: not in the app, the WASM package, the CLI, or an exported PDF. rustdoc bundles FiraSans, FiraMono, SourceSerif4, SourceCodePro and NanumBarunGothic with no stable option to omit them [solid: research R4], and mdBook's default theme bundles Source Code Pro [solid: research R2]. The fonts are served from the site itself, never fetched from elsewhere, so VIII holds. OFL-1.1 lets fonts be bundled and redistributed with software; it forbids selling a font on its own, which the site never does. [ADR 0008](0008-pdf-export.md)'s choice to embed no fonts in exported PDFs is unchanged.

**CI-only tooling: H.** Constitution IX governs what the project ships and the code it is licensed under, the reading [ADR 0011](0011-blender-research-tool.md) already applies. A tool that runs in a CI job and leaves nothing in a shipped artifact or in the published site is outside it. This ratifies `Swatinem/rust-cache` (LGPL-3.0). What such a tool writes into a published or shipped output is not CI-only: the fonts above fall under E, not H.

Licences of tools that `cargo install` puts on a runner or a contributor's machine, which `cargo deny` never checks:

| Tool | Licence | Used by |
|---|---|---|
| `mdbook` | MPL-2.0 | the `docs` step |
| `lychee` | Apache-2.0 OR MIT | the `docs` step |
| `cargo-deny` | Apache-2.0 OR MIT | the `deny` step |
| `wasm-bindgen-cli` | Apache-2.0 OR MIT | the `wasm` step; its generated glue ships in the WASM package, so it is held to the allow-list, which it passes |

### Consequences

* Good, because the site builds from one pinned binary with no Python or Node toolchain, and its output is reproducible.
* Good, because the full API reference can be published as rustdoc writes it, with no patched theme to maintain.
* Good, because a CI tool's licence no longer reads as a breach of IX, and the reading is written down once.
* Bad, because the published site carries files under a licence that is not on the allow-list. The exception is scoped to font files in the docs site; any other asset outside the allow-list needs its own ADR.
* Bad, because H moves a judgement to review: whether a tool's output reaches a shipped artifact is not something `cargo deny` can see.

### Confirmation

* Review: a PR that adds an OFL font, or any asset outside the allow-list, to anything but the docs site is rejected; a CI tool whose output lands in a shipped artifact or the published site is held to the allow-list.
* The `docs` step's build (W008) is the point where every third-party asset in the built site is listed with its licence. An asset that is neither on the allow-list nor an OFL-1.1 font stops that task for a new ADR. mdBook 0.5 embeds icons as inline SVG; their licence is not yet confirmed [verify].
* A tool added to `scripts/check.sh --setup` adds its row to the table above.

## Pros and cons of the options

### A: mdBook

* Good, because MPL-2.0, one binary, client-side search, no analytics since 0.5.0, and unknown config keys are errors.
* Bad, because its theming is plainer than the alternatives', and its default theme carries one OFL font.

### B: Zola

* Bad, because it is EUPL-1.2 since 0.22.0, which is outside the allow-list [solid: research R1].

### C: Material for MkDocs or Zensical

* Bad, because Material reaches end of life around 2026-11 and loads Google Fonts by default; Zensical is alpha; both add a Python toolchain [solid; the end-of-life date is likely].

### D: Starlight, Docusaurus or VitePress

* Bad, because each adds a Node toolchain to the docs lane, and each has a blocker of its own: Astro's CLI sends telemetry unless disabled, Docusaurus has no offline search built in, VitePress stable has been idle since 2025-08.

### E: OFL in the docs site only

* Good, because rustdoc and mdBook are used as shipped.
* Bad, because it is an exception to the allow-list, however narrow.

### F: strip or replace the fonts

* Good, because the allow-list holds with no exception.
* Bad, because there is no stable option for it; a post-build step that deletes fonts and patches CSS breaks on each rustdoc release.

### G: no rustdoc

* Bad, because contributors lose the API reference, and the exception is still needed for mdBook's own theme.

### H: CI-only tooling is outside IX

* Good, because it matches what IX protects (what users receive) and the reading ADR 0011 already took.
* Bad, because it depends on review to see that a tool's output stays out of shipped artifacts.

### I: IX covers CI tooling

* Good, because one rule covers everything.
* Bad, because `rust-cache` would have to be replaced with hand-written caching, for no gain to any user.

## More information

* 004 plan, Decision Ledger W-001, W-007, W-008; research R1, R2, R4.
* Unblocks W016 (rustdoc under `/dev/api/`), which carries `blocked:W-007`.
* Revisit if rustdoc gains a stable way to omit its fonts, or if a CI tool starts writing into a shipped artifact.
