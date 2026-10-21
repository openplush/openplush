# Architecture decision records

An ADR records a decision that changes a principle or the project's shape, so the reasoning survives the people who made it. Written in [MADR](https://adr.github.io/madr/) format: copy [`0000-template.md`](0000-template.md) to the next free number.

**When an ADR is required**:
- A change to a constitution principle, or to a public contract (constitution, Development Workflow).
- A new dependency category (AGENTS.md, Licenses).
- A dependency whose license is not on the allowlist (Constitution IX, T006).
- A new sideways edge in the crate DAG (`crates/AGENTS.md`).
- An exception to `unsafe_code = "forbid"` ([the workspace lints](../reference/checks.md#workspace-lints-clippy)).

Smaller decisions go in the feature plan's Decision Ledger (`specs/<feature>/plan.md`) instead.

Every ADR, from its H1 and the status in its front matter ([generated](../reference/checks.md#generated-index-lists-docs) when the book is built):

| ADR | Title | Status |
|---|---|---|
{{#include index.inc}}

## How to read an ADR

Every ADR follows the [template](0000-template.md), so its parts are always in the same place:

- **Front matter**: the status, the date it was written, and who decided.
- **Context and problem statement**: what forced the decision, with links to the spec, plan section, task or PR that raised it.
- **Decision drivers**: the constitution principles and other forces the options were judged against.
- **Considered options** and **Pros and cons of the options**: everything that was on the table, and why each won or lost.
- **Decision outcome**: the chosen option and the reason. Read this first if you only want the answer. Its **Consequences** list what the decision costs as well as what it buys, and **Confirmation** names the gate, test or spike that shows it still holds.
- **More information**: follow-up work and when to revisit.

**Status** says how far to rely on it:

| Status | Meaning |
|---|---|
| Proposed | Written and argued, not yet accepted by the maintainer. Work may follow it, but it can still change. |
| Accepted | Binding. |
| Rejected | Considered and turned down; kept so the question isn't reopened without new facts. |
| Deprecated | No longer applies, with nothing replacing it. |
| Superseded by ADR-NNNN | Replaced; read the newer ADR. |

Once accepted, an ADR is a record of what was decided and why on its date: its body is not edited, and a change is a new ADR that supersedes it. It never describes the code as it is now: once code lands, the code and `docs/` are the source of truth ([ADR 0003](0003-as-built-docs.md)). To follow a decision to its work:

- **Tasks**: the feature's `tasks.md` (`specs/<feature>/tasks.md`) cites the ADR on the tasks that implement or depend on it, and the plan's Decision Ledger records the smaller decisions made along the way.
- **Code**: the as-built docs describe what exists, starting from the [architecture](../explanation/architecture.md) and the [crate reference](../reference/crates/README.md); the ADR's Context and Confirmation name the crates, workflows or gates it touches.
- **History**: the PR that merged the ADR, and the PRs that cite it, carry the discussion.
