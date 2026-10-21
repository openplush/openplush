# Project status

What is built and what is not, as of **2026-10-20**. This is the one page that answers "is X built yet?"; every other page links here instead of saying. The facts come from the ticked tasks in each feature's `tasks.md` and from the code on the branch: when the two disagree, the code is right and this page is out of date.

## Entries

OpenPlush is pre-alpha: nothing a maker can use exists yet. The engine stages exist as separate crates, but no command or app runs them end to end.

### By capability

In pipeline order. "Built" means merged, with its tests; each stage is described in [the architecture](../explanation/architecture.md) and its crate in [the crate reference](../reference/crates.md).

| Capability | Built | Not built yet |
|---|---|---|
| Import | OBJ, STL and PLY readers, each OpenPlush's own (`openplush-io`), picked by the file's extension (the `import` stage); polygons triangulated, a non-finite coordinate refused | glTF and GLB; material names read from the file |
| Model health and repair | Every defect located without changing the model (the `validate` stage); repair steps weld, degenerate faces, over-shared edges, bowtie vertices, orientation and small components, giving a [manifold](../reference/glossary.md#geometry) mesh (the `repair` stage); filling a hole the maker lists, with a patch triangulated, refined and smoothed to match the surface around it | — |
| Seams | Seam sets, seam chains, shortest-path seam drawing, [darts](../reference/glossary.md#geometry), the stuffing opening, [charts](../reference/glossary.md#geometry) cut open along seams with their sewing map, the disk and fold checks with a proposed fix (the `chart` stage) | Dart suggestions from distortion; the concave-region warning; seams imported from the model's UV map |
| Flatten | The boundary-pinned solver with an interior that cannot flip, the flip check, the not-closable fallback with a dart proposal, step-wise sessions, distortion metrics and a per-piece summary (`openplush-flatten`); whole charts laid flat in chart order, with the refusals mapped and a dart offered as a proposal (the `flatten` stage) | The stepping session that caches each chart's layout; timing in the browser |
| Pattern | Outlines and pattern pieces, per-edge [seam allowance](../reference/glossary.md#sewing), the seam-length parity check, real-world scale, the fabric type (`openplush-pattern`) | Fabric presets (waiting for measured values); notches, labels, grain and nap arrows; collapsing identical and mirror-image pieces |
| Layout | Shelf packing on a roll, as-printed and flipped copies, page tiling for Letter and A4 with overlap, registration marks and a scale square (`openplush-layout`) | Fabric-aware rotation; optimised nesting (a later release) |
| Export | True-scale SVG and tiled PDF, with the warning summary block (`openplush-export`) | Driven by the pipeline: no command or app produces a file yet |
| End to end | `import`, `validate`, `repair` and `chart` through `openplush-pipeline` | The remaining stages and their memoisation; the CLI's commands (it parses arguments and runs nothing); the end-to-end and determinism tests |
| App | `@openplush/engine-client`: the typed worker protocol, a mock worker, crash handling, and the first slice of the real worker host | The web app (`apps/web` does not exist); the WebAssembly binding exports no operation yet |

Two crates have no code: `openplush-fields` and `openplush-decal`. `openplush-project` (the project file) has only the provenance keys a saved seam or marking will be stored by; the file itself is not built.

### By feature

How far each feature's spec and plan have got, and its ticked tasks out of all the tasks in its `tasks.md`.

| Feature | Spec and plan | Tasks ticked | Where it stands |
|---|---|---|---|
| [001 pattern pipeline](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/tasks.md) | Spec draft, its blocking questions answered 2026-10-13 and three minor defaults open; planned | 101 of 189 | Setup and foundations done except the determinism and golden workflows and the golden compare, which wait for output to compare. The first user story (seams in, printable pattern out) is mostly built engine-side, as above; its pipeline wiring, CLI and web shell are not. Later stories have barely started |
| [002 seam-placement hints](https://github.com/openplush/openplush/blob/main/specs/002-seam-placement-hints/tasks.md) | Spec draft, its questions answered 2026-10-13; planned | 5 of 82 | Fixtures and two spikes (the default thresholds, symmetry) done; no code. Its crate needs 001's pipeline first |
| [003 source seam import](https://github.com/openplush/openplush/blob/main/specs/003-source-seam-import/tasks.md) | Spec draft, its questions answered 2026-10-16 and 2026-10-17; planned | 15 of 71 | Fixtures and spikes done; materials carried on the soup and the mesh, and the UV-cut tolerance set. Reading materials waits for 001's glTF import |
| [004 docs site](https://github.com/openplush/openplush/blob/main/specs/004-docs-site/tasks.md) | Spec draft, no clarification session; planned; five decisions accepted, the rest proposed | 31 of 36 | The site decision, the writing guide and page templates, the agents' docs procedure, the book build with its navigation check, links out of the book pointing at GitHub, the onboarding pages (start here, setup, this page, the [glossary](../reference/glossary.md)), the engine-concept and agents explanations, the pull-request how-to, the contributor book's landing page, `CONTRIBUTING.md` as a pointer into the book, the [first-change tutorial](../tutorials/first-change.md), the link and network checks, the site's landing page, its workflow with the determinism job, the maker guide's first pages with the plain-language check, and the facts the agent files held moved into the book (meshes, code rules, commands, review and merge) done; publishing, the API reference and one page per crate not yet |

### Known gaps

- **No real plush model with a known-good hand-made pattern** is in `fixtures/`; every model there is generated. Several success criteria, 002's headline one among them, cannot be measured until one arrives. [Contributing a test model](README.md#the-lanes) needs no code.
- **The fabric preset values are unmeasured.** They wait for someone who sews with fleece, minky and faux fur to review them.
- **Flattening speed in the browser is unmeasured**: the spike timing the solver on a large chart under WebAssembly is open.
