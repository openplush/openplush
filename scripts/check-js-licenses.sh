#!/usr/bin/env bash
# JS license gate: Constitution IX's allowlist, for npm as cargo deny is for crates.
#
# Every installed npm package, dev dependencies included, must declare in its own
# package.json a license expression that deny.toml's [licenses] allow satisfies:
#   - `A OR B` passes if any alternative is an allowed ID;
#   - `A AND B` passes only if every part is;
#   - `X WITH exception` is never itself allowed, so it passes only as an OR
#     alternative to an allowed ID, as under cargo deny;
#   - IDs compare case-sensitively; one pair of outer parentheses is stripped.
# Fails on a missing license, `SEE LICENSE IN …`, `UNLICENSED`, a `licenses`
# array of more than one entry, and any expression mixing AND with OR: stricter
# than cargo deny, deliberately, until a real dependency needs one.
#
# pnpm lists the packages but never judges them: when the license field is
# missing, pnpm 10 guesses one from the LICENSE text and does not mark the guess.
#
# Usage: scripts/check-js-licenses.sh [root]   (default: the repository)
# Tests: scripts/tests/check-js-licenses.test.sh
# Needs: bash, python3 >= 3.11, and pnpm once the lockfile has packages.
set -euo pipefail

root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$root"

die() {
  echo "check-js-licenses: $*" >&2
  exit 1
}

# One allowed ID per line; fails if deny.toml has none.
allow=$(python3 -c 'import sys, tomllib
allow = tomllib.load(open("deny.toml", "rb")).get("licenses", {}).get("allow") or []
if not allow or not all(isinstance(a, str) for a in allow):
    sys.exit("check-js-licenses: deny.toml [licenses] allow is empty, missing, or not a list of IDs")
print("\n".join(allow))') || exit 1

[[ -f pnpm-lock.yaml ]] || die "$root/pnpm-lock.yaml is missing"
if ! grep -q '^packages:' pnpm-lock.yaml; then
  echo "check-js-licenses: no npm dependencies"
  exit 0
fi

listing=$(pnpm licenses list --json) || die "pnpm licenses list failed"

# The judge reads the listing on stdin and its own source on fd 3.
python3 /dev/fd/3 "$allow" <<<"$listing" 3<<'PY'
import json, sys

def declared(meta):
    """(expression, None), or (what was declared, why it is not an expression)."""
    lic, lics = meta.get("license"), meta.get("licenses")
    if isinstance(lic, dict):
        lic = lic.get("type")
    if lic is None and isinstance(lics, list):
        if len(lics) != 1:
            return f"licenses: {len(lics)} entries", "declare one expression"
        lic = lics[0].get("type") if isinstance(lics[0], dict) else None
    if not isinstance(lic, str) or not lic.strip():
        return "(none)", "no license declared"
    lic = lic.strip()
    if lic.startswith("SEE LICENSE IN") or lic == "UNLICENSED":
        return lic, "not an SPDX expression"
    return lic, None

def strip_outer(expr):
    """expr without one pair of parentheses enclosing all of it."""
    depth = 0
    for i, c in enumerate(expr):
        depth += {"(": 1, ")": -1}.get(c, 0)
        if depth == 0:
            return expr[1:-1].strip() if i == len(expr) - 1 and i > 0 else expr
    return expr

def verdict(expr, allow):
    """None if the expression passes, else why it fails."""
    words = strip_outer(expr).split()
    if "AND" in words and "OR" in words:
        return "mixes AND and OR"
    op = "AND" if "AND" in words else "OR"
    terms = " ".join(words).split(f" {op} ")
    if any("(" in t or ")" in t or len(t.split()) not in (1, 3) or
           (len(t.split()) == 3 and t.split()[1] != "WITH") for t in terms):
        return "not a flat SPDX expression"
    ok = [t in allow for t in terms]
    if op == "AND" and not all(ok):
        return "every part must be allowed, and WITH never is"
    if not any(ok):
        return "not allowed" if len(terms) == 1 else "no alternative is allowed"
    return None

def check(name, path, version, allow):
    try:
        with open(f"{path}/package.json", encoding="utf-8") as f:
            meta = json.load(f)
    except (OSError, ValueError) as e:
        return f"{name}@{version}: unreadable package.json ({type(e).__name__})"
    lic, why = declared(meta)
    why = why or verdict(lic, allow)
    return f"{name}@{version}: {lic} ({why})" if why else None

def main():
    allow = set(sys.argv[1].splitlines())
    try:
        listing = json.load(sys.stdin)
    except ValueError:
        sys.exit("check-js-licenses: pnpm printed no JSON, but pnpm-lock.yaml has packages")
    seen, bad = set(), []
    for e in (e for entries in listing.values() for e in entries):
        paths, versions = e.get("paths") or [], e.get("versions") or []
        if not paths or len(paths) != len(versions):
            bad.append(f"{e.get('name')}: listed without one path per version")
        for path, version in zip(paths, versions):
            if path not in seen:
                seen.add(path)
                bad.append(check(e.get("name"), path, version, allow))
    bad = sorted(b for b in bad if b)
    if bad:
        print("check-js-licenses: declared license not allowed by deny.toml:", file=sys.stderr)
        print("\n".join(f"  {b}" for b in bad), file=sys.stderr)
        sys.exit(1)
    n = len(seen)
    print(f"check-js-licenses: ok ({n} package{'' if n == 1 else 's'})")

main()
PY
