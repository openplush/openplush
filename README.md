# OpenPlush

Turn a 3D model into a sewable plush pattern. Free, open source, and it runs entirely on your device.

> **What is built today, and what is not:** the [project status](docs/start/status.md). What exists is described in [docs/](docs/) ([architecture](docs/explanation/architecture.md), [engine API](docs/reference/engine-api.md), [crates](docs/reference/crates.md)); what is planned is in the [specs index](specs/README.md).

## What it will do
- Import a 3D model (OBJ, glTF) and set its real-world size.
- Draw seams directly on the model.
- Flatten every piece with a stretch/compression heatmap and suggestions for darts or extra seams.
- Add seam allowances, match notches, piece labels, and grain/nap arrows for fur fabrics.
- Place markings, eyes, and other details on the pieces.
- Print true-scale, tiled PDF or SVG patterns on Letter or A4.
- Save everything in one open project file.

## Principles
- **Free forever.** MIT OR Apache-2.0. No accounts, no paywalls.
- **Private.** Your model never leaves your computer. No uploads, no telemetry.
- **Correct or loud.** A pattern that silently comes out wrong is worse than an error.

The full set is in the [constitution](.specify/memory/constitution.md).

## Design documents

Start at the [specs index](specs/README.md) — it shows what is specified and planned, plus the known holes. What is built is on the [status page](docs/start/status.md).

**001 — pattern pipeline** (the product)

| Document | Contents |
|---|---|
| [Spec](specs/001-pattern-pipeline/spec.md) | What we're building and why |
| [Research](specs/001-pattern-pipeline/research.md) | Stack decision, library audit, algorithms, fabric profiles |
| [Plan](specs/001-pattern-pipeline/plan.md) | Architecture, repo layout, decision ledger |
| [Data model](specs/001-pattern-pipeline/data-model.md) | Mesh structures, fabric, warnings, geometric edge cases |
| [Contracts](specs/001-pattern-pipeline/contracts/) | Engine API, worker protocol, project file schema |
| [Quickstart](specs/001-pattern-pipeline/quickstart.md) | The acceptance walkthrough |
| [Tasks](specs/001-pattern-pipeline/tasks.md) | 179 work items — **start here to contribute** |

**002 — seam-placement hints** (shows you where seams belong; depends on 001)

| Document | Contents |
|---|---|
| [Spec](specs/002-seam-placement-hints/spec.md) · [Plan](specs/002-seam-placement-hints/plan.md) · [Research](specs/002-seam-placement-hints/research.md) | The four detectors and why each one matters for a plush |
| [Data model](specs/002-seam-placement-hints/data-model.md) · [Tasks](specs/002-seam-placement-hints/tasks.md) | 82 work items |

## Contributing
Contributions are welcome in four lanes: the Rust geometry engine, the TypeScript web app, test models (no code needed), and docs. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
