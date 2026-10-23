"""The repository as gen-depgraph reads it: committed files, their packages and categories, and the graph."""

import json
import os
import re
import subprocess
import tomllib
from collections import Counter, defaultdict
from pathlib import Path

LANG = {".rs": "Rust", ".ts": "TypeScript", ".py": "Python", ".sh": "Shell", ".yml": "YAML", ".yaml": "YAML",
        ".md": "Markdown", ".toml": "TOML", ".json": "JSON", ".obj": "OBJ", ".svg": "SVG"}
MANIFESTS = ("Cargo.toml", "package.json", "pyproject.toml", "requirements.txt", "go.mod", "pom.xml", "build.gradle")
CODE_EXT = (".rs", ".ts", ".py", ".sh")
# Directories of prose and data: one node per second-level folder, not per file.
GROUP_TOPS = ("docs", "specs", "fixtures", ".claude", ".specify")
# Blender's own modules ship inside bpy, which is what requirements.txt declares.
PY_ALIASES = {"bmesh": "bpy", "mathutils": "bpy"}


class GenError(Exception):
    """A file the graph cannot be built from; the message names it."""


class Repo:
    """The files git tracks under root, read as UTF-8 on demand."""

    def __init__(self, root):
        self.root = Path(root)
        out = subprocess.run(["git", "-C", str(root), "ls-files", "-z", "--eol"], capture_output=True, text=True)
        if out.returncode:
            raise GenError(f"{root}: git ls-files failed: {out.stderr.strip()}")
        self.files, self.binary, self._text = [], set(), {}
        for entry in filter(None, out.stdout.split("\0")):
            meta, path = entry.split("\t", 1)
            self.files.append(path)
            if meta.split()[0] == "i/-text":
                self.binary.add(path)
        self.fileset = set(self.files)

    def read(self, path):
        """The file's text; a binary file has none."""
        if path in self.binary:
            return ""
        if path not in self._text:
            try:
                self._text[path] = (self.root / path).read_text(encoding="utf-8")
            except (OSError, UnicodeDecodeError) as e:
                raise GenError(f"{path}: {e}") from e
        return self._text[path]


class Graph:
    """Nodes by id and (source, target, kind) edges; an edge needs both ends."""

    def __init__(self):
        self.nodes, self.edges = {}, set()

    def node(self, nid, **fields):
        self.nodes.setdefault(nid, {"id": nid, **fields})

    def edge(self, source, target, kind):
        if source != target and source in self.nodes and target in self.nodes:
            self.edges.add((source, target, kind))

    def external(self, registry, lang, name):
        nid = f"ext:{registry}:{name}"
        self.node(nid, label=name, path=f"{registry}/{name}", pkg=f"ext:{registry}", cat="External",
                  lang=lang, loc=0, kind="external")
        return nid


def lang_of(path):
    return LANG.get(os.path.splitext(path)[1], "Other")


def loc(text):
    return sum(1 for line in text.splitlines() if line.strip())


def parse_cargo(repo):
    """Each Cargo package by directory: its name and dependencies by the identifier its source uses."""
    pkgs = {}
    for f in repo.files:
        if os.path.basename(f) != "Cargo.toml":
            continue
        try:
            data = tomllib.loads(repo.read(f))
        except tomllib.TOMLDecodeError as e:
            raise GenError(f"{f}: {e}") from e
        if "package" not in data:
            continue
        deps = {}
        for table in [data, *(data.get("target") or {}).values()]:
            for section, kind in (("dependencies", "normal"), ("build-dependencies", "build"), ("dev-dependencies", "dev")):
                for name, spec in (table.get(section) or {}).items():
                    real = spec.get("package", name) if isinstance(spec, dict) else name
                    # Source names a dependency by its key; `package` renames only the crate it resolves to.
                    deps.setdefault(name.replace("-", "_"), (real, kind))
        pkgs[os.path.dirname(f)] = {"name": data["package"]["name"], "deps": deps}
    return pkgs


def project_name(repo):
    """The name the page shows: the root package.json's, else the Cargo workspace repository's last segment.

    Never the checkout directory's name, so the same commit gives the same bytes wherever it is checked out.
    """
    if "package.json" in repo.fileset:
        try:
            name = json.loads(repo.read("package.json")).get("name")
        except json.JSONDecodeError as e:
            raise GenError(f"package.json: {e}") from e
        if isinstance(name, str) and name:
            return name.rsplit("/", 1)[-1]
    if "Cargo.toml" in repo.fileset:
        try:
            workspace = tomllib.loads(repo.read("Cargo.toml")).get("workspace", {})
        except tomllib.TOMLDecodeError as e:
            raise GenError(f"Cargo.toml: {e}") from e
        url = workspace.get("package", {}).get("repository")
        if isinstance(url, str) and url.rstrip("/"):
            return url.rstrip("/").rsplit("/", 1)[-1]
    return "repository"


class Layout:
    """Which package (level 2) and category (level 1) each file belongs to."""

    def __init__(self, repo, cargo):
        self.repo, self.cargo = repo, cargo

    def pkg(self, path):
        parts, d = path.split("/"), os.path.dirname(path)
        while d:
            if d in self.cargo or d + "/package.json" in self.repo.fileset:
                return d
            d = os.path.dirname(d)
        if parts[0] in ("research", "scripts", ".github") + GROUP_TOPS and len(parts) > 2:
            return "/".join(parts[:2])
        return parts[0] if len(parts) > 1 else "(root)"

    @staticmethod
    def is_test(path):
        b = os.path.basename(path)
        return ("/tests/" in "/" + path or b.startswith("test_") or ".test." in b or b == "tests.rs"
                or b == "test-setup.ts" or b.startswith("mock-") or path.startswith("fixtures/")
                or "-testkit/" in path)

    def category(self, path):
        top = path.split("/")[0]
        if self.is_test(path):
            return "Tests"
        return {"apps": "Frontend", "packages": "Frontend", "bindings": "Backend", "crates": "Core",
                "research": "Research", "docs": "Docs", "specs": "Docs"}.get(top, "Tools")

    def label(self, path):
        b = os.path.basename(path)
        if b in ("lib.rs", "main.rs"):
            return os.path.basename(self.pkg(path)).removeprefix("openplush-") + "::" + b[:-3]
        if b in ("mod.rs", "tests.rs", "index.ts", "__init__.py"):
            return os.path.basename(os.path.dirname(path)) + "/" + b
        return b


def add_nodes(repo, layout, graph):
    """A node per code file and workflow, and one per prose or data folder."""
    groups = defaultdict(list)
    for f in repo.files:
        if f.endswith(CODE_EXT) or (f.startswith(".github/workflows/") and f.endswith(".yml")):
            graph.node(f, label=layout.label(f), path=f, pkg=layout.pkg(f), cat=layout.category(f),
                       lang=lang_of(f), loc=loc(repo.read(f)), kind="file")
        elif f.split("/")[0] in GROUP_TOPS:
            parts = f.split("/")
            groups["/".join(parts[:2]) if len(parts) > 2 else parts[0]].append(f)
    for g, fs in groups.items():
        langs = Counter(lang_of(f) for f in fs)
        graph.node(g + "/", label=g.split("/")[-1] + "/", path=g + "/", pkg=g, cat=layout.category(g + "/x"),
                   lang=langs.most_common(1)[0][0], loc=sum(loc(repo.read(f)) for f in fs), kind="group",
                   files=len(fs))
    return groups


def add_ts_edges(repo, graph):
    """TypeScript imports, ts-rs generated types back to their Rust source, and package.json devDependencies."""
    ts_types = {}
    for f in sorted(n for n in graph.nodes if n.endswith(".rs")):
        text = repo.read(f)
        if "ts(export" in text:
            for name in re.findall(r"pub\s+(?:struct|enum|type)\s+(\w+)", text):
                ts_types.setdefault(name, f)
    for f in [n for n in graph.nodes if n.endswith(".ts")]:
        text = repo.read(f)
        specs = re.findall(r"""(?:from|import)\s*\(?\s*["']([^"']+)["']""", text)
        specs += re.findall(r"""new URL\(\s*["']([^"']+)["']""", text)
        for spec in specs:
            if spec.startswith("."):
                base = os.path.normpath(os.path.join(os.path.dirname(f), spec))
                target = next((c for c in (base, base + ".ts", base + "/index.ts") if c in graph.nodes), None)
                if target:
                    graph.edge(f, target, "import")
            else:
                name = "/".join(spec.split("/")[:2]) if spec.startswith("@") else spec.split("/")[0]
                graph.edge(f, graph.external("npm", "TypeScript", name), "import")
        stem = os.path.splitext(os.path.basename(f))[0]
        if "/generated/" in f and "ts-rs" in text and stem in ts_types:
            graph.edge(f, ts_types[stem], "codegen")
    for f in repo.files:
        if os.path.basename(f) == "package.json":
            add_npm_manifest(repo, graph, f)


def add_npm_manifest(repo, graph, manifest):
    try:
        pj = json.loads(repo.read(manifest))
    except json.JSONDecodeError as e:
        raise GenError(f"{manifest}: {e}") from e
    exports = pj.get("exports", {})
    entry = exports.get(".") if isinstance(exports, dict) else exports
    if not isinstance(entry, str):
        return
    entry = os.path.normpath(os.path.join(os.path.dirname(manifest), entry))
    if entry not in graph.nodes:
        return
    pkg = graph.nodes[entry]["pkg"]
    for section in ("dependencies", "devDependencies"):
        for name in pj.get(section, {}):
            target = graph.external("npm", "TypeScript", name)
            if not any(t == target and graph.nodes[s]["pkg"] == pkg for s, t, _ in graph.edges):
                graph.edge(entry, target, "manifest")


def declared_python(repo, path):
    """Import names the nearest requirements.txt declares; the rest are local or the standard library."""
    d = os.path.dirname(path)
    while True:
        req = (d + "/" if d else "") + "requirements.txt"
        if req in repo.fileset:
            names = set()
            for line in repo.read(req).splitlines():
                name = re.split(r"[=<>~!\[;#\s]", line.strip(), maxsplit=1)[0]
                if name:
                    names.add(name.lower().replace("-", "_"))
            return names
        if not d:
            return set()
        d = os.path.dirname(d)


def add_py_edges(repo, graph):
    for f in [n for n in graph.nodes if n.endswith(".py")]:
        text, d, declared = repo.read(f), os.path.dirname(f), declared_python(repo, f)
        mods = re.findall(r"^\s*from\s+([.\w]+)\s+import", text, re.M)
        for line in re.findall(r"^\s*import\s+([\w., ]+)", text, re.M):
            mods += [m.strip().split(" ")[0] for m in line.split(",")]
        for m in mods:
            top = m.lstrip(".").split(".")[0]
            local = os.path.join(d, top + ".py")
            if local in graph.nodes:
                graph.edge(f, local, "import")
            elif PY_ALIASES.get(top, top).lower() in declared:
                graph.edge(f, graph.external("pypi", "Python", PY_ALIASES.get(top, top)), "import")


def add_ref_edges(repo, graph, groups, entry_points):
    """Paths named in scripts, workflows, docs and agent config; fixture folders named in code."""
    scripts = [n for n in graph.nodes if n.startswith("scripts/") and "/tests/" not in n and graph.nodes[n]["kind"] == "file"]
    names = Counter(os.path.basename(s) for s in scripts)
    fixtures = [g for g in graph.nodes if g.startswith("fixtures/") and g.count("/") == 2]
    for src, n in list(graph.nodes.items()):
        if n["kind"] == "external" or src.startswith("fixtures/"):
            continue
        text = "\n".join(repo.read(f) for f in groups[src.rstrip("/")]) if n["kind"] == "group" else repo.read(src)
        for g in fixtures:
            if g.rstrip("/") in text or (src.endswith("testkit/src/fixtures.rs") and f'"{g.split("/")[1]}"' in text):
                graph.edge(src, g, "data")
        if n["lang"] in ("Rust", "TypeScript"):
            continue
        for s in scripts:
            b = os.path.basename(s)
            if s in text or (names[b] == 1 and re.search(r"(?<![\w.-])" + re.escape(b) + r"\b", text)):
                graph.edge(src, s, "ref")
        for d, entry in entry_points.items():
            if re.search(re.escape(d) + r"\b", text) or re.search(r"-p\s+" + re.escape(os.path.basename(d)) + r"\b", text):
                graph.edge(src, entry, "ref")
