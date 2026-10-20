#!/usr/bin/env bash
# Tests for scripts/merge-gate.py: one case per gate condition, each a fetched
# PR state on stdin and the verdict it must get.
#
# What matters: every condition fails loudly with a detail naming what to fix,
# a tier H PR is never ready, tier C is ready like A, tier B holds 24 h after
# the last check, the review passes only by a summary for this head, its
# threads or review.yml's skip rules, a stale summary asks for one review per
# head, a reviewer's changes-requested blocks, and the comment carries its
# marker and the head it judged.
set -euo pipefail
# expect runs at the end of a pipeline and counts in this shell.
shopt -s lastpipe

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gate="$here/../merge-gate.py"
pass=0
fail=0

ok() { pass=$((pass + 1)); echo "ok   $1"; }
bad() { fail=$((fail + 1)); echo "FAIL $1"; [[ -n ${2:-} ]] && while IFS= read -r l; do echo "     | $l"; done <<<"$2"; return 0; }

head=0123456789abcdef0123456789abcdef01234567
now=2026-10-19T12:00:00Z
checks='["fmt, clippy, test", "licenses and advisories", "typecheck, unit, e2e", "build and test on wasm32"]'

# A tier A PR with everything green: each case mutates it with jq.
ready() {
  jq -nc --arg head "$head" --arg now "$now" --argjson checks "$checks" '{
    mode: "dry-run", now: $now, repository: "openplush/openplush",
    pr: {number: 7, draft: false, title: "feat(web): show the layout",
         base: {ref: "main"}, head: {sha: $head, repo: {full_name: "openplush/openplush"}},
         mergeable: true, mergeable_state: "clean", auto_merge: null, labels: [{name: "tier:A"}]},
    review_decision: "APPROVED",
    tier: {tier: "A", computed: "A", label: "A", relabel: false, review_exempt: false, reasons: ["no path reaches tier B or C"]},
    check_runs: [$checks[] | {id: 1, name: ., status: "completed", conclusion: "success", completed_at: "2026-10-19T11:00:00Z"}],
    comments: [{user: {login: "github-actions[bot]"},
                body: "**Review summary (PR 7, head \($head))**\n\nFindings: 🔴 0 / 🟡 1 / 🟣 0 (all posted inline)."}],
    threads: [{isResolved: true, comments: [{body: "🟡 could be shorter"}]}]
  }'
}

# expect <case> <jq expression that must be true>, the fetched state on stdin.
expect() {
  local name=$1 want=$2 out
  if ! out=$(python3 "$gate" 2>&1); then bad "$name (exit nonzero)" "$out"; return; fi
  if [[ $(jq -r "$want" <<<"$out") == true ]]; then ok "$name"; else bad "$name: expected $want" "$out"; fi
}
# failing <condition name> <detail substring>: a jq expression for one failed condition.
failing() { printf '(.ready | not) and ([.conditions[] | select(.name == "%s")] | .[0] | (.ok | not) and (.detail | contains("%s")))' "$1" "$2"; }

ready | expect "everything green on tier A is ready" '.ready and (.conditions | all(.ok)) and .tier == "A"'
ready | expect "the comment carries its marker and the head" '.comment | startswith("<!-- merge-gate -->") and contains("0123456")'
ready | expect "a fresh summary asks for no review" '.rereview == false and (.comment | contains("<!-- rereview:") | not)'
ready | jq -c '.pr.draft = true' | expect "a draft is not ready" "$(failing "not a draft" "draft")"
ready | jq -c '.pr.head.repo.full_name = "someone/openplush"' | expect "a fork head is not ready" "$(failing "head in this repository" "someone/openplush")"
ready | jq -c '.pr.base.ref = "001/10-parent"' | expect "a stacked PR is not ready" "$(failing "base is main" "001/10-parent")"
ready | jq -c '.pr.mergeable = false | .pr.mergeable_state = "dirty"' | expect "a conflict is not ready" "$(failing "mergeable, no conflict" "dirty")"
ready | jq -c '.pr.mergeable = null | .pr.mergeable_state = "unknown"' | expect "unknown mergeability is not ready" "$(failing "mergeable, no conflict" "not computed")"
ready | jq -c '.check_runs |= map(select(.name != "typecheck, unit, e2e"))' | expect "a missing required check is not ready" "$(failing "required checks green on this head" "typecheck, unit, e2e: missing")"
ready | jq -c '.check_runs[0].conclusion = "failure"' | expect "a failed required check is not ready" "$(failing "required checks green on this head" "fmt, clippy, test: failure")"
ready | jq -c '.check_runs[0] |= (.status = "in_progress" | .conclusion = null)' | expect "a running required check is not ready" "$(failing "required checks green on this head" "fmt, clippy, test: in_progress")"
ready | jq -c '.check_runs[0].conclusion = "skipped"' | expect "a skipped required check counts as green" '.ready'
ready | jq -c '.check_runs += [.check_runs[0] | .id = 9 | .conclusion = "success"] | .check_runs[0].conclusion = "failure"' |
  expect "the newest run of a check name wins (a re-run after a failure)" '.ready'
ready | jq -c '.check_runs += [{id: 5, name: "adversarial review", status: "completed", conclusion: "failure", completed_at: "2026-10-19T11:00:00Z"}]' |
  expect "a failed review run on this head is not ready" "$(failing "review" "adversarial review: failure")"
ready | jq -c '.check_runs += [{id: 5, name: "adversarial review", status: "in_progress", conclusion: null, completed_at: null}]' |
  expect "a review still running on this head is not ready" "$(failing "review" "adversarial review: in_progress")"
ready | jq -c '.comments = []' | expect "no summary and no exemption is not ready" "$(failing "review" "no review summary")"
ready | jq -c '.comments = [] | .tier.review_exempt = true' | expect "a review skipped by its rules passes without a summary" '.ready'
ready | jq -c '.comments[0].user.login = "lvndrpup"' | expect "a summary not by github-actions is not a summary" "$(failing "review" "no review summary")"
ready | jq -c '.comments[0].body |= sub("🔴 0"; "🔴 1") | .threads += [{isResolved: false, comments: [{body: "🔴 wrong sign"}]}]' |
  expect "a 🔴 with its thread open is not ready" "$(failing "review" "1 🔴 thread")"
ready | jq -c '.comments[0].body |= sub("🔴 0"; "🔴 1") | .threads += [{isResolved: true, comments: [{body: "🔴 wrong sign"}, {body: "fixed in abc"}]}]' |
  expect "a 🔴 whose thread is resolved passes" '.ready'
ready | jq -c --arg head "$head" '.comments += [{user: {login: "github-actions[bot]"}, body: "**Review summary (PR 7, head \($head))**\n\nFindings: 🔴 2 / 🟡 0 / 🟣 0."}] | .threads += [{isResolved: false, comments: [{body: "🔴 later"}]}]' |
  expect "the newest summary is the one read" "$(failing "review" "2 🔴")"
ready | jq -c --arg head "$head" '.comments[0].body = "**Review summary (PR 7, head \($head))**\n\nno counts here"' | expect "an unreadable summary is not ready" "$(failing "review" "count not found")"
ready | jq -c '.comments[0].body = "**Review summary (PR 7, head x)**\n\nFindings: 🔴 0"' | expect "a summary naming no head is not ready" "$(failing "review" "head not found")"
old=1111111111111111111111111111111111111111
stale() { ready | jq -c --arg old "$old" '.comments[0].body |= sub("head [0-9a-f]+"; "head \($old)")'; }
stale | expect "a summary for another head is not ready and asks for a review" "$(failing "review" "for head 1111111, not this head") and .rereview == true"
stale | expect "the request is marked in the comment, so it is made once per head" '.comment | contains("<!-- rereview: 0123456789abcdef0123456789abcdef01234567 -->")'
stale | jq -c '.comments += [{user: {login: "openplush-merge-gate[bot]"}, body: "<!-- merge-gate -->\n<!-- rereview: 0123456789abcdef0123456789abcdef01234567 -->\n### Merge gate: not ready"}]' |
  expect "a review already requested for this head is not requested again" "$(failing "review" "requested") and .rereview == false and (.comment | contains(\"<!-- rereview: 0123456\"))"
stale | jq -c '.comments += [{user: {login: "openplush-merge-gate[bot]"}, body: "<!-- merge-gate -->\n<!-- rereview: 2222222222222222222222222222222222222222 -->\n### Merge gate: not ready"}]' |
  expect "a request for an earlier head does not count for this one" '.rereview == true'
stale | jq -c '.pr.labels += [{name: "rereview"}]' | expect "a rereview label already on the PR waits for the review" "$(failing "review" "waiting") and .rereview == false"
stale | jq -c '.check_runs += [{id: 5, name: "adversarial review", status: "completed", conclusion: "success", completed_at: "2026-10-19T11:30:00Z"}]' |
  expect "a review that ran on this head without a summary is the maintainer's to look at, not asked again" "$(failing "review" "maintainer") and .rereview == false"
stale | jq -c '.tier.review_exempt = true' | expect "a stale summary on an exempt PR still asks for a review" '.rereview == true'
ready | jq -c '.review_decision = "CHANGES_REQUESTED"' | expect "a reviewer's changes requested is not ready" "$(failing "no changes requested" "requested changes")"
ready | jq -c '.review_decision = "REVIEW_REQUIRED"' | expect "review required by a ruleset is not the gate's concern" '.ready'
ready | jq -c '.review_decision = null' | expect "no review decision is fine" '.ready'
ready | jq -c '.threads += [{isResolved: false, comments: [{body: "🟡 rename this"}]}]' | expect "an unresolved 🟡 thread is not ready" "$(failing "no unresolved review threads" "1 unresolved")"
ready | jq -c '.tier.tier = "B" | .tier.label = "B"' | expect "tier B within 24 h holds" "$(failing "tier" "holds until 2026-10-20T11:00:00Z")"
ready | jq -c '.tier.tier = "B" | .tier.label = "B"' | expect "the hold is reported" '.hold_until == "2026-10-20T11:00:00Z"'
ready | jq -c '.tier.tier = "B" | .now = "2026-10-20T11:00:00Z"' | expect "tier B after 24 h is ready" '.ready'
ready | jq -c '.tier.tier = "B" | .now = "2026-10-20T11:00:00Z" | .check_runs[1].completed_at = "2026-10-20T10:30:00Z"' |
  expect "the hold counts from the latest required check" "$(failing "tier" "holds until 2026-10-21T10:30:00Z")"
ready | jq -c '.tier.tier = "C" | .tier.computed = "C"' | expect "tier C is ready like A, with no hold" '.ready and .hold_until == null'
ready | jq -c '.tier.tier = "H" | .tier.computed = "H"' | expect "tier H is never ready" "$(failing "tier" "the maintainer merges")"
ready | jq -c '.tier.tier = "H" | .tier.computed = "H"' | expect "tier H says so in the comment" '.comment | contains("the maintainer merges")'
ready | jq -c '.mode = "on"' | expect "mode on is ready too" '.ready and (.comment | contains("mode on"))'
ready | jq -c '.pr.draft = true | .check_runs = []' | expect "every failed condition is listed, not just the first" '[.conditions[] | select(.ok | not)] | length == 2'

if out=$(python3 "$gate" <<<'{"mode": "on"}' 2>&1); then bad "a missing field fails" "$out"; else ok "a missing field fails"; fi
if out=$(ready | jq -c '.mode = "sideways"' | python3 "$gate" 2>&1); then bad "an unknown mode fails" "$out"; else ok "an unknown mode fails"; fi

echo "merge-gate tests: $pass passed, $fail failed"
((fail == 0))
