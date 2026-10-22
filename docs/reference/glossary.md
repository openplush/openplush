# Glossary

<!-- index: every domain term (geometry, sewing, process) and every ID prefix the specs use. -->

Every domain term and ID prefix the contributor pages use, as this repository uses it: geometry from the engine's code and specs, sewing from feature 001's spec, process from the agent files, the skills and the ADRs.

## Entries

Each section is in alphabetical order.

### Geometry

- **Boundary loop**: a closed chain of edges that each border only one face; the rim of a hole in the surface. Repair treats every boundary loop as an [opening](#geometry) unless the maker lists it as an accidental hole to fill. A loop is never filled without being asked.
- **Chart**: a connected patch of the surface, bounded by seams, that flattens into one pattern piece. A chart must be a topological disk (connected, one boundary loop, Euler characteristic 1) and must not fold back over itself, or it cannot lie flat as one piece. Darts are slits inside a chart, not boundaries between charts.
- **Dart**: a seam that runs from a seam or the surface's edge to a tip inside a piece. Sewn shut, it takes in fabric so a flat piece can cup into a curve. The engine may propose a dart where a piece cannot lie flat; a proposal changes nothing until the maker accepts it.
- **Developable**: a surface that unrolls flat without stretching, such as a cylinder or a cone. It flattens with almost zero distortion, so tests use developable shapes as an oracle for the flattener.
- **Distortion**: how much flattening stretches or compresses each triangle, measured as the two singular values of the map from the triangle's 3D shape to its flat one: the largest is the stretch (above 1 stretches), the smallest the compression (below 1 compresses), and their product the change in area. The fabric decides how much is too much.
- **Flattening**, or **parameterization**: giving each vertex of a chart a 2D position, so the chart lies flat as a pattern piece. OpenPlush fixes the chart's boundary first, so both sides of every seam keep exactly equal lengths, then solves the interior so no triangle flips over ([ADR 0005](../adr/0005-boundary-pinned-flattening.md)).
- **Half-edge**: one direction of an edge, belonging to the face on its left, or to no face on the surface's edge. Each half-edge knows its twin, the same edge in the other direction, so walking around a face or a vertex is a series of lookups. The repaired mesh, `ManifoldMesh`, is stored as half-edges.
- **Manifold**: a mesh in which every edge borders one or two faces and the faces around each vertex form a single fan, so the surface is well defined everywhere. Repair must produce one before seams can be cut or anything flattened.
- **Mesh**: a surface made of triangles that share vertices and edges. A model is imported as a triangle soup, triangles whose connections are not yet checked, and becomes a manifold half-edge mesh after repair.
- **Opening**: a seam left unsewn, for turning the plush right side out and stuffing it.
- **Seam**: a path of edges on the model where two pieces of fabric are sewn together, which is also where the surface is cut into charts. The seam set holds every seam edge, each marked as a cut, a [dart](#geometry) or an [opening](#geometry).

### Sewing

- **Grain**: the direction of the threads in a woven fabric, or of the rows in a knit. Fabric stretches differently along the grain, across it, and on the bias (the diagonal), so each piece carries a grain arrow the sewist lines up with the fabric's edge.
- **Nap**: fur or pile that lies in one direction, as on faux fur and minky; stroked the other way, it looks and feels different. Every piece cut from a napped fabric must have its nap running the same way on the finished plush, so a napped piece keeps one orientation when it is laid out.
- **Notch**: a small mark on a piece's edge that meets a matching mark on the piece it is sewn to, showing which edges go together and where they line up. Matching notches sit at equal distances along both sides of a seam.
- **Pattern piece**: one flat shape to cut from fabric, made from one chart: its sewing line, the seam allowance around it, and its labels, such as how many to cut as printed and how many flipped.
- **Seam allowance**: the strip of fabric between the sewing line and the cut edge, which the stitches hold on to. OpenPlush sets it per edge, in millimetres.

### Process

- **ADR**: an architecture decision record in `docs/adr/`: one decision, the options weighed, and why one won. A change to a principle, a new dependency category or a licence outside the allowlist needs one ([the ADR index](../adr/README.md)).
- **Constitution**: the binding principles, numbered I to X, that every spec, plan and review defers to ([the constitution](https://github.com/openplush/openplush/blob/main/.specify/memory/constitution.md)).
- **Decision ledger**: the table in each feature's `plan.md` that lists the feature's decisions, each with its date, ID, what was decided, and its status (proposed, or accepted by the maintainer).
- **Factory**: the unattended task factory. Each run, a *firing*, starts as `/factory <feature>`, takes the next ready [tasks](#process) of a feature, opens a stacked pull request for each, and stops at a named condition instead of asking a question ([the factory skill](https://github.com/openplush/openplush/blob/main/.claude/skills/factory/SKILL.md)).
- **Fixture**: a test model or data file under `fixtures/`, such as a mesh, a model with exactly one repair defect, or a reference seam set. A *golden* fixture is a stage's expected output, and a pull request that changes one is [tier B](../how-to/review-and-merge.md#tiers). What each folder holds, and how the files are generated: [the fixtures README](https://github.com/openplush/openplush/blob/main/fixtures/README.md).
- **Grant**, or **standing authorization**: a time-boxed permission the maintainer gives a [factory](#process) run as one argument (hours, features, merge, paths, decisions); one that allows merging lets the run merge a [tier H](#process) pull request the merge gate shows ready but for its tier ([Standing authorizations](../explanation/agents.md#standing-authorizations), [ADR 0020](../adr/0020-factory-grants.md)).
- **Plan**: a feature's `plan.md`: how it will be built, with the architecture, the technology choices, the decision ledger and the order of work.
- **Review finding**: one comment from the automated review, marked 🔴 blocking (fixed, or answered with why it stands), 🟡 optional, or 🟣 pre-existing (there before the pull request, needing nothing) ([Get a pull request reviewed and merged](../how-to/review-and-merge.md)).
- **Spec**: a feature's `spec.md`: what the feature does and why, as user stories, numbered requirements and success criteria, with no technology choices.
- **Stack position**: the `NN` in a branch named `NNN/NN-slug`: a pull request's place in its feature's sequence, assigned when the pull request opens and never reused ([Name the branch and stack it](../how-to/pull-requests.md#3-name-the-branch-and-stack-it)).
- **Steward**: the procedure an agent follows to drive a pull request through CI, review findings, stacks and merges ([the steward skill](https://github.com/openplush/openplush/blob/main/.claude/skills/steward/SKILL.md)).
- **Task**: one numbered line in a feature's `tasks.md`: a piece of work, the files it touches, and a tick when it is done. Markers on the line: `[P]` can run in parallel with its neighbours; `[US1]` names the user story it serves; `needs:` lists tasks that must be ticked first; `blocked:` names a proposed decision only the maintainer can clear; `human:` means a person must do it, and no agent picks it up. A task is **ready** when it is unticked, has no `blocked:` or `human:` token, every task in its `needs:` is ticked, and the phases before it are done.
- **Tier**: who merges a pull request, set by the riskiest path or task it touches: **A**, **B** and **C** merge through the merge gate, **H** by the maintainer or under a grant. Which paths fall in each, and the gate's conditions, are in [Get a pull request reviewed and merged](../how-to/review-and-merge.md#tiers) ([ADR 0014](../adr/0014-auto-merge-tiers.md), [ADR 0015](../adr/0015-merge-gate.md), [ADR 0018](../adr/0018-hands-off-merging.md)).
- **Tier H**: the [tier](#process) the maintainer merges, or a factory run under a [grant](#process) that allows it ([which paths](../how-to/review-and-merge.md#tiers), [why](../explanation/agents.md#the-merge-gate-tiers-and-who-merges)).

### ID prefixes

[ADR 0021](../adr/0021-one-id-scheme.md) sets one scheme for every feature: tasks are `T`, decisions `D-`, and a spike is named by its task. The table lists the prefixes in use, each feature with its own letter. Spec IDs (`FR-`, `SC-`, `US`, `P`) restart in every feature: say which feature, as in "001 FR-023". A number in brackets after an ID names one numbered item inside it: D-066 (9) is item 9 of decision D-066.

| Prefix | Example | What it names | Feature | Defined in |
|---|---|---|---|---|
| `#` | #241 | A pull request or issue | All | GitHub |
| `ADR` | ADR 0013 | An architecture decision record | All | `docs/adr/` |
| `D-` | D-013 | A decision | 001 | `plan.md`, Decision Ledger |
| `FR-` | FR-023 | A functional requirement | Each | `spec.md` |
| `H` | H012 | A task | 002 | `tasks.md` |
| `H` (one digit) | H2 | A spike: a timeboxed experiment that answers one question | 002 | `tasks.md`, `research.md` |
| `H-` | H-003 | A decision | 002 | `plan.md`, Decision Ledger |
| `P` | P1 | A user story's priority, P1 highest | Each | `spec.md` |
| `Q` | Q2 | A clarification question and its answer | 001, 002 | `spec.md`, Clarifications |
| `Q` | Q1 | A spike | 003 | `research.md`, `tasks.md` |
| `R` | R7 | A numbered research section | Each | `research.md` |
| `S` | S7 | A spike | 001 | `tasks.md`, `research.md` |
| `S-` | S-014 | A decision | 003 | `plan.md`, Decision Ledger |
| `SC-` | SC-005 | A measurable success criterion | Each | `spec.md` |
| `T` | T079 | A task | 001 | `tasks.md` |
| `U` | U042 | A task | 003 | `tasks.md` |
| `US` | US1 | A user story | Each | `spec.md` |
| `W` | W018 | A task | 004 | `tasks.md` |
| `W-` | W-004 | A decision | 004 | `plan.md`, Decision Ledger |
| `§` | data-model §3 | A numbered section of a document | Each | `data-model.md`, the contracts |
| Roman numeral | Constitution IX | A principle | All | The constitution |
