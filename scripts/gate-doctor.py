#!/usr/bin/env python3
"""Check that a repository has everything the merge gate needs, and print the fix for what it lacks.

Reads, through `gh api`: every label in .github/labels.json against the repository's
labels; the repository's "Allow auto-merge" setting; the two secrets merge-gate.yml
knows its GitHub App by; the MERGE_GATE_MODE repository variable. Prints one line per
prerequisite, ok, missing or unknown, with the exact fix under each missing one; an
unknown one names the permission the caller lacks. Every call is a read: it never
changes a setting. No shell and no Unix tools, so it runs the same on Linux, macOS
and Windows. Needs Python 3.11+ and the GitHub CLI, authenticated.

Usage: python3 scripts/gate-doctor.py [--repo OWNER/NAME]   (default: the repository gh sees here)
Exit status: 0 when every prerequisite is ok, 1 when one is missing or unknown, 2 on a usage error.
Tests: scripts/tests/gate-doctor.test.sh
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LABELS = ROOT / ".github" / "labels.json"
WORKFLOW = ROOT / ".github" / "workflows" / "merge-gate.yml"
APP_ID_SECRET = "MERGE_GATE_APP_ID"
APP_KEY_SECRET = "MERGE_GATE_PRIVATE_KEY"
MODE_VARIABLE = "MERGE_GATE_MODE"
REPO = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")
HTTP_STATUS = re.compile(r"\(HTTP (\d{3})\)")
APP_PERMISSION = re.compile(r"^\s*permission-([a-z-]+):\s*(\w+)", re.MULTILINE)
BARE = re.compile(r"[A-Za-z0-9:._/=-]+")
# Characters bash, PowerShell and cmd do not all read the same inside double quotes.
UNQUOTABLE = re.compile(r"[\"`$%!]")

# state ("ok", "missing" or "unknown"), what is checked, detail, fix.
Row = tuple[str, str, str, str]
Api = Callable[[str, bool], list[object]]


class SetupError(Exception):
    """The doctor cannot run: no repository named, or the checkout's own files unreadable."""


class ApiError(Exception):
    """A `gh api` call failed; `status` is its HTTP status when gh printed one."""

    def __init__(self, status: int | None, message: str):
        super().__init__(message)
        self.status = status
        self.message = message


def run_gh(gh: str, args: list[str]) -> str:
    result = subprocess.run([gh, *args], capture_output=True, text=True, encoding="utf-8", check=False)
    if result.returncode != 0:
        lines = result.stderr.strip().splitlines()
        status = HTTP_STATUS.search(result.stderr)
        raise ApiError(int(status[1]) if status else None, lines[-1] if lines else f"gh exited {result.returncode}")
    return result.stdout


def json_values(text: str) -> list[object]:
    """Every JSON value in `text`: `gh api --paginate` prints one per page, back to back."""
    decoder = json.JSONDecoder()
    values: list[object] = []
    i = 0
    text = text.strip()
    while i < len(text):
        value, i = decoder.raw_decode(text, i)
        values.append(value)
        while i < len(text) and text[i].isspace():
            i += 1
    return values


def gh_api(gh: str) -> Api:
    def call(path: str, paginate: bool) -> list[object]:
        out = run_gh(gh, ["api", *(["--paginate"] if paginate else []), path])
        try:
            return json_values(out)
        except ValueError as e:
            raise ApiError(None, f"unreadable JSON from gh api {path}: {e}") from e
    return call


def one(api: Api, path: str) -> object:
    values = api(path, False)
    if len(values) != 1:
        raise ApiError(None, f"gh api {path} printed {len(values)} JSON values, not one")
    return values[0]


def default_repo(gh: str) -> str:
    try:
        view = json.loads(run_gh(gh, ["repo", "view", "--json", "nameWithOwner"]))
    except (ApiError, ValueError) as e:
        raise SetupError(f"no repository found here ({e}); pass --repo owner/name") from e
    name = view.get("nameWithOwner") if isinstance(view, dict) else None
    if not isinstance(name, str) or not REPO.fullmatch(name):
        raise SetupError("gh repo view named no repository; pass --repo owner/name")
    return name


def read_labels() -> list[dict[str, str]]:
    try:
        labels = json.loads(LABELS.read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        raise SetupError(f"cannot read {LABELS}: {e}") from e
    if not isinstance(labels, list) or not all(
            isinstance(label, dict) and all(isinstance(label.get(k), str) for k in ("name", "color", "description"))
            for label in labels):
        raise SetupError(f"{LABELS}: each label needs a name, color and description")
    for label in labels:
        for key in ("name", "color", "description"):
            quote(label[key])
    return sorted(labels, key=lambda label: label["name"])


def app_permissions() -> str:
    """The App permissions merge-gate.yml asks its token for, as "contents: write, …"."""
    try:
        found = APP_PERMISSION.findall(WORKFLOW.read_text(encoding="utf-8"))
    except OSError as e:
        raise SetupError(f"cannot read {WORKFLOW}: {e}") from e
    if not found:
        raise SetupError(f"{WORKFLOW} asks for no App permission")
    return ", ".join(f"{name}: {level}" for name, level in found)


def quote(text: str) -> str:
    if BARE.fullmatch(text):
        return text
    if UNQUOTABLE.search(text):
        raise SetupError(f"{text!r} has a character the shells quote differently; print a fix by hand")
    return f'"{text}"'


def unknown(error: ApiError, needs: str) -> str:
    if error.status in (401, 403, 404):
        return f"needs {needs} ({error.message})"
    return f"gh api failed: {error.message}"


def label_rows(api: Api, repo: str, wanted: list[dict[str, str]]) -> list[Row]:
    try:
        pages = api(f"repos/{repo}/labels?per_page=100", True)
        have = {label.get("name") for page in pages if isinstance(page, list) for label in page if isinstance(label, dict)}
    except ApiError as e:
        return [("unknown", f"label {label['name']}", unknown(e, f"read access to {repo}"), "") for label in wanted]
    rows: list[Row] = []
    for label in wanted:
        name = label["name"]
        if name in have:
            rows.append(("ok", f"label {name}", "", ""))
            continue
        fix = (f"gh label create {quote(name)} --repo {repo} --color {quote(label['color'])} "
               f"--description {quote(label['description'])}, or scripts/sync-labels.sh --repo {repo} for every label")
        rows.append(("missing", f"label {name}", "", fix))
    return rows


def repository(api: Api, repo: str) -> tuple[Row, bool]:
    """The auto-merge row, and whether the caller is an admin of the repository."""
    try:
        doc = one(api, f"repos/{repo}")
    except ApiError as e:
        return ("unknown", "auto-merge", unknown(e, f"read access to {repo}"), ""), False
    doc = doc if isinstance(doc, dict) else {}
    permissions = doc.get("permissions")
    admin = isinstance(permissions, dict) and permissions.get("admin") is True
    setting = doc.get("allow_auto_merge")
    if setting is True:
        return ("ok", "auto-merge", "on", ""), admin
    if setting is False:
        fix = (f"gh repo edit {repo} --enable-auto-merge, or tick \"Allow auto-merge\" under "
               f"Pull Requests at https://github.com/{repo}/settings")
        return ("missing", "auto-merge", "off", fix), admin
    return ("unknown", "auto-merge", f"needs admin access to {repo}: GitHub shows the merge settings to admins only", ""), admin


def secret_fix(name: str, repo: str, permissions: str) -> str:
    page = f"https://github.com/{repo}/settings/secrets/actions"
    if name == APP_ID_SECRET:
        return (f"create a GitHub App with {permissions}, install it on {repo}, then "
                f"gh secret set {name} --repo {repo} --body <its App ID>, or add it at {page}")
    return (f"gh secret set {name} --repo {repo} < key.pem (PowerShell: Get-Content key.pem -Raw | "
            f"gh secret set {name} --repo {repo}), with the App's private key, or paste the key at {page}")


def secret_rows(api: Api, repo: str, admin: bool, permissions: str) -> list[Row]:
    rows: list[Row] = []
    for name in sorted((APP_ID_SECRET, APP_KEY_SECRET)):
        try:
            one(api, f"repos/{repo}/actions/secrets/{name}")
            rows.append(("ok", f"secret {name}", "set", ""))
        except ApiError as e:
            # GitHub may answer 404 for a secret it hides from a non-admin: only an admin's 404 means unset.
            if e.status == 404 and admin:
                rows.append(("missing", f"secret {name}", "not set", secret_fix(name, repo, permissions)))
            else:
                rows.append(("unknown", f"secret {name}", unknown(e, f"admin access to {repo}, or a token with Secrets: read"), ""))
    return rows


def mode_row(api: Api, repo: str, admin: bool) -> Row:
    subject = f"variable {MODE_VARIABLE}"
    live = f"gh variable set {MODE_VARIABLE} --repo {repo} --body on"
    try:
        doc = one(api, f"repos/{repo}/actions/variables/{MODE_VARIABLE}")
        value = doc.get("value") if isinstance(doc, dict) else None
    except ApiError as e:
        if e.status != 404 or not admin:
            return "unknown", subject, unknown(e, f"admin access to {repo}, or a token with Variables: read"), ""
        value = None
    if value == "on":
        return "ok", subject, "on: the gate enables auto-merge", ""
    if value == "dry-run":
        return "ok", subject, f"dry-run: the gate rehearses, labelling but never enabling auto-merge; {live} makes it live", ""
    state = "unset" if value is None else "off" if value == "off" else f"{value!r}, which the gate does not read"
    fix = (f"gh variable set {MODE_VARIABLE} --repo {repo} --body dry-run to rehearse, then {live} "
           f"once its comments read right, or set it at https://github.com/{repo}/settings/variables/actions")
    return "missing", subject, f"{state}: the gate runs nothing", fix


def report(repo: str, rows: list[Row]) -> str:
    lines = [f"gate-doctor: {repo}"]
    for state, subject, detail, fix in rows:
        lines.append(f"{state:<8} {subject}" + (f": {detail}" if detail else ""))
        if fix:
            lines.append(f"{'':<8} fix: {fix}")
    counts = {state: sum(1 for row in rows if row[0] == state) for state in ("ok", "missing", "unknown")}
    lines.append(f"gate-doctor: {counts['ok']} ok, {counts['missing']} missing, {counts['unknown']} unknown")
    return "\n".join(lines)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="gate-doctor.py", description="Check the merge gate's prerequisites; changes nothing.")
    parser.add_argument("--repo", help="owner/name (default: the repository gh sees here)")
    args = parser.parse_args(argv)
    if args.repo is not None and not REPO.fullmatch(args.repo):
        parser.error(f"--repo must be owner/name, got {args.repo!r}")
    return args


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    sys.stdout.reconfigure(encoding="utf-8")
    gh = shutil.which("gh")
    try:
        if gh is None:
            raise SetupError("gh, the GitHub CLI, is required: https://cli.github.com")
        wanted = read_labels()
        permissions = app_permissions()
        repo = args.repo or default_repo(gh)
    except SetupError as e:
        print(f"gate-doctor: {e}", file=sys.stderr)
        return 2
    api = gh_api(gh)
    auto_merge, admin = repository(api, repo)
    rows = [*label_rows(api, repo, wanted), auto_merge, *secret_rows(api, repo, admin, permissions), mode_row(api, repo, admin)]
    print(report(repo, rows))
    return 0 if all(row[0] == "ok" for row in rows) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
