# Quickstart: seeing each story work

| Story | Run | Expect |
|---|---|---|
| US1 | `/factory 005 <window>` | Stops at once, naming `<window>`; nothing pushed |
| US2 | `/factory 005` on a quiet feature | Ends with a handoff ([contract](contracts/handoff.md)); the session stops |
| US3 | `scripts/tests/check-attribution.test.sh`; then a PR with an agent trailer | Test passes; the PR's check fails naming the commit |
| US4 | `python3 scripts/gate-doctor.py` | Lists each missing prerequisite with its fix; changes nothing |
| US4 | Gate in `dry-run` | Its comment says dry-run and names `MERGE_GATE_MODE=on` |
| US5 | Read a handoff's blockers | Each has facts, file and ID, one recommendation, the default |
| US6 | `/factory 005 grant hours=1 merge=no paths=specs/ decisions=no`, then `/factory continue` | The handoff's kickoff states the grant; `continue` renews it with a fresh start |
| US7 | `scripts/check.sh docs` once 004's W044 lands | Folder index lists are generated; the ledger stays as it is (F-003) |
