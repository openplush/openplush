# Data Model: Factory Hardening

No stored data; these are the shapes agents and scripts read and write.

## Grant (US6, ADR 0020)

| Field | Type | Rule |
|---|---|---|
| hours | positive number | Ends at start + hours; a firing after that does no work |
| features | list of `NNN` | Only these features' tasks are picked |
| merge | yes / no | "yes" lets the run merge a tier H PR only when the merge gate shows every condition but the tier passing (ADR 0020) |
| paths | list of path prefixes | Paths the run may change beyond its tier's default |
| decisions | yes / no | Whether the run may accept a Proposed decision |

`continue` renews the last grant with a fresh start time.

## Handoff (US2)

See [contracts/handoff.md](contracts/handoff.md). Every firing emits exactly one, last.

## Decision prompt (US5)

| Part | Rule |
|---|---|
| Facts | What is true now, measured, with numbers where there are any |
| Where it lives | File and ID (`specs/004-docs-site/plan.md` D-017) |
| Recommendation | Exactly one |
| Default | What happens if nobody answers |

## Gate prerequisite (US4)

| Prerequisite | Read through | States |
|---|---|---|
| Each label in `.github/labels.json` | the repository's labels | present, missing |
| Repository auto-merge setting | the repository | on, off, unknown |
| The merge-gate App installed | the App's installation | present, missing, unknown |
| `MERGE_GATE_MODE` variable | the repository's variables | `on`, `dry-run`, unset or `off`, unknown |

"Unknown" names the permission the caller lacks.
