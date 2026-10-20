#!/usr/bin/env python3
"""Judge whether a pull request may auto-merge, by ADR 0015 and ADR 0018, from fetched state.

Reads one JSON document on stdin, assembled by .github/workflows/merge-gate.yml:
  {"mode": "dry-run"|"on", "now": RFC 3339, "repository": "owner/name",
   "pr": GitHub's pull object, "tier": scripts/pr-tier.py's output,
   "check_runs": the head commit's check runs, "comments": the PR's issue comments,
   "threads": [{"isResolved": bool, "comments": [{"body": str}]}],
   "review_decision": GraphQL's reviewDecision or null}
and prints one:
  {"ready": bool, "tier": "A"|"B"|"C"|"H", "hold_until": RFC 3339|null, "rereview": bool,
   "conditions": [{"name": str, "ok": bool, "detail": str}], "comment": markdown}
`rereview` asks the workflow to add the label: the newest summary is for another head
and no review was asked for this one yet. It calls no API: the workflow fetches,
this decides, the workflow acts.
Needs Python 3.11+. Tests: scripts/tests/merge-gate.test.sh
"""

import json
import re
import sys
from typing import TypeVar
from datetime import datetime, timedelta

REQUIRED_CHECKS = ("fmt, clippy, test", "licenses and advisories", "typecheck, unit, e2e", "build and test on wasm32")
# Present only on the head the review ran on (ADR 0013: once per PR).
REVIEW_CHECKS = ("changed paths", "adversarial review")
GREEN = frozenset({"success", "skipped"})
SUMMARY_AUTHOR = "github-actions[bot]"
RED_COUNT = re.compile(r"🔴\s*(\d+)")
SUMMARY_HEAD = re.compile(r"head ([0-9a-f]{40})")
HOLD = timedelta(hours=24)
MARKER = "<!-- merge-gate -->"
# Left in the gate's own comment for each head it asked a review for, so it asks once.
REREVIEW_MARK = re.compile(r"<!-- rereview: ([0-9a-f]{40}) -->")
REREVIEW_LABEL = "rereview"
CHANGES_REQUESTED = "CHANGES_REQUESTED"
MODES = ("dry-run", "on")

T = TypeVar("T")
Doc = dict[str, object]
Condition = tuple[str, bool, str]
# A review verdict: ok, detail, and whether to ask for a review on this head now.
Review = tuple[bool, str, bool]


class InputError(Exception):
    """The input document is not what the workflow assembles."""


def field(obj: object, name: str, kind: type[T]) -> T:
    value = obj.get(name) if isinstance(obj, dict) else None
    if not isinstance(value, kind):
        raise InputError(f"'{name}' must be {kind.__name__}, got {type(value).__name__}")
    return value


def parse_time(text: str) -> datetime:
    return datetime.fromisoformat(text.replace("Z", "+00:00"))


def fmt_time(when: datetime) -> str:
    return when.strftime("%Y-%m-%dT%H:%M:%SZ")


def latest_runs(check_runs: list[object]) -> dict[str, Doc]:
    """The newest check run per name: a re-run supersedes an earlier failure."""
    latest: dict[str, Doc] = {}
    for run in check_runs:
        if not isinstance(run, dict):
            continue
        name = field(run, "name", str)
        if name not in latest or field(run, "id", int) > field(latest[name], "id", int):
            latest[name] = run
    return latest


def run_state(run: Doc | None) -> str:
    if run is None:
        return "missing"
    status = field(run, "status", str)
    return field(run, "conclusion", str) if status == "completed" else status


def check_condition(runs: dict[str, Doc], names: tuple[str, ...], absent_ok: bool) -> tuple[bool, str]:
    problems = []
    for name in names:
        run = runs.get(name)
        if run is None and absent_ok:
            continue
        state = run_state(run)
        if state not in GREEN:
            problems.append(f"{name}: {state}")
    return (False, "; ".join(problems)) if problems else (True, "")


def newest_summary(comments: list[object], number: int) -> str | None:
    prefix = f"**Review summary (PR {number}, head "
    found = None
    for comment in comments:
        if not isinstance(comment, dict):
            continue
        body = field(comment, "body", str)
        if field(field(comment, "user", dict), "login", str) == SUMMARY_AUTHOR and body.startswith(prefix):
            found = body
    return found


def open_red_threads(threads: list[object]) -> int:
    count = 0
    for thread in threads:
        if not isinstance(thread, dict) or field(thread, "isResolved", bool):
            continue
        bodies = [field(c, "body", str) for c in field(thread, "comments", list) if isinstance(c, dict)]
        if any("🔴" in body for body in bodies):
            count += 1
    return count


def asked_heads(comments: list[object]) -> set[str]:
    """The heads the gate's own comment already asked a review for."""
    heads: set[str] = set()
    for comment in comments:
        if isinstance(comment, dict) and field(comment, "body", str).startswith(MARKER):
            heads.update(REREVIEW_MARK.findall(field(comment, "body", str)))
    return heads


def pr_labels(pr: Doc) -> list[str]:
    return [field(label, "name", str) for label in field(pr, "labels", list) if isinstance(label, dict)]


def stale_summary(doc: Doc, runs: dict[str, Doc], head: str, reviewed: str) -> Review:
    """Why a summary for another head fails, and whether to ask for a review now."""
    where = f"newest summary is for head {reviewed[:7]}, not this head"
    if rereview_pending(doc):
        return False, f"{where}; `rereview` is on the PR, waiting for the review to start", False
    if any(name in runs for name in REVIEW_CHECKS):
        return False, f"{where}, and the review ran on this head without posting one: the maintainer must look", False
    if head in asked_heads(field(doc, "comments", list)):
        return False, f"{where}; a review was requested for this head", False
    return False, f"{where}: requesting a review", True


def rereview_pending(doc: Doc) -> bool:
    return REREVIEW_LABEL in pr_labels(field(doc, "pr", dict))


def summary_findings(summary: str, threads: list[object]) -> tuple[bool, str]:
    match = RED_COUNT.search(summary)
    if match is None:
        return False, "summary found but its 🔴 count not found"
    reds = int(match[1])
    if reds == 0:
        return True, "newest summary reports 🔴 0"
    open_reds = open_red_threads(threads)
    if open_reds:
        return False, f"newest summary reports {reds} 🔴, {open_reds} 🔴 thread(s) unresolved"
    return True, f"newest summary reports {reds} 🔴, every 🔴 thread resolved"


def review_condition(doc: Doc, runs: dict[str, Doc], exempt: bool) -> Review:
    ok, detail = check_condition(runs, REVIEW_CHECKS, absent_ok=True)
    if not ok:
        return False, detail, False
    pr = field(doc, "pr", dict)
    head = field(field(pr, "head", dict), "sha", str)
    summary = newest_summary(field(doc, "comments", list), field(pr, "number", int))
    if summary is None:
        return (True, "skipped by review.yml's rules", False) if exempt else (False, "no review summary yet", False)
    match = SUMMARY_HEAD.search(summary)
    if match is None:
        return False, "summary found but its head not found", False
    if match[1] != head:
        return stale_summary(doc, runs, head, match[1])
    ok, detail = summary_findings(summary, field(doc, "threads", list))
    return ok, detail, False


def tier_condition(tier: str, runs: dict[str, Doc], now: datetime) -> tuple[bool, str, datetime | None]:
    if tier in ("A", "C"):
        return True, f"tier {tier}", None
    if tier == "H":
        return False, "tier H: the maintainer merges", None
    completed = [field(runs[n], "completed_at", str) for n in REQUIRED_CHECKS if n in runs and runs[n].get("completed_at")]
    if len(completed) < len(REQUIRED_CHECKS):
        return False, "tier B: the 24 h hold starts when every required check has completed", None
    until = max(parse_time(t) for t in completed) + HOLD
    if now < until:
        return False, f"tier B: holds until {fmt_time(until)}", until
    return True, f"tier B, held since {fmt_time(until)}", None


def judge(doc: Doc) -> tuple[list[Condition], datetime | None, bool]:
    pr = field(doc, "pr", dict)
    head = field(pr, "head", dict)
    tier = field(doc, "tier", dict)
    runs = latest_runs(field(doc, "check_runs", list))
    now = parse_time(field(doc, "now", str))
    repository = field(doc, "repository", str)
    head_repo = field(field(head, "repo", dict), "full_name", str)
    base = field(field(pr, "base", dict), "ref", str)
    mergeable = pr.get("mergeable")
    state = str(pr.get("mergeable_state"))
    threads = field(doc, "threads", list)
    unresolved = sum(1 for t in threads if isinstance(t, dict) and not field(t, "isResolved", bool))
    checks_ok, checks_detail = check_condition(runs, REQUIRED_CHECKS, absent_ok=False)
    review_ok, review_detail, rereview = review_condition(doc, runs, field(tier, "review_exempt", bool))
    tier_ok, tier_detail, hold = tier_condition(field(tier, "tier", str), runs, now)
    decision = doc.get("review_decision")
    conditions: list[Condition] = [
        ("not a draft", not field(pr, "draft", bool), "" if not pr["draft"] else "a draft never merges"),
        ("head in this repository", head_repo == repository, "" if head_repo == repository else f"head is in {head_repo}"),
        ("base is main", base == "main", "" if base == "main" else f"base is {base}: a stacked PR merges after its parent"),
        ("mergeable, no conflict", mergeable is True and state != "dirty",
         "" if mergeable is True and state != "dirty" else
         "mergeability not computed yet" if mergeable is None else f"mergeable_state is {state}"),
        ("required checks green on this head", checks_ok, checks_detail),
        ("review", review_ok, review_detail),
        ("no unresolved review threads", unresolved == 0, "" if unresolved == 0 else f"{unresolved} unresolved thread(s)"),
        ("no changes requested", decision != CHANGES_REQUESTED, "" if decision != CHANGES_REQUESTED else "a reviewer requested changes"),
        ("tier", tier_ok, tier_detail),
    ]
    return conditions, hold, rereview


def render(doc: Doc, conditions: list[Condition], ready: bool, asked: bool) -> str:
    pr = field(doc, "pr", dict)
    tier = field(doc, "tier", dict)
    mode = field(doc, "mode", str)
    sha = field(field(pr, "head", dict), "sha", str)
    effective = field(tier, "tier", str)
    reasons = "; ".join(str(r) for r in field(tier, "reasons", list))
    rows = "\n".join(f"| {name} | {'✅' if ok else '❌'} | {detail} |" for name, ok, detail in conditions)
    mark = f"<!-- rereview: {sha} -->\n" if asked else ""
    if effective == "H":
        verdict = "Tier H: the maintainer merges. The gate never labels or merges it."
    elif ready and mode == "on":
        verdict = "Ready: `merge:ready` applied and auto-merge enabled, matched to this head."
    elif ready:
        verdict = "Ready: `merge:ready` applied. Mode dry-run: auto-merge is not enabled."
    else:
        verdict = ("Not ready: fix what fails above. The gate judges again when a CI workflow completes, "
                   "on every push, label and edit, and every 30 minutes.")
    return (
        f"{MARKER}\n{mark}### Merge gate: {'ready' if ready else 'not ready'} (head `{sha[:7]}`, mode {mode}, "
        f"checked {field(doc, 'now', str)})\n\n"
        f"Tier **{effective}** (computed {field(tier, 'computed', str)}, label {tier.get('label') or 'none'}): {reasons}\n\n"
        f"| Condition | | Detail |\n|---|---|---|\n{rows}\n\n{verdict}\n"
    )


def main() -> int:
    try:
        doc = json.load(sys.stdin)
        if not isinstance(doc, dict):
            raise InputError("the document must be an object")
        if field(doc, "mode", str) not in MODES:
            raise InputError(f"mode must be one of {MODES}")
        conditions, hold, rereview = judge(doc)
        ready = all(ok for _, ok, _ in conditions)
        head = field(field(field(doc, "pr", dict), "head", dict), "sha", str)
        asked = rereview or head in asked_heads(field(doc, "comments", list))
        comment = render(doc, conditions, ready, asked)
    except (InputError, json.JSONDecodeError, ValueError) as e:
        print(f"merge-gate: {e}", file=sys.stderr)
        return 2
    json.dump({
        "ready": ready, "tier": field(field(doc, "tier", dict), "tier", str),
        "hold_until": fmt_time(hold) if hold else None, "rereview": rereview,
        "conditions": [{"name": n, "ok": ok, "detail": d} for n, ok, d in conditions],
        "comment": comment,
    }, sys.stdout)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
