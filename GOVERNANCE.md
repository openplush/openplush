# Governance

How decisions are made in OpenPlush, and who makes them. Pre-alpha: this is deliberately small and will grow with the contributor base.

## Roles

| Role | Who | Can |
|---|---|---|
| Maintainer | @lvndrpup | Merge to `main`, change specs and plans, approve ADRs, amend the constitution, enforce the code of conduct |
| Lane reviewer | none yet | Review and approve PRs in one lane (engine, shell, fixtures, docs); listed in `.github/CODEOWNERS` |
| Contributor | anyone | Open issues and PRs in any lane |

A contributor becomes a lane reviewer after several merged PRs in that lane, on a maintainer's invitation, recorded by adding them to `CODEOWNERS`.

## How decisions are made

From smallest to largest:

1. **Code and docs** — a PR, reviewed against [`AGENTS.md`](AGENTS.md) and the constitution. One reviewable slice per PR (the test task plus the implementation task(s) that make it pass), test first, merge commits only. Detail: [docs/how-to/pull-requests.md](docs/how-to/pull-requests.md).
2. **What gets built** — a spec, plan, and tasks under `specs/<feature>/`, written with [Spec Kit](https://github.com/github/spec-kit). Every non-trivial change maps to a task ID.
3. **Design decisions inside a feature** — an entry in that feature's Decision Ledger (`plan.md`), with an absolute date and a reason.
4. **Principle or public-contract changes, new dependency categories, non-allowlist licenses, sideways crate edges, `unsafe` exceptions** — an ADR in [`docs/adr/`](docs/adr/). The full list is in the [ADR README](docs/adr/README.md).
5. **The constitution** ([`.specify/memory/constitution.md`](.specify/memory/constitution.md)) — amended as its own Governance section says: one PR containing the change, an ADR, and a version bump (MAJOR, MINOR, or PATCH by the kind of change), ratified by maintainer approval. It binds everything above it: a PR, spec, or ADR that conflicts with it is changed, not the reverse.

When people disagree, the tie-breaker is evidence: a failing test, a measurement against the fixture corpus, or a spike result beats an argument. Without evidence, the maintainer decides and records why.

## Labels

Lanes and work types are labels, defined in [`.github/labels.json`](.github/labels.json). GitHub doesn't read that file; a maintainer applies it with `scripts/sync-labels.sh`.

## Code of conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Maintainers enforce it.
