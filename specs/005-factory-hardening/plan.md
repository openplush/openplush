# Implementation Plan: Factory Hardening

**Branch**: `005-factory-hardening` | **Date**: 2026-10-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/005-factory-hardening/spec.md`

## Summary

Each process rule the factory broke moves to one checked home: the factory skill refuses an unfilled kickoff and ends every firing with a handoff; `AGENTS.md` owns commit identity and a CI script enforces it on each PR's own commits and body; a read-only diagnostic checks the merge gate's prerequisites; the gate's comment names its rehearsal mode; `steward` reports stalled checks once; `AGENTS.md` gives decision prompts one shape with a glossary behind it. Grants (US6) and the ledger layout (US7) are maintainer decisions first: this plan drafted them (F-002, F-003), and the maintainer accepted both on 2026-10-21.

## Technical Context

| Field | Value |
|---|---|
| Language | Bash for checks that follow `scripts/check-*.sh`; Python 3.11+ where the same script must run on Windows (`scripts/merge-gate.py` is the precedent) |
| Dependencies | None new. `gh` for the diagnostic's API reads, `git`, `jq`, `python3`, all already prerequisites in `scripts/check.sh` |
| Testing | `scripts/tests/*.test.sh`, run by `scripts/check.sh`'s `boundary` step; skills have no harness, so a skill rule is shown by a worked example in its PR |
| Target | GitHub Actions runners (Linux) for CI; contributor machines on Linux, macOS and Windows for the diagnostic |
| Constraints | No settings changed by any script (FR-009); no history rewritten (FR-008); every new gate passes on `main` when it lands |
| Scope | One skill, one agent-rules section, one glossary page, three scripts with tests, one workflow step, one ADR draft |

## Constitution Check

| Principle | Status |
|---|---|
| I Engine/Shell separation | ✅ Nothing under `crates/`, `apps/` or `packages/` changes |
| II CLI parity | ✅ Not applicable |
| III No silent wrong output | ✅ The attribution check and the diagnostic fail or report loudly; the diagnostic reports a setting it cannot read as unknown, never as present |
| IV Test-first | ✅ Each script lands after its failing test |
| V Determinism | ✅ Script output sorted, no timestamps |
| VI Units | ✅ Not applicable |
| VII Explicit repair | ✅ The diagnostic prints fixes and never applies them |
| VIII Local-first, no telemetry | ✅ The diagnostic calls the GitHub API only when a person runs it; nothing at app runtime changes |
| IX Licences | ✅ No new dependency |
| X Baseline before cleverness | ✅ Prose rules and small scripts first; the ledger stays hand-written (F-003); only folder index lists are generated |

## Project Structure

### Documentation (this feature)

```text
specs/005-factory-hardening/
├── plan.md          # this file
├── research.md      # Phase 0
├── data-model.md    # grant, handoff, decision prompt, gate prerequisite
├── quickstart.md    # how to see each story working
├── contracts/
│   ├── factory-arguments.md   # kickoff, placeholder rule, grant fields
│   └── handoff.md             # the handoff's shape
└── tasks.md         # /speckit-tasks
```

### Source (repository root)

```text
.claude/skills/factory/SKILL.md      # US1, US2, later US6
.claude/skills/steward/SKILL.md      # US4: stalled checks
AGENTS.md                            # US3: identity line; US5: decision prompts
docs/reference/glossary.md           # US5 (004's W043)
docs/adr/0020-factory-grants.md      # US6 draft
scripts/check-attribution.sh         # US3, with scripts/tests/check-attribution.test.sh
scripts/gate-doctor.py               # US4, with scripts/tests/gate-doctor.test.sh
scripts/merge-gate.py                # US4: the dry-run line
.github/workflows/                   # US3: where the attribution check sees the PR range and body
```

**Structure Decision**: no new directory; every change lands beside the file it hardens.

## Thresholds

| Name | Value | Why |
|---|---|---|
| Session context stop (F-004) | 300k tokens | Sessions at 273k–366k were the ones that drifted (#321); 300k leaves room for one more step and the handoff |
| Check not started | 15 min after the head was pushed | Runners here start within a few minutes; 15 is well past normal queueing |
| Check not finished | 60 min after it started | The slowest job (`wasm`) finishes in under 30 min |

## Phasing

| Phase | What | Gate |
|---|---|---|
| 1 | US1, US2 (factory skill), US3 (attribution), US4 (gate setup), US5 (decision prompts) | Maintainer merges (tier H) |
| 2 | US6 grant: ADR 0020 accepted, then the skill and triggers | F-002 accepted 2026-10-21 |
| 3 | US7 ledger layout | F-003 accepted 2026-10-21; F013 follows 004's W044 |

## Decision Ledger

| Date | ID | Decision | Status |
|---|---|---|---|
| 2026-10-20 | F-001 | **`AGENTS.md` is the one source for commit identity and attribution** ([#319](https://github.com/openplush/openplush/issues/319)): commits carry the contributor's own identity and no agent attribution (author, co-author, session line, generated-by footer). Hooks and session prompts defer to it, and a CI check on each PR's own commits and body enforces it. Forward-only: history is never rewritten | Accepted (standing authorization, 2026-10-21; grant on the 005 ledger #345) |
| 2026-10-20 | F-002 | **A grant is a structured `/factory` argument** ([#316](https://github.com/openplush/openplush/issues/316)): hours, features, merge yes/no, paths, decisions yes/no, and `continue` to renew the last one. What a grant may override relative to [ADR 0018](../../docs/adr/0018-hands-off-merging.md) is set by ADR 0020; scheduled triggers expire with the grant. `human:decision` | Accepted (maintainer, 2026-10-21): option 2, merge on the gate's verdict; recorded in ADR 0020 |
| 2026-10-20 | F-003 | **Ledger layout** ([#320](https://github.com/openplush/openplush/issues/320)). Measured (research R6): of the last 50 merged PRs, 28 merged a parent or `main` into their branch and 16 of those hit a conflict; the Decision Ledger was in 2, a `tasks.md` in 9, an index list (`docs/SUMMARY.md`, a folder `README.md`, `docs/adr/README.md`, `docs/reference/crates.md`) in 7. Options: (a) one file per decision with a generated ledger table; (b) an append-only ledger section; (c) the ledger as is. **Recommendation: (c) for the ledger**, since it caused 2 of 16 and (a) adds a generator and a file per row to every feature; (b) still conflicts when two branches append after the same last row. Put the effort where the conflicts are: generated folder index lists (004's W-018) and `tasks.md` as-built notes. Default if unanswered: (c). `human:decision` | Accepted (maintainer, 2026-10-21): (c), the ledger as is; folder index lists are generated instead (004's W-018, W044) |
| 2026-10-20 | F-004 | **A factory session stops after one firing, or at the next step boundary past 300k tokens of context**, with the reason and the handoff ([#321](https://github.com/openplush/openplush/issues/321)) | Accepted (standing authorization, 2026-10-21; grant on the 005 ledger #345) |

## Complexity Tracking

None: no Constitution violation.
