---
description: "Task list for 005 factory hardening"
---

# Tasks: Factory Hardening

**Input**: Design documents in `/specs/005-factory-hardening/`
**Prerequisites**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)
**Generated**: 2026-10-20

**Tests first where there is code**: every script under `scripts/` lands after a failing test under `scripts/tests/`, as `AGENTS.md` requires. Skills and agent rules have no test harness; their PR gives a worked example instead and says so.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: which user story it serves (US1 to US7 in spec.md)
- `needs:ID[,ID]`: unticked tasks this one waits on, where the phase order alone does not say so
- `blocked:F-0NN`: a proposed decision this one waits on; only the maintainer clears it
- `human:XX`: needs a person; no agent picks it up
- Task IDs are `F0NN`, distinct from 001's `T`, 002's `H`, 003's `U` and 004's `W`. This feature's decisions are `F-0NN`, as in plan.md.

## PR packaging

One intent per PR, test commit before implementation commit, interdependent intents stacked ([ADR 0013](../../docs/adr/0013-intent-sized-pull-requests.md)). Branches are `005/NN-slug`; 01 is this spec, plan and task list.

### Batch these

| PR | Tasks |
|---|---|
| `005/NN-gate-setup` | F006, F007, F008, F009 (merge-gate setup is self-serve, and stalls surface) |

### Isolate these

| Task | Why |
|---|---|
| **F001** ADR 0020 | A decision the maintainer accepts before the code that relies on it |
| **F002** ledger layout | A decision; its layout change touches every feature's plan |
| **F003**, **F004** factory skill | Each changes how every unattended run behaves; read alone |
| **F005** attribution check | A new CI gate; its blast radius is every PR |

---

## Phase 0: Decisions

- [ ] F001 `docs/adr/0020-factory-grants.md`, MADR, Proposed: the grant argument's fields ([data-model.md](./data-model.md#grant-us6-waits-on-f-002)), what a grant may and may not override with [ADR 0018](../../docs/adr/0018-hands-off-merging.md) named, scheduled triggers expiring with the grant, the options and one recommendation. No implementation until accepted. `human:decision` (F-002)
- [ ] F002 The ledger layout (F-003): the options (one file per decision with a generated table, an append-only section, as is) with one recommendation in plan.md's ledger, and the measured count of ledger conflicts in the last 50 merged stacked PRs. `human:decision`

## Phase 1: A kickoff with a blank field never starts (US1) 🎯 MVP

- [ ] F003 [US1] `.claude/skills/factory/SKILL.md`: before any work, scan the arguments for placeholders as [contracts/factory-arguments.md](./contracts/factory-arguments.md) defines them, stop at the first and name the missing field; task-level blockers still skip and record. A worked example in the PR body (no skill test harness)

## Phase 2: Every firing hands off, and a session stays short (US2)

- [ ] F004 [US2] `.claude/skills/factory/SKILL.md`: every firing, including one that stops early, ends by emitting the handoff in [contracts/handoff.md](./contracts/handoff.md); the session stops after one firing or at the next step boundary past plan.md's context threshold (F-004), with the reason; build output for a second target (wasm32) is removed after each step that built it. needs:F003. A worked example in the PR body

## Phase 3: Commits and PRs carry the maintainer's identity only (US3)

- [ ] F005 [P] [US3] `scripts/tests/check-attribution.test.sh` then `scripts/check-attribution.sh`: fails when a commit in `base..head` has an agent author, a Claude `Co-Authored-By` trailer or a `Claude-Session` line, or when the PR body has a "Generated with/by Claude Code" footer, naming each; never reads commits outside the range. Wired into a `pull_request` workflow step that passes the range and the body through the environment (research R2; actionlint passes). `AGENTS.md` "Commits and PRs" gains the one identity line (F-001), and the check's entry goes in `docs/reference/checks.md`

## Phase 4: The merge gate is set up without hand-holding (US4)

- [ ] F006 [P] [US4] `scripts/tests/gate-doctor.test.sh` then `scripts/gate-doctor.py`: reads, through `gh api`, every prerequisite in [data-model.md](./data-model.md#gate-prerequisite-us4) (each label in `.github/labels.json`, the repository's allow-auto-merge setting, the merge-gate App, `MERGE_GATE_MODE`) and prints each missing one with its fix; an unreadable one is "unknown" with the permission it needs; changes no setting; runs the same on Windows (research R3)
- [ ] F007 [P] [US4] `scripts/merge-gate.py` and `scripts/tests/merge-gate.test.sh`: in `dry-run` mode the gate comment says so and names what switches it to live (`MERGE_GATE_MODE=on`)
- [ ] F008 [P] [US4] `.claude/skills/steward/SKILL.md`: a check that has not started 15 min after the head was pushed, or not finished 60 min after it started (plan.md, Thresholds), is reported once and not re-polled
- [ ] F009 [US4] Confirm the stale-summary and changes-requested cases are tested in `scripts/tests/merge-gate.test.sh` (research R4) and say so on [#322](https://github.com/openplush/openplush/issues/322) with the test names. No code

## Phase 5: Every decision prompt can be answered cold (US5)

- [x] F010 [P] [US5] `AGENTS.md`: a short "Decision prompts" rule: facts, where it lives (file and ID), one recommendation, the default if unanswered, linking the glossary; it applies to `AskUserQuestion` calls, ledger-issue comments and handoffs. The glossary terms are 004's W043. As built: one bullet in Workflow, beside the stop-and-ask rule, where a prompt arises; it links the glossary page rather than the entries W043 adds, so it holds before and after W043 lands

## Phase 6: A grant is given once and honoured the same way (US6)

- [ ] F011 [US6] `.claude/skills/factory/SKILL.md`: `/factory` reads the grant argument and `continue`, as ADR 0020 sets, and states the grant in the handoff's kickoff. needs:F001,F004 `blocked:F-002`
- [ ] F012 [US6] `.claude/skills/factory/SKILL.md`, Scheduling: a firing after the grant's end does no work, reports the expiry and turns its scheduled task off. needs:F011 `blocked:F-002`

## Phase 7: Stacked PRs stop conflicting on shared lists (US7)

- [ ] F013 [US7] Apply the accepted ledger layout to every feature's plan.md, with a generator and check if the layout is generated. needs:F002 `blocked:F-003`

## Dependencies and execution order

- Phase 0 is drafted now and accepted by the maintainer; Phases 6 and 7 wait on it.
- Phases 1–5 are independent of each other except F004, which edits the same skill after F003.
- 004's W043 (glossary terms) lands beside F010; neither waits on the other.

## Parallel opportunities

- F003, F005, F006, F007, F008 and F010 touch different files and can run at once; F004 follows F003 in the same file.

## Implementation strategy

- MVP: US1 (F003). Then US2 and US3, which stop the costliest failures; then US4 and US5; US6 and US7 once their decisions are accepted.

## Notes

- Skill rules are proven by worked examples in their PR bodies; when a skill test harness exists, they gain fixtures.
