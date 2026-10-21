# Research: Factory Hardening

Facts measured 2026-10-20 on `main` at `684ff43`.

## R1. Attribution in history (#319)

- **Measured** `[solid]`: since 2026-10-13, 760 commits authored `Claude <noreply@anthropic.com>`, 629 by the maintainer, 293 with a Claude `Co-Authored-By` trailer, 280 with a `Claude-Session` line.
- **Decision**: check forward only, on the commits a PR adds (`base..head`) and its description. History stays as is.
- **Rationale**: rewriting `main` would break every open stack and every link to a commit.
- **Alternatives**: a commit-msg hook only (bypassed by server-side commits and other harnesses); a check over all of `main` (fails forever on history).

## R2. Where the check sees the PR (#319)

- **Decision**: a step in a workflow triggered by `pull_request`, reading `github.event.pull_request.base.sha`, `head.sha` and `body`. The body comes through an environment variable, never interpolated into the script.
- **Rationale**: `pull_request` events carry the range and the body without extra API calls or write tokens; fork PRs work.
- **Alternatives**: `pull_request_target` (gives secrets to fork code; ruled out by the repo's workflow rules).

## R3. Cross-platform gate diagnostic (#322)

- **Decision**: Python 3 with `gh api` subprocess calls, read-only.
- **Rationale**: Bash and `grep` steps did not run in a PowerShell session (#322); `scripts/merge-gate.py` already sets Python as the gate's language; `gh` handles auth on every platform.
- **Alternatives**: a PowerShell twin (two copies drift); a docs page of manual steps (the status quo that took 30 minutes).

## R4. Already handled in the gate (#322)

- `[solid]` `scripts/merge-gate.py` reads the review summary's head (`SUMMARY_HEAD`) and treats a summary for another head as stale, asking for one review per head; a reviewer's `CHANGES_REQUESTED` blocks (ADR 0018). `scripts/tests/merge-gate.test.sh` covers both ("a summary for another head is not ready and asks for a review", "the request is marked in the comment, so it is made once per head", and the changes-requested cases).

## R5. Placeholders in kickoffs (#317)

- **Decision**: a placeholder is an angle-bracketed word (`<window>`) or angle brackets containing a pipe (`<accepted | change it>`). The first one stops the firing with its text.
- **Rationale**: both observed stalls used these two shapes; a false stop costs one message, a false start hours.
- **Alternatives**: an allow-list of fields per kickoff template (needs a template registry nobody maintains).

## R6. Ledger conflicts (#320)

- **Measured** `[solid]` on `main` at `e9845ce`: the last 50 merged PRs (#255–#324). For each, every merge commit inside its branch (a merge-up of the parent or `main`) was replayed with `git merge-tree --write-tree`, which reports the files that conflict.
  - 28 of the 50 carried at least one merge-up; 16 of those had a conflicting one (20 conflicting merges).
  - Files, counted once per PR: a `specs/*/tasks.md` in 9 (ticks and as-built notes on neighbouring lines); an index list in 7 (`docs/SUMMARY.md`, `docs/explanation/README.md`, `docs/start/README.md`, `docs/adr/README.md`, `docs/reference/crates.md`); a Decision Ledger row in `specs/*/plan.md` in 2 (#267, #300, both two rows added after the same last row); other content in 7.
  - Limit: a PR whose author resolved a conflict by rebasing instead of merging leaves no merge commit and is not counted, so these are lower bounds.
- **Decision**: proposed in plan.md, F-003.
- **Alternatives**: one file per decision with a generated table (removes ledger conflicts, adds a generator and a file per row); an append-only section (still conflicts when two branches append after the same row).
