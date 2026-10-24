# Data Model: Factory Hardening

No stored data; these are the shapes agents and scripts read and write.

## Grant (US6, ADR 0020)

| Field | Type | Rule |
|---|---|---|
| hours | positive number | Ends at start + hours; a firing after that does no work |
| features | list of `NNN` | Only these features' tasks are picked |
| paths | list of path prefixes | Protected paths the run may change beyond its tasks' own |
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
