# Contributing to OpenPlush

Thanks for helping. OpenPlush turns a 3D model into a printable sewing pattern for a plush toy, entirely on your own device. Everything a contributor needs is in the contributor book under [`docs/`](docs/README.md); this page is the short version and points into it.

## Where to help

- **Test models are the most useful contribution right now**, and need no code. A model that comes with a known-good hand-made sewing pattern is worth the most. Offer one with the [test model issue form](https://github.com/openplush/openplush/issues/new?template=fixture.yml).
- **Engine** (`crates/`): Rust; geometry helps.
- **Web app** (`apps/`, `packages/`): TypeScript and Three.js; no Rust.
- **Docs** (`docs/`): writing; sewing expertise is especially welcome.

[The lanes](docs/start/README.md#the-lanes) say more, and the [status page](docs/start/status.md) says what is built and what is not.

## Before your first pull request

1. Read [Start here](docs/start/README.md): what OpenPlush is and the path through the book.
2. [Set up and run every check](docs/start/setup.md): three commands to a passing run of every gate CI runs.
3. Pick a ready task from a feature's `tasks.md`, starting at the [specs index](specs/README.md); trivial fixes need none.
4. [Scope and open a pull request](docs/how-to/pull-requests.md): tests first, one intent per pull request, the template filled in. For a docs change, [write a documentation page](docs/how-to/write-docs.md).
5. Read the [constitution](.specify/memory/constitution.md); reviewers enforce it.

What each check enforces, such as the workspace lints, the tests that must also run on wasm32 and the dependency licences, is in [the checks reference](docs/reference/checks.md).

AI coding agents follow [`AGENTS.md`](AGENTS.md), whose rules apply to you too. You're responsible for everything you submit, whoever or whatever wrote it.

## Where to ask

Open a [GitHub issue](https://github.com/openplush/openplush/issues/new/choose) with the form that fits, or comment on the pull request. Everyone follows the [code of conduct](CODE_OF_CONDUCT.md).

## License
Contributions are dual-licensed under MIT OR Apache-2.0, as stated in the [README](README.md#license). There is no CLA.
