# How to get a pull request reviewed and merged

<!-- index: the review, the merge gate's conditions, and which tier merges how. -->

This page takes an open pull request through the automated review and the merge gate to `main`. Use it once your PR is open; how to scope and open one is in [Scope and open a pull request](pull-requests.md).

## You will

Have your PR merged, by the merge gate or by the maintainer, depending on its tier.

## Before you start

- A PR opened from the template, with CI running ([Scope and open a pull request](pull-requests.md)).
- What the review and the gate are for, and why they work this way: [How agents work](../explanation/agents.md#the-merge-gate-tiers-and-who-merges).

## Steps

1. **Wait for the review summary.** The review runs when the PR opens or is marked ready for review, except on a `docs(tasks): tick …` PR, a PR from a `chore/` branch, and a PR that changes only files under `docs/`; the gate's comment then says the review was skipped by its rules. It posts a comment that starts `Review summary (PR N, head SHA)` and review comments on the lines it found something on, each marked 🔴 blocking, 🟡 optional or 🟣 pre-existing.
2. **Answer every 🔴.** Fix it, with a test that fails before the fix and passes after, or reply on its thread saying why it stands. Answer the 🟡 findings in one comment on the PR, saying which you fixed and why the rest stay. A 🟣 finding was there before your PR and needs nothing.
3. **Resolve each thread you answered.** The gate counts an unresolved thread as an unanswered finding.
4. **Push your fixes in one go.** After a push the summary is for an older head, so the gate asks for a new review itself, once per head; you never add the `rereview` label. Only the maintainer adds it by hand, to ask for another run, or adds `mutation-audit` for a run that also checks that the tests catch deliberate bugs. A rerun reviews only what changed since the last summary. A push also removes `merge:ready`, and the gate judges the new head when its CI finishes.
5. **Read the gate's comment.** It starts `Merge gate` and lists each condition with ✅ or ❌: not a draft, head in this repository, base is `main`, no conflict, the required checks green on this head, a review summary for this head with no 🔴, no unresolved thread, no changes requested by a reviewer, and the tier. Fix whatever has ❌. For a stacked PR, `base is main` turns ✅ when the PR below it merges.
6. **Let the tier decide who merges.** The gate computes the tier from the diff: a PR's tier is the highest any of its paths or tasks reaches, and the gate can raise a tier label but never lower one.

### Tiers

| Tier | When | Who merges |
|---|---|---|
| H | `.github/`, `scripts/`, `.claude/`, `docs/adr/`, `.specify/`; any `AGENTS.md`, `CLAUDE.md`, `GOVERNANCE.md`, `CODE_OF_CONDUCT.md`; `deny.toml`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.gitattributes`, `LICENSE-*`; a ticked `human:` task; labelled `tier:H` by the maintainer | The maintainer, or a factory run whose grant allows it ([ADR 0020](../adr/0020-factory-grants.md)) |
| C | `crates/`, `bindings/`, `specs/`; any `Cargo.toml`, `Cargo.lock`, `package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `.cargo/`; a Decision Ledger entry | The gate, with auto-merge once ready |
| B | `fixtures/golden/`, `*.snap`, or an expected SVG or PDF fixture | The gate, with auto-merge 24 h after the head's CI went green |
| A | Everything else, and a `docs(tasks): tick …` PR whose diff touches only `specs/*/tasks.md` with ticks and as-built notes | The gate, with auto-merge once ready |

"Ready" is every other condition in step 5 passing. The gate then labels the PR `merge:ready` and turns on auto-merge, with a merge commit. Tier H never gets the label; the comment says the maintainer merges it, and so may a factory run under a grant that allows merging, on this comment's verdict for every condition but the tier ([ADR 0020](../adr/0020-factory-grants.md)).

## Check it worked

The PR shows as merged into `main`. Until then, the gate's comment says what it is waiting for: a ❌ condition to fix, the tier B hold, or the maintainer for tier H.
