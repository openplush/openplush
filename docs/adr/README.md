# Architecture decision records

An ADR records a decision that changes a principle or the project's shape, so the reasoning survives the people who made it. Written in [MADR](https://adr.github.io/madr/) format: copy [`0000-template.md`](0000-template.md) to the next free number.

**When an ADR is required**:
- A change to a constitution principle, or to a public contract (constitution, Development Workflow).
- A new dependency category (AGENTS.md, Licenses).
- A dependency whose license is not on the allowlist (Constitution IX, T006).
- A new sideways edge in the crate DAG (`crates/AGENTS.md`).
- An exception to `unsafe_code = "forbid"` (CONTRIBUTING.md, Warnings are errors).

Smaller decisions go in the feature plan's Decision Ledger (`specs/<feature>/plan.md`) instead.

| ADR | Title | Status |
|---|---|---|
| [0002](0002-robust-predicates.md) | Use the `robust` crate for exact geometric predicates | Accepted |
| [0003](0003-as-built-docs.md) | Once code lands, the code and `docs/` are the source of truth | Accepted |
| [0004](0004-cli-argument-parsing.md) | Use `clap` to parse the CLI's arguments | Proposed |
| [0005](0005-boundary-pinned-flattening.md) | Boundary-pinned flattening holds; the fallback stays a rare typed error and the dart signal is area-integrated compression | Proposed |

ADR 0001 is reserved for the S1 preprocessing decision (T001).
