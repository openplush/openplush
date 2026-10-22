---
description: "Task list for 005 factory hardening"
kind: process
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
| **F014** ADR 0021 | A decision the maintainer accepts before the work that applies it |
| **F016** the rename | Touches every feature's spec files; lands while no open PR edits 002 to 005, and read alone |
| **F017** the scheme checks | Changes what the status page counts; its blast radius is every `tasks.md` |

---

## Phase 0: Decisions

- [x] F001 `docs/adr/0020-factory-grants.md`, MADR, Proposed: the grant argument's fields ([data-model.md](./data-model.md#grant-us6-adr-0020)), what a grant may and may not override with [ADR 0018](../../docs/adr/0018-hands-off-merging.md) named, scheduled triggers expiring with the grant, the options and one recommendation. No implementation until accepted. (F-002) As built: accepted by the maintainer on 2026-10-21, as option 2 (a grant may merge a tier H PR the gate shows green but for its tier)
- [x] F002 The ledger layout (F-003): the options (one file per decision with a generated table, an append-only section, as is) with one recommendation in plan.md's ledger, and the measured count of ledger conflicts in the last 50 merged stacked PRs. As built: accepted by the maintainer on 2026-10-21, as (c), the ledger as is

## Phase 1: A kickoff with a blank field never starts (US1) 🎯 MVP

- [x] F003 [US1] `.claude/skills/factory/SKILL.md`: before any work, scan the arguments for placeholders as [contracts/factory-arguments.md](./contracts/factory-arguments.md) defines them, stop at the first and name the missing field; task-level blockers still skip and record. A worked example in the PR body (no skill test harness). As built: the skill's Arguments section gives both shapes as patterns and scans only the arguments the firing received, never the skill's own `<feature>` notation; the field named is the label before the placeholder on its line, else the bracketed word; a kickoff stop writes nothing, not even the ledger comment, so its reason `kickoff: placeholder` is the session's output; `docs/explanation/agents.md` lists the new stop

## Phase 2: Every firing hands off, and a session stays short (US2)

- [x] F004 [US2] `.claude/skills/factory/SKILL.md`: every firing, including one that stops early, ends by emitting the handoff in [contracts/handoff.md](./contracts/handoff.md); the session stops after one firing or at the next step boundary past plan.md's context threshold (F-004), with the reason; build output for a second target (wasm32) is removed after each step that built it. needs:F003. A worked example in the PR body. As built: the handoff is a skill section emitted after step 6's ledger comment, its State opening with the ledger's `Stopped:` reason (a new `one firing` when no stop condition ended it); a kickoff stop, which writes no ledger comment, still emits it, leaving the unfillable field out of the kickoff and asking for it under Blockers; the 300k-token stop (plan.md, Thresholds; F-004) is checked at each step boundary, defined as the end of a numbered step or of one task in step 4; a Build output section puts every agent of a firing on one shared `CARGO_TARGET_DIR` and removes its wasm32 output unless another agent's wasm32 build is running; `docs/explanation/agents.md` describes the handoff and the cap

## Phase 3: Commits and PRs carry the maintainer's identity only (US3)

- [x] F005 [P] [US3] `scripts/tests/check-attribution.test.sh` then `scripts/check-attribution.sh`: fails when a commit in `base..head` has an agent author, a Claude `Co-Authored-By` trailer or a `Claude-Session` line, or when the PR body has a "Generated with/by Claude Code" footer, naming each; never reads commits outside the range. Wired into a `pull_request` workflow step that passes the range and the body through the environment (research R2; actionlint passes). `AGENTS.md` "Commits and PRs" gains the one identity line (F-001), and the check's entry goes in `docs/reference/checks.md`. As built: the step is the last of `engine.yml`'s `fmt, clippy, test` job, so it blocks the merge gate, with `fetch-depth: 0` and the base and head SHAs fetched; it reads the body from the API when it runs, not from the event (R2), because the event keeps the body as opened and a re-run would still see a footer removed since; an agent committer and a footer or session link in a commit message fail too; `scripts/check.sh` does not run it (no PR locally); `steward` links the identity line instead of restating it

## Phase 4: The merge gate is set up without hand-holding (US4)

- [x] F006 [P] [US4] `scripts/tests/gate-doctor.test.sh` then `scripts/gate-doctor.py`: reads, through `gh api`, every prerequisite in [data-model.md](./data-model.md#gate-prerequisite-us4) (each label in `.github/labels.json`, the repository's allow-auto-merge setting, the merge-gate App, `MERGE_GATE_MODE`) and prints each missing one with its fix; an unreadable one is "unknown" with the permission it needs; changes no setting; runs the same on Windows (research R3). As built: GitHub shows an App's installation only to the App, so the doctor checks the App by the two secrets `merge-gate.yml` reads it from (`MERGE_GATE_APP_ID`, `MERGE_GATE_PRIVATE_KEY`) and takes the App's permissions for the fix from that workflow; a 404 on a secret or the variable counts as missing only for an admin, since GitHub may hide them from others; `dry-run` is ok, with the command that makes it live
- [x] F007 [P] [US4] `scripts/merge-gate.py` and `scripts/tests/merge-gate.test.sh`: in `dry-run` mode the gate comment says so and names what switches it to live (`MERGE_GATE_MODE=on`). As built: every dry-run comment, ready or not and tier H too, ends with the rehearsal line; the ready verdict no longer repeats that auto-merge is off
- [x] F008 [P] [US4] `.claude/skills/steward/SKILL.md`: a check that has not started 15 min after the head was pushed, or not finished 60 min after it started (plan.md, Thresholds), is reported once and not re-polled. As built: one bullet in `steward`, Waiting, with the thresholds and their reason inline; it reports to the user, or in a `factory` firing's ledger comment, and looks again on the next event or check-in
- [x] F009 [US4] Confirm the stale-summary and changes-requested cases are tested in `scripts/tests/merge-gate.test.sh` (research R4) and say so on [#322](https://github.com/openplush/openplush/issues/322) with the test names. No code. As built: the stale summary is covered by "a summary for another head is not ready and asks for a review" and the six cases after it, changes-requested by "a reviewer's changes requested is not ready", "review required by a ruleset is not the gate's concern" and "no review decision is fine"; said so on #322

## Phase 5: Every decision prompt can be answered cold (US5)

- [x] F010 [P] [US5] `AGENTS.md`: a short "Decision prompts" rule: facts, where it lives (file and ID), one recommendation, the default if unanswered, linking the glossary; it applies to `AskUserQuestion` calls, ledger-issue comments and handoffs. The glossary terms are 004's W043. As built: one bullet in Workflow, beside the stop-and-ask rule, where a prompt arises; it links the glossary page rather than the entries W043 adds, so it holds before and after W043 lands

## Phase 6: A grant is given once and honoured the same way (US6)

- [x] F011 [US6] `.claude/skills/factory/SKILL.md`: `/factory` reads the grant argument and `continue`, as ADR 0020 sets, and states the grant in the handoff's kickoff. needs:F001,F004. As built: Arguments gains Grant (syntax, record, continue, renew); a new Merging under a grant section holds ADR 0020's merge rule; the firing writes `ends=` into the grant record and the kickoff, so a fresh session or a scheduled firing continues the same clock; the ledger comment gains `Grant:`, `Merged:` and `Accepted:` lines. AGENTS.md, `steward`, the tier table, the agents explanation and the glossary name the exception with a link
- [x] F012 [US6] `.claude/skills/factory/SKILL.md`, Scheduling: a firing after the grant's end does no work, reports the expiry and turns its scheduled task off. needs:F011. As built: an End check in One firing runs after the grant is read and again at each step boundary; past `ends=` the firing turns off each enabled scheduled task whose prompt is its kickoff and stops with `grant expired` (a new Stop conditions row), and its kickoff becomes `/factory continue`. Scheduling fires the handoff's kickoff, never `/factory continue`

## Phase 7: Stacked PRs stop conflicting on shared lists (US7)

- [x] F013 [US7] Generate the folder index lists, as 004's W044 does; the ledger stays as it is (F-003). needs:W044. As built: W044 (#354) generates them; nothing else to apply

## Phase 8: Status reads without decoding IDs (US8)

- [x] F014 [US8] `docs/adr/0021-one-id-scheme.md`, MADR: one ID scheme in every feature, renamed in place, spike numbers retired, the alias rule for IDs in history, and the one exception to ADR 0019; the options measured against the prefixes in use. (F-005) As built: accepted by the maintainer on 2026-10-22; listed in `docs/SUMMARY.md`
- [ ] F015 [P] [US8] `AGENTS.md` (Comments and prose: one bullet), `.claude/skills/factory/SKILL.md` (Ledger, Handoff and stop-reason templates: words first, linked IDs; no task ID in branch names), `.claude/skills/steward/SKILL.md` (finding answers and handoffs), `.github/pull_request_template.md` (the Tasks example), and a link from `docs/how-to/write-docs.md`'s process-ID rule: the status-prose rule (F-006, FR-018). A worked example in the PR body: the last firing comment on #241 rewritten. Check by hand before merging whether a GitHub search for `"001 T084"` returns only 001's PRs; if not, the factory's readiness check searches the task's branch instead. needs:F014
- [ ] F016 [US8] The rename (F-005, FR-019, FR-020): in 002 to 005, qualify every reference to another feature's ID with its feature first, then rename tasks to `T` and decisions to `D-`, one commit per feature; retire spike labels in 001 to 003 in favour of their task IDs, and turn the one readiness token that names a spike, `human:S8` on 001's T008, into `human:sewist` (`scripts/pr-tier.py` and the factory read only the literal `human:`, so the reason word is free); drop spike IDs from `research/` folder names and their references, and IDs from research code comments; update ADRs, `docs/` and `scripts/tests/` references; the glossary's ID table gets one row per kind and the alias rule. Verified by a one-off script, given in the PR body, that every bare `T` and `D-` in a feature's files resolves to that feature. needs:F014
- [ ] F017 [US8] `scripts/tests/gen-status.test.sh`, `scripts/tests/report-tick-lag.test.sh`, `scripts/tests/check-hygiene.test.sh`, `scripts/tests/check-docs-plain.test.sh` then their scripts: a task line is counted only as `T`, and a task line or ledger row in another form fails `scripts/check.sh` naming it (FR-021); the hygiene and maker-page checks match the current scheme. needs:F016

## Dependencies and execution order

- Phase 0 is drafted now and accepted by the maintainer; Phases 6 and 7 wait on it.
- Phases 1–5 are independent of each other except F004, which edits the same skill after F003.
- Phase 8 starts after F014 is accepted; F015 and F016 are independent, F017 follows F016 so its checks pass on the renamed tree.
- 004's W043 (glossary terms) lands beside F010; neither waits on the other.

## Parallel opportunities

- F003, F005, F006, F007, F008 and F010 touch different files and can run at once; F004 follows F003 in the same file.

## Implementation strategy

- MVP: US1 (F003). Then US2 and US3, which stop the costliest failures; then US4 and US5; US6 and US7 once their decisions are accepted.

## Notes

- Skill rules are proven by worked examples in their PR bodies; when a skill test harness exists, they gain fixtures.
