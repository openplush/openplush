# Explanation

Understanding-oriented: the reasoning behind the design, for readers who want to know why.

- [Architecture](architecture.md): the engine as built: the stage chain and its types, errors versus warnings, IDs and provenance, determinism and tolerances.
- [The pipeline, one model at a time](pipeline.md): one model followed through every stage, the crate that owns each, and what can fail there.
- [Seams, charts, darts and openings](seams.md): how seams cut a mesh into charts, what darts and openings are, and why a chart must be a disk.
- [Flattening and exact seam lengths](flattening.md): what flattening does to a chart, and why every seam edge keeps its exact 3D length.
- [Sewing concepts for programmers](sewing.md): seam allowance, grain, nap and notches, what each means for a pattern and where it lives in the engine.
- [How agents work](agents.md): how AI agents take a feature from spec to merged pull request: Spec Kit, the factory, steward, the review on opening, the merge gate and standing authorizations.

The reasoning behind decisions not yet built, and the alternatives considered, is in the research and Decision Ledger of [`specs/001-pattern-pipeline/`](https://github.com/openplush/openplush/tree/main/specs/001-pattern-pipeline/).
