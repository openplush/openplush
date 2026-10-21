# Project status

What is built and what is not, as of **2026-10-20**. This is the one page that answers "is X built yet?"; every other page links here instead of saying. The facts come from the ticked tasks in each feature's `tasks.md` and from the code on the branch: when the two disagree, the code is right and this page is out of date.

## Entries

OpenPlush is pre-alpha: nothing a maker can use exists yet. The engine stages exist as separate crates, but no command or app runs them end to end.

### By capability

In pipeline order. "Built" means merged, with its tests; each stage is described in [the architecture](../explanation/architecture.md) and its crate in [the crate reference](../reference/crates/README.md).

| Capability | Built | Not built yet |
|---|---|---|
| Import | OBJ, STL and PLY readers, each OpenPlush's own (`openplush-io`), picked by the file's extension (the `import` stage); polygons triangulated, a non-finite coordinate refused; glTF and GLB (the default scene, its buffers beside the file or embedded, material names, and a warning naming what it skips) | FBX |
| Model health and repair | Every defect located without changing the model (the `validate` stage); repair steps weld, degenerate faces, over-shared edges, bowtie vertices, orientation and small components, giving a [manifold](../reference/glossary.md#geometry) mesh (the `repair` stage); filling a hole the maker lists, with a patch triangulated, refined and smoothed to match the surface around it | — |
| Seams | Seam sets, seam chains, shortest-path seam drawing, [darts](../reference/glossary.md#geometry), the stuffing opening, [charts](../reference/glossary.md#geometry) cut open along seams with their sewing map, the disk and fold checks with a proposed fix (the `chart` stage) | Dart suggestions from distortion; the concave-region warning; seams imported from the model's UV map |
| Flatten | The boundary-pinned solver with an interior that cannot flip, the flip check, the not-closable fallback with a dart proposal, step-wise sessions, distortion metrics and a per-piece summary (`openplush-flatten`); whole charts laid flat in chart order, with the refusals mapped and a dart offered as a proposal (the `flatten` stage) | The stepping session that caches each chart's layout; timing in the browser |
| Pattern | Outlines and pattern pieces, per-edge [seam allowance](../reference/glossary.md#sewing), the seam-length parity check, real-world scale, the fabric type (`openplush-pattern`); each chart's layout made into a piece in millimetres with its cut line, after every seam's parity is checked (the `pattern` stage); the scale for the maker's finished size, warned outside 75 mm to 1000 mm (the `size` stage) | Fabric presets (waiting for measured values); notches, labels, grain and nap arrows; collapsing identical and mirror-image pieces |
| Layout | Shelf packing on a roll, as-printed and flipped copies, page tiling for Letter and A4 with overlap, registration marks and a scale square (`openplush-layout`) | Fabric-aware rotation; optimised nesting (a later release) |
| Export | True-scale SVG and tiled PDF, with the warning summary block (`openplush-export`); both driven by the pipeline from a set of pieces and their warnings (the `layout`, `export_svg` and `export_pdf` stages) | The web app's export |
| End to end | `import`, `validate`, `repair`, `chart`, `flatten`, `size`, `pattern`, `layout`, `export_svg` and `export_pdf` through `openplush-pipeline`; `validate`, `repair` and `pattern` from the command line, `pattern` writing the SVG and the PDF | The remaining stages and their memoisation; `pattern`'s `pieces.json`, `metrics.json` and `warnings.json`; the end-to-end and determinism tests |
| App | `@openplush/engine-client`: the typed worker protocol, a mock worker, crash handling, and the first slice of the real worker host | The web app (`apps/web` does not exist); the WebAssembly binding exports no operation yet |

Two crates have no code: `openplush-fields` and `openplush-decal`. `openplush-project` (the project file) has only the provenance keys a saved seam or marking will be stored by; the file itself is not built.

### By feature

How far each feature's spec and plan have got, and where its work stands.

| Feature | Spec and plan | Where it stands |
|---|---|---|
| [001 pattern pipeline](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/tasks.md) | Spec draft, its blocking questions answered 2026-10-13 and three minor defaults open; planned | Setup and foundations done except the determinism and golden workflows and the golden compare, which wait for output to compare. The first user story (seams in, printable pattern out) is mostly built engine-side, as above; its pipeline wiring, CLI and web shell are not. Later stories have barely started |
| [002 seam-placement hints](https://github.com/openplush/openplush/blob/main/specs/002-seam-placement-hints/tasks.md) | Spec draft, its questions answered 2026-10-13; planned | Fixtures and two spikes (the default thresholds, symmetry) done; no code. Its crate needs 001's pipeline first |
| [003 source seam import](https://github.com/openplush/openplush/blob/main/specs/003-source-seam-import/tasks.md) | Spec draft, its questions answered 2026-10-16 and 2026-10-17; planned | Fixtures and spikes done; materials read from OBJ and glTF and carried on the soup and the mesh, the UV-cut tolerance set, and the engine finding a model's UV cuts, projection points and material boundaries, with each cut that stops partway proposed as its own dart; the import stage, the CLI and the app not yet |
| [004 docs site](https://github.com/openplush/openplush/blob/main/specs/004-docs-site/tasks.md) | Spec draft, no clarification session; planned; five decisions accepted, the rest proposed | The site decision, the writing guide and page templates, the agents' docs procedure, the book build with its navigation check, links out of the book pointing at GitHub, the onboarding pages (start here, setup, this page, the [glossary](../reference/glossary.md)), the engine-concept and agents explanations, the pull-request how-to, the contributor book's landing page, `CONTRIBUTING.md` as a pointer into the book, the [first-change tutorial](../tutorials/first-change.md), the link and network checks, the site's landing page, its workflow with the determinism job, the maker guide's first pages with the plain-language check, the facts the agent files held moved into the book (meshes, code rules, commands, review and merge), the task counts below generated from each `tasks.md`, and a check that fails on a task count copied anywhere else, generated folder index lists, one page per crate, and the API reference built from rustdoc done; publishing not yet |

Each feature's ticked tasks, out of all the tasks in its `tasks.md`. [`scripts/gen-status.py`](../reference/checks.md#generated-status-counts-docs) writes this table from those files each time `scripts/check.sh docs` builds the book, so it is never out of date:

{{#include status-counts.inc}}

### Known gaps

- **No real plush model with a known-good hand-made pattern** is in `fixtures/`; every model there is generated. Several success criteria, 002's headline one among them, cannot be measured until one arrives. [Contributing a test model](README.md#the-lanes) needs no code.
- **The fabric preset values are unmeasured.** They wait for someone who sews with fleece, minky and faux fur to review them.
- **Flattening speed in the browser is unmeasured**: the spike timing the solver on a large chart under WebAssembly is open.
