#!/usr/bin/env bash
# Tests for scripts/report-tick-lag.sh: each case runs the report on a throwaway
# repository whose commits have fixed dates, and compares its whole output.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
report="$here/../report-tick-lag.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export LC_ALL=C
# A git hook exports these, and they would point every git call at this checkout.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE

pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

# expect <case> <wanted exit> <wanted output> <report args>...
expect() {
  local name=$1 want_status=$2 want=$3 out status=0
  shift 3
  out=$("$report" "$@" 2>&1) || status=$?
  if [[ $status -eq $want_status && $out == "$want" ]]; then
    ok "$name"
  else
    bad "$name (wanted exit $want_status, got $status)" "$(diff <(echo "$want") <(echo "$out") || true)"
  fi
}

# Fixture IDs use prefixes no feature has, so none reads as a reference to a real task.

repo="$tmp/repo"
g() { git -C "$repo" -c core.hooksPath=/dev/null -c commit.gpgsign=false "$@"; }

# commit <date> <subject>: an empty commit dated <date> at midnight UTC.
commit() {
  GIT_AUTHOR_DATE="$1T00:00:00Z" GIT_COMMITTER_DATE="$1T00:00:00Z" \
    GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@example.invalid \
    GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@example.invalid \
    g commit -q --allow-empty -m "$2"
}

mkdir -p "$repo/specs/001-a" "$repo/specs/002-b" "$repo/specs/004-d"
git init -q -b main "$repo"
printf '%s\n' "# Tasks" "" "- [ ] A010 pipeline work" >"$repo/specs/001-a/tasks.md"
printf '%s\n' "- [ ] B001 hints" >"$repo/specs/002-b/tasks.md"
printf '%s\n' "## Phase 1" "" "- [ ] C001 first" "- [x] C002 done. As built: fine" "- [ ] C003 third" \
  "- [ ] C004 fourth" "- [ ] C005 fifth" "- [ ] C006 sixth" "- [ ] C007 seventh" >"$repo/specs/004-d/tasks.md"
g add specs
commit 2026-09-30 "docs(tasks): the tasks"
commit 2026-10-01 "feat(a): first slice (C001)"
commit 2026-10-05 "feat(a): ticked work (C002)"
commit 2026-10-08 "feat(b): two at once (C003, C004)"
commit 2026-10-09 "docs(tasks): add work (C005–C006)"
commit 2026-10-10 "feat(a): second slice (C001)"
commit 2026-10-12 "feat(c): pipeline work (A010, repair)"
commit 2026-10-14 "feat(d): an ID no tasks.md has (C099)"
commit 2026-10-15 "fix(e): an ID outside parentheses C007"
commit 2026-10-16 "feat(h): hints (B001)"

lag_main="001-a A010 8d feat(c): pipeline work (A010, repair)
002-b B001 4d feat(h): hints (B001)
004-d C001 10d feat(a): second slice (C001)
004-d C003 12d feat(b): two at once (C003, C004)
004-d C004 12d feat(b): two at once (C003, C004)
tick-lag: 5 unticked tasks named on main"

expect "an unticked task named in a subject is listed, a ticked one is not, sorted by feature then ID" 0 \
  "$lag_main" --now 2026-10-20 "$repo"
expect "--now takes an epoch" 0 "$lag_main" --now "$(date -u -d 2026-10-20 +%s)" "$repo"
expect "--now fixes the age" 0 "001-a A010 18d feat(c): pipeline work (A010, repair)
002-b B001 14d feat(h): hints (B001)
004-d C001 20d feat(a): second slice (C001)
004-d C003 22d feat(b): two at once (C003, C004)
004-d C004 22d feat(b): two at once (C003, C004)
tick-lag: 5 unticked tasks named on main" --now 2026-10-30 "$repo"

first=$("$report" --now 2026-10-20 "$repo" 2>&1) || true
second=$("$report" --now 2026-10-20 "$repo" 2>&1) || true
if [[ $first == "$second" ]]; then ok "two runs print the same bytes"; else bad "two runs print the same bytes" "$second"; fi

# Run from inside the repository with no root argument.
out=$(cd "$repo" && "$report" --now 2026-10-20 2>&1) || true
if [[ $out == "$lag_main" ]]; then ok "the root defaults to the current repository"; else bad "the root defaults to the current repository" "$out"; fi

g checkout -q -b side
commit 2026-10-18 "feat(s): side work (C005)"
g checkout -q main
expect "--ref picks the branch" 0 "001-a A010 8d feat(c): pipeline work (A010, repair)
002-b B001 4d feat(h): hints (B001)
004-d C001 10d feat(a): second slice (C001)
004-d C003 12d feat(b): two at once (C003, C004)
004-d C004 12d feat(b): two at once (C003, C004)
004-d C005 2d feat(s): side work (C005)
tick-lag: 6 unticked tasks named on side" --ref side --now 2026-10-20 "$repo"

g update-ref refs/remotes/origin/main main
g checkout -q --detach
g branch -q -D main
expect "without a local main the report reads origin/main" 0 "${lag_main/on main/on origin/main}" --now 2026-10-20 "$repo"

sed -i 's/^- \[ \] /- [x] /' "$repo/specs/004-d/tasks.md" "$repo/specs/001-a/tasks.md"
sed -i 's/^- \[ \] /- [X] /' "$repo/specs/002-b/tasks.md"
expect "a tick only in the working tree does not count" 0 "${lag_main/on main/on origin/main}" --now 2026-10-20 "$repo"
g add specs
commit 2026-10-19 "docs(tasks): tick"
g update-ref refs/remotes/origin/main HEAD
expect "nothing lags once every named task is ticked" 0 "tick-lag: 0 unticked tasks named on origin/main" \
  --now 2026-10-20 "$repo"

expect "an unknown option is a usage error" 2 "report-tick-lag: unknown option --bogus
usage: scripts/report-tick-lag.sh [--ref <ref>] [--now <epoch|date>] [root]" --bogus "$repo"
expect "an unreadable --now is a usage error" 2 "report-tick-lag: --now: not an epoch or a date: someday
usage: scripts/report-tick-lag.sh [--ref <ref>] [--now <epoch|date>] [root]" --now someday "$repo"
expect "a --ref that does not resolve is a usage error" 2 "report-tick-lag: --ref: no commit named nope
usage: scripts/report-tick-lag.sh [--ref <ref>] [--now <epoch|date>] [root]" --ref nope "$repo"
g update-ref -d refs/remotes/origin/main
expect "no main and no origin/main is a usage error" 2 "report-tick-lag: neither main nor origin/main exists; pass --ref
usage: scripts/report-tick-lag.sh [--ref <ref>] [--now <epoch|date>] [root]" "$repo"

# Task numbers repeat across features (ADR 0021): a subject names its feature, as
# "(002 <task>)", and a bare ID keeps its meaning from before: T is 001's, and H, U,
# W, F are 002 to 005's. Task IDs are split so the hygiene check does not read them.
t1="T""001" t9="T""009"
repo="$tmp/features"
mkdir -p "$repo/specs/001-a" "$repo/specs/002-b"
git init -q -b main "$repo"
printf '%s\n' "- [ ] $t9 pipeline" >"$repo/specs/001-a/tasks.md"
printf '%s\n' "- [ ] $t1 hints" "- [ ] $t9 more hints" >"$repo/specs/002-b/tasks.md"
g add specs
commit 2026-10-01 "docs(tasks): the tasks"
commit 2026-10-02 "feat(a): pipeline work ($t9)"
commit 2026-10-03 "feat(h): written before the rename (H009)"
commit 2026-10-04 "feat(h): hints (002 $t1)"
expect "a subject's feature picks the task, and a bare ID keeps its earlier feature" 0 "001-a $t9 18d feat(a): pipeline work ($t9)
002-b $t1 16d feat(h): hints (002 $t1)
002-b $t9 17d feat(h): written before the rename (H009)
tick-lag: 3 unticked tasks named on main" --now 2026-10-20 "$repo"

echo
echo "$pass passed, $fail failed"
[[ $fail -eq 0 ]]
