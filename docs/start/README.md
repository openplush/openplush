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

### The lanes

Contributors work in four lanes, each with its own folder and skills:

| Lane | Folder | You need |
|---|---|---|
| Engine | `crates/` | Rust; geometry helps |
| Web app | `apps/`, `packages/` | TypeScript and Three.js; no Rust |
| Test models | `fixtures/` | 3D modelling or sewing; no code |
| Docs | `docs/` | Writing; sewing expertise is especially welcome |

The rules for the two code lanes are in [`crates/AGENTS.md`](https://github.com/openplush/openplush/blob/main/crates/AGENTS.md) and [`apps/AGENTS.md`](https://github.com/openplush/openplush/blob/main/apps/AGENTS.md). They are written for AI coding agents and apply to people too. The reasons behind the engine's rules are in [Code rules for the engine](../reference/code-rules.md).

A test model that arrives with a **known-good hand-made sewing pattern** is worth more than any single code change: the engine's output can be checked against it, and it is the only way to check that seam hints land where experienced makers actually cut. Offer one with the [test model issue form](https://github.com/openplush/openplush/issues/new?template=fixture.yml); the [known gaps](status.md#known-gaps) say what the models in `fixtures/` lack.

### The path through this book

Read in this order; each step builds on the one before.

1. **Set up**: [install the tools and run every check](setup.md), so you know your machine matches what CI runs.
2. **See where things stand**: the [project status](status.md), by capability and by feature.
3. **Learn how the engine works**: the [architecture](../explanation/architecture.md), stage by stage and why each stage's output type is its guarantee; then the [crates](../reference/crates/README.md) and the [engine API](../reference/engine-api.md). [Feature 001's quickstart](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/quickstart.md) walks one model through the whole pipeline as planned.
4. **Make a first change**: [your first engine change, test first](../tutorials/first-change.md), an exercise you throw away, so a real task's steps are familiar.
5. **Pick a task**: work is planned per feature as a numbered list in `tasks.md`, starting from the [specs index](https://github.com/openplush/openplush/blob/main/specs/README.md). Pick one that is ready: the [glossary](../reference/glossary.md#process) says what makes a task ready and what each marker on its line means. A docs or typo fix and a few other trivial changes need no task; [the workflow rules](https://github.com/openplush/openplush/blob/main/AGENTS.md#workflow) list them.
6. **Make the change**: [scope and open a pull request](../how-to/pull-requests.md), test first, one intent per pull request; for a docs change, [write a documentation page](../how-to/write-docs.md).
7. **Learn how the project is run**: the [constitution](https://github.com/openplush/openplush/blob/main/CONSTITUTION.md), whose principles every review enforces, [governance](https://github.com/openplush/openplush/blob/main/GOVERNANCE.md), the roles and how decisions are made, and [who merges a pull request](../adr/0014-auto-merge-tiers.md), by the risk of what it touches.
8. **Look things up**: the [reference](../reference/README.md), with the [glossary](../reference/glossary.md) for every domain term and ID prefix.
9. **Go deeper into a decision**: the [architecture decision records](../adr/README.md), each with the options weighed and why one won.

### Where to ask

Ask on [GitHub issues](https://github.com/openplush/openplush/issues/new/choose). Pick the form that fits: a wrong pattern or a crash; a docs or sewing correction, which is also the place for a page you could not follow; offering a test model; or proposing or claiming work. A question about a pull request goes in a comment on it. Every project space follows the [code of conduct](https://github.com/openplush/openplush/blob/main/CODE_OF_CONDUCT.md).
