---
status: Accepted
date: 2026-10-14
deciders: maintainer, on mentor feedback (reviewed in the PR that adds this ADR)
---

# Once code lands, the code and `docs/` are the source of truth

## Context and problem statement

`docs/` redirected readers to `specs/`, so agents and contributors learned the system from plans written before the code. The plans had drifted: implementing T028–T031 took three corrections to `tasks.md` (paths, a narrowed task, a test that could no longer compile as written), and the engine API contract still says "signatures here are the contract" for sections the code has since settled. The question: when a spec and the built system disagree, which one is authoritative, and where does a reader find what exists?

## Decision drivers

* A reader, human or agent, must be able to trust the page they land on (Constitution III: plausible but wrong output is the worst outcome, and that includes wrong docs that produce wrong code).
* One home per fact (AGENTS.md, DRY): no two live documents describing one API.
* Docs must stay current without a separate effort; docs written at the end drift like plans do.
* Spec Kit's specs remain the record of intent and decisions.

## Considered options

* **A. Specs stay authoritative.** Contracts are kept live; `docs/` points to them.
* **B. Both live.** `docs/` documents the built system and the contracts are kept in sync with it.
* **C. Code and `docs/` become authoritative once built.** A built spec section points to its as-built docs and freezes as the record of intent.

## Decision outcome

Chosen option: **C**, because it gives each fact exactly one home at any time: the spec until the code exists, then the code (rustdoc) plus `docs/` for what rustdoc cannot show. [likely] This is the usual docs-as-code arrangement; the project-specific part is retiring contract sections one at a time.

The rules:

1. **Docs ship with the change.** A PR that changes public behaviour updates its as-built docs in the same PR; the triggers are listed once, in AGENTS.md ("Specs and docs"). `docs/` prose does not count against the 400-line PR budget, so the budget never pushes docs out.
2. **Built sections freeze.** When a contract or data-model section is fully built, it gets an "As built" pointer to its rustdoc and `docs/` page and stops changing. It stays as the record of what was intended and why.
3. **Later changes edit the code and its docs.** Changing built public behaviour is a change to the code and its as-built docs together. A public-contract change still needs an ADR (constitution, Development Workflow); other decisions go in the feature's Decision Ledger.
4. **Specs for unbuilt parts stay authoritative** until those parts land.

### Consequences

* Good, because a reader lands on a page that describes what exists, and the code can't silently disagree with the reference: rustdoc is generated from it, and error and warning strings are pinned by tests.
* Good, because docs are written in small pieces by whoever made the change, while they still remember it.
* Bad, because a reader who wants the full picture of a part-built feature reads two places, the as-built docs and the spec for the unbuilt rest. Each spec section says which applies.
* Bad, because existing spec tasks that wrote docs at the end (001's T168, 002's H079) and one that treats the contract as authoritative (002's H034) need rewording.

### Confirmation

The `docs-reviewer` agent (`.claude/agents/docs-reviewer.md`) runs in the review workflow on any PR touching a public surface or `docs/`. It flags a public change without docs, or docs that contradict the code, as blocking.

## Pros and cons of the options

### A. Specs stay authoritative

* Good, because it needs no new process.
* Bad, because the contract has to be kept in sync with the code by hand, and the drift that prompted this ADR shows that doesn't happen.

### B. Both live

* Good, because readers can read either.
* Bad, because two live descriptions of one API, with no rule for which wins.

## More information

001 adopts this in its Decision Ledger (D-023): contract sections 1 and 5 are the first to freeze. Related: AGENTS.md "Specs and docs"; `docs/reference/engine-api.md`.
