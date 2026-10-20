# Explanation

Understanding-oriented: the reasoning behind the design, for readers who want to know why.

- [Architecture](architecture.md): the engine as built: the stage chain and its types, errors versus warnings, IDs and provenance, determinism and tolerances.
- [The pipeline, one model at a time](pipeline.md): one model followed through every stage, the crate that owns each, and what can fail there.

The reasoning behind decisions not yet built, and the alternatives considered, is in the research and Decision Ledger of [`specs/001-pattern-pipeline/`](https://github.com/openplush/openplush/tree/main/specs/001-pattern-pipeline/).
