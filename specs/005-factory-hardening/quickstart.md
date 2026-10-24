# Quickstart: seeing each story work

| Story | Run | Expect |
|---|---|---|
| US1 | `/factory 005 <window>` | Stops at once, naming `<window>`; nothing pushed |
| US2 | `/factory 005` on a quiet feature | Ends with a handoff ([contract](contracts/handoff.md)); the session stops |
| US3 | `scripts/tests/check-attribution.test.sh`; then a PR with an agent trailer | Test passes; the PR's check fails naming the commit |
| US4 | A check not started 15 minutes after the push | The agent reports it once and stops polling |
| US5 | Read a handoff's blockers | Each has facts, file and ID, one recommendation, the default |
| US6 | `/factory 005 grant hours=1 paths=specs/ decisions=no`, then `/factory continue` | The handoff's kickoff states the grant; `continue` renews it with a fresh start |
| US7 | `scripts/check.sh docs` once 004's T044 lands | Folder index lists are generated; the ledger stays as it is (D-003) |
