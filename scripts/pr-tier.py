#!/usr/bin/env python3
"""Compute a pull request's merge tier (A, B or C) from its diff, by ADR 0014.

Reads one JSON document on stdin:
  {"title": str, "head_ref": str, "labels": [str], "changed_files": int,
   "files": [{"filename": str, "status": str, "previous_filename"?: str, "patch"?: str}]}
(the files are GitHub's pulls/N/files entries) and prints one:
  {"tier": "A"|"B"|"C", "computed": ..., "label": "A"|"B"|"C"|null,
   "relabel": bool, "review_exempt": bool, "reasons": [str]}
`tier` is the higher of the computed tier and any tier: label; `relabel` says
the label must be set to it. `review_exempt` is whether review.yml skips the
PR by its own rules. Needs Python 3.11+. Tests: scripts/tests/pr-tier.test.sh
"""

import json
import re
import sys
from typing import TypeVar
from pathlib import PurePosixPath

C_DIRS = ("crates/", "specs/", ".specify/", "docs/adr/", ".github/", ".claude/", "scripts/", "bindings/")
C_NAMES = frozenset({
    "AGENTS.md", "CLAUDE.md", "GOVERNANCE.md", "CODE_OF_CONDUCT.md",
    "Cargo.toml", "Cargo.lock", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml",
    "deny.toml", "rust-toolchain.toml",
})
EXPECTED_OUTPUT_SUFFIXES = (".svg", ".pdf")
TICK_TITLE = "docs(tasks): tick"
TASKS_FILE = re.compile(r"specs/[^/]+/tasks\.md")
UNTICKED = re.compile(r"^(?P<indent>\s*)- \[ \] (?P<text>.*)$")
TICKED = re.compile(r"^(?P<indent>\s*)- \[x\] (?P<text>.*)$")
FILE_CAP = 3000
RANK = {"A": 0, "B": 1, "C": 2}

T = TypeVar("T")
Tier = str
File = dict[str, object]


class InputError(Exception):
    """The input document is not what the gate sends."""


def field(obj: dict[str, object], name: str, kind: type[T]) -> T:
    value = obj.get(name)
    if not isinstance(value, kind):
        raise InputError(f"'{name}' must be {kind.__name__}, got {type(value).__name__}")
    return value


def path_tier(path: str) -> tuple[Tier, str]:
    """The tier one changed path reaches, with the rule that puts it there."""
    parts = PurePosixPath(path).parts
    if path.startswith(C_DIRS):
        return "C", f"{path}: under a tier C directory"
    if ".cargo" in parts:
        return "C", f"{path}: under .cargo/"
    if parts and parts[-1] in C_NAMES:
        return "C", f"{path}: a rules file, manifest, lockfile or toolchain file"
    if path.startswith("fixtures/golden/"):
        return "B", f"{path}: expected output under fixtures/golden/"
    if path.endswith(".snap"):
        return "B", f"{path}: a snapshot"
    if path.startswith("fixtures/") and path.endswith(EXPECTED_OUTPUT_SUFFIXES):
        return "B", f"{path}: an expected SVG or PDF fixture"
    return "A", ""


def file_paths(file: File) -> list[str]:
    """A rename counts both its names."""
    paths = [field(file, "filename", str)]
    previous = file.get("previous_filename")
    if isinstance(previous, str):
        paths.append(previous)
    return paths


def tick_pairs(patch: str) -> list[tuple[str, str]]:
    """Pair each removed line with the added line that replaces it, hunk by hunk."""
    pairs: list[tuple[str, str]] = []
    removed: list[str] = []
    added: list[str] = []

    def flush() -> None:
        if len(removed) != len(added):
            raise ValueError(f"{len(removed)} lines removed but {len(added)} added")
        pairs.extend(zip(removed, added, strict=True))
        removed.clear()
        added.clear()

    for line in patch.splitlines():
        if line.startswith("@@"):
            flush()
        elif line.startswith("-"):
            removed.append(line[1:])
        elif line.startswith("+"):
            added.append(line[1:])
        elif line.startswith("\\"):
            continue
        else:
            flush()
    flush()
    return pairs


def not_a_tick(old: str, new: str) -> str | None:
    """Why this changed line is not a tick with an as-built note, or None if it is."""
    before = UNTICKED.match(old)
    after = TICKED.match(new)
    if before is None or after is None:
        return "not a '- [ ]' to '- [x]' flip"
    if before["indent"] != after["indent"] or not after["text"].startswith(before["text"]):
        return "the task text changed"
    if len(after["text"]) == len(before["text"]):
        return "no as-built note appended"
    if "human:" in before["text"]:
        return "a human: task is the maintainer's to tick"
    return None


def file_tick_failure(file: File) -> str | None:
    name = field(file, "filename", str)
    if TASKS_FILE.fullmatch(name) is None or file.get("status") != "modified":
        return f"{name}: not a modified specs/*/tasks.md"
    patch = file.get("patch")
    if not isinstance(patch, str):
        return f"{name}: no patch to read (too large?)"
    try:
        pairs = tick_pairs(patch)
    except ValueError as e:
        return f"{name}: {e}"
    for old, new in pairs:
        reason = not_a_tick(old, new)
        if reason is not None:
            return f"{name}: {reason}: {new.strip()[:80]}"
    return None


def tick_failure(title: str, files: list[File]) -> str | None:
    """Why the tick exception does not apply, or None if the PR is a pure tick."""
    if not title.startswith(TICK_TITLE):
        return f"title does not start with '{TICK_TITLE}'"
    if not files:
        return "no files changed"
    for file in files:
        reason = file_tick_failure(file)
        if reason is not None:
            return reason
    return None


def compute(title: str, changed_files: int, files: list[File]) -> tuple[Tier, list[str]]:
    if changed_files > FILE_CAP or changed_files > len(files):
        return "C", [f"file listing incomplete: {changed_files} changed, {len(files)} listed, cap {FILE_CAP}"]
    tick = tick_failure(title, files)
    if tick is None:
        return "A", ["a tick PR: only '- [ ]' to '- [x]' flips with as-built notes in specs/*/tasks.md"]
    tier: Tier = "A"
    reasons: list[str] = []
    for file in files:
        for path in file_paths(file):
            path_t, reason = path_tier(path)
            if path_t != "A":
                reasons.append(reason)
            if RANK[path_t] > RANK[tier]:
                tier = path_t
    if tier == "C" and title.startswith(TICK_TITLE):
        reasons.insert(0, f"not a tick PR: {tick}")
    if tier == "A":
        reasons.append("no path reaches tier B or C")
    return tier, reasons


def label_tier(labels: list[str]) -> Tier | None:
    tiers = [label[len("tier:"):] for label in labels if label in ("tier:A", "tier:B", "tier:C")]
    return max(tiers, key=RANK.__getitem__) if tiers else None


def review_exempt(title: str, head_ref: str, files: list[File]) -> bool:
    docs_only = bool(files) and all(field(f, "filename", str).startswith("docs/") for f in files)
    return title.startswith(TICK_TITLE) or head_ref.startswith("chore/") or docs_only


def main() -> int:
    try:
        doc = json.load(sys.stdin)
    except json.JSONDecodeError as e:
        print(f"pr-tier: stdin is not JSON: {e}", file=sys.stderr)
        return 2
    try:
        if not isinstance(doc, dict):
            raise InputError("the document must be an object")
        title = field(doc, "title", str)
        head_ref = field(doc, "head_ref", str)
        labels = [label for label in field(doc, "labels", list) if isinstance(label, str)]
        changed_files = field(doc, "changed_files", int)
        files = [f for f in field(doc, "files", list) if isinstance(f, dict)]
        computed, reasons = compute(title, changed_files, files)
        label = label_tier(labels)
        exempt = review_exempt(title, head_ref, files)
    except InputError as e:
        print(f"pr-tier: {e}", file=sys.stderr)
        return 2
    tier = computed if label is None or RANK[computed] >= RANK[label] else label
    json.dump({
        "tier": tier, "computed": computed, "label": label,
        "relabel": label != tier, "review_exempt": exempt, "reasons": reasons,
    }, sys.stdout)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
