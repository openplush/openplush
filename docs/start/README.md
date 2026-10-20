# Start here

This page tells you what OpenPlush is, which part of the documentation is for you, and the order to read the contributor book in. Read it first; every other page assumes it.

## The short answer

OpenPlush turns a 3D model into a printable sewing pattern for a plush toy: you mark where the [seams](../reference/glossary.md#geometry) go, and it [flattens](../reference/glossary.md#geometry) each piece, adds [seam allowance](../reference/glossary.md#sewing), and prints [pattern pieces](../reference/glossary.md#sewing) at true scale. It is a Rust geometry engine, compiled both to a native command line and to WebAssembly, under a TypeScript and Three.js web app, and everything runs on the user's own device. It is free and open source (MIT OR Apache-2.0), with no accounts, uploads or telemetry.

What works today is on the [status page](status.md).

## How it works

### Who each part is for

The documentation has two books, for two readers who never share a page:

- **This book is for contributors**: anyone changing OpenPlush, in Rust, TypeScript, test models or the docs.
- **The maker guide is for makers**: people using OpenPlush to sew a plush. It needs no programming.

Inside the project, contributors work in four lanes, the Rust engine, the TypeScript web app, test models and the docs, and each needs different skills; [the lanes](https://github.com/openplush/openplush/blob/main/CONTRIBUTING.md#lanes) list them.

### The path through this book

Read in this order; each step builds on the one before.

1. **Set up**: [install the tools and run every check](setup.md), so you know your machine matches what CI runs.
2. **See where things stand**: the [project status](status.md), by capability and by feature.
3. **Learn how the engine works**: the [architecture](../explanation/architecture.md), stage by stage and why each stage's output type is its guarantee; then the [crates](../reference/crates.md) and the [engine API](../reference/engine-api.md).
4. **Pick a task**: work is planned per feature as a numbered list in `tasks.md`, starting from the [specs index](https://github.com/openplush/openplush/blob/main/specs/README.md). Pick one that is ready: the [glossary](../reference/glossary.md#process) says what makes a task ready and what each marker on its line means.
5. **Make the change**: [scope and open a pull request](../how-to/pull-requests.md), test first, one intent per pull request; for a docs change, [write a documentation page](../how-to/write-docs.md).
6. **Learn how the project is run**: the [constitution](https://github.com/openplush/openplush/blob/main/.specify/memory/constitution.md), whose principles every review enforces, [governance](https://github.com/openplush/openplush/blob/main/GOVERNANCE.md), the roles and how decisions are made, and [who merges a pull request](../adr/0014-auto-merge-tiers.md), by the risk of what it touches.
7. **Look things up**: the [reference](../reference/README.md), with the [glossary](../reference/glossary.md) for every domain term and ID prefix.
8. **Go deeper into a decision**: the [architecture decision records](../adr/README.md), each with the options weighed and why one won.

### Where to ask

Ask on [GitHub issues](https://github.com/openplush/openplush/issues/new/choose). Pick the form that fits: a wrong pattern or a crash; a docs or sewing correction, which is also the place for a page you could not follow; offering a test model; or proposing or claiming work. A question about a pull request goes in a comment on it. Every project space follows the [code of conduct](https://github.com/openplush/openplush/blob/main/CODE_OF_CONDUCT.md).
