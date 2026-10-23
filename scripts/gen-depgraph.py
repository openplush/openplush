#!/usr/bin/env python3
"""Generate the docs site's interactive dependency graph of the repository.

Reads the files git tracks and writes one self-contained page, so it can never go
stale and is never committed. The graph has three levels: subsystems (each file's
category), packages (Cargo packages, npm packages and second-level folders) and
files. Edges come from what the code names:
  - Rust: `mod` declarations followed from every crate root, `use` and qualified
    paths, and Cargo dependencies no source names; a dev-dependency is a `dev` edge
    and a path inside an inline `#[cfg(test)]` module a `test` edge;
  - TypeScript imports, and each ts-rs generated type back to the Rust type it is
    generated from;
  - Python imports of a local file, or of a name the nearest requirements.txt
    declares; any other import is the standard library and is left out;
  - path references in scripts, workflows, docs and agent configuration, and
    fixture folders named in code.
Depth and cycles read production imports only. D3 is vendored beside this script
and inlined, and its licence is written beside the page, so the page loads nothing
from the network. The output is byte-identical for the same commit.
It fails, writing nothing, naming the file, on a manifest that does not parse or
has the wrong shape, or a tracked file that is missing or not UTF-8.

Usage: scripts/gen-depgraph.py [root] [out]
       (default root: the repository; default out: <root>/target/site/dev/depgraph)
Tests: scripts/tests/gen-depgraph.test.sh
"""

import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

ASSETS = Path(__file__).resolve().parent / "depgraph"
sys.path.insert(0, str(ASSETS))
sys.dont_write_bytecode = True

from levels import levels  # noqa: E402
from repo import MANIFESTS, GenError, Graph, Layout, Repo, add_nodes, add_py_edges, add_ref_edges, add_ts_edges, parse_cargo, project_name  # noqa: E402
from rust import add_rust_edges  # noqa: E402


def build(root):
    """The page's data for the repository at root."""
    repo = Repo(root)
    cargo = parse_cargo(repo)
    layout, graph = Layout(repo, cargo), Graph()
    groups = add_nodes(repo, layout, graph)
    crates = add_rust_edges(repo, graph, cargo)
    add_ts_edges(repo, graph)
    add_py_edges(repo, graph)
    entry_points = {d: info["root"] for d, info in sorted(cargo.items()) if info["root"]}
    for f in repo.files:
        if os.path.basename(f) == "package.json" and os.path.dirname(f) + "/src/index.ts" in graph.nodes:
            entry_points[os.path.dirname(f)] = os.path.dirname(f) + "/src/index.ts"
    add_ref_edges(repo, graph, groups, entry_points)
    roots = {info["root"] for info in cargo.values()} | set(crates.lib.values())
    manifests = [f for f in repo.files if os.path.basename(f) in MANIFESTS]
    data = levels(graph, cargo, roots, manifests)
    data["project"] = project_name(repo)
    return data


def page(data):
    """The template with D3 and the data inlined; `</` is escaped so neither can close its script."""
    d3 = (ASSETS / "d3.min.js").read_text(encoding="utf-8")
    payload = json.dumps(data, separators=(",", ":"), ensure_ascii=False).replace("</", "<\\/")
    if "</script" in d3.lower():
        raise GenError(f"{ASSETS / 'd3.min.js'}: contains </script, which would end the inlined script")
    template = (ASSETS / "template.html").read_text(encoding="utf-8")
    return template.replace("__D3__", d3, 1).replace("__DATA__", payload, 1)


def main(argv):
    root = Path(argv[1]) if len(argv) > 1 else Path(__file__).resolve().parent.parent
    out = Path(argv[2]) if len(argv) > 2 else root / "target/site/dev/depgraph"
    try:
        html = page(build(root))
    except GenError as e:
        print(f"gen-depgraph: {e}", file=sys.stderr)
        return 1
    out.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".depgraph-", dir=out.parent))
    staging.chmod(0o755)
    (staging / "index.html").write_text(html, encoding="utf-8")
    shutil.copyfile(ASSETS / "LICENSE-d3.txt", staging / "LICENSE-d3.txt")
    if out.exists():
        shutil.rmtree(out)
    shutil.move(str(staging), out)
    print(f"gen-depgraph: wrote {out}/index.html")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
