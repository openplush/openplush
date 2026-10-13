# OpenPlush

Turn a 3D model into a sewable plush pattern. Free, open source, and it runs entirely on your device.

> **Status: pre-alpha, specification phase.** There is no usable app yet. The design is being worked out in [`specs/`](specs/).

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
| Document | Contents |
|---|---|
| [Spec](specs/001-pattern-pipeline/spec.md) | What we're building and why |
| [Research](specs/001-pattern-pipeline/research.md) | Stack decision, library audit, algorithms |
| [Plan](specs/001-pattern-pipeline/plan.md) | Architecture, repo layout, decision ledger |
| [Data model](specs/001-pattern-pipeline/data-model.md) | Mesh structures and geometric edge cases |

## Contributing
Contributions are welcome in four lanes: the Rust geometry engine, the TypeScript web app, test models (no code needed), and docs. See [CONTRIBUTING.md](CONTRIBUTING.md).

## License
Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
