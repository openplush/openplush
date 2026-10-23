"""Rust edges: `mod` declarations, `use` and qualified paths, and Cargo dependencies no source names."""

import os
import re
from collections import defaultdict

MOD_DECL = re.compile(r"(#\[cfg\(test\)\]\s*)?(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?\bmod\s+(\w+)\s*;")
INLINE_TEST = re.compile(r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{")
# A raw, plain or char literal, or a comment, matched left to right so none hides another.
LEXEME = re.compile(r'b?r(#*)".*?"\1|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])\'|//[^\n]*|/\*.*?\*/', re.S)
PATH = re.compile(r"\b((?:crate|super|self|[a-z_][a-z0-9_]*)(?:::[A-Za-z_][A-Za-z0-9_]*)+)")


def strip_comments(text):
    """The code with comments removed and string literals emptied, so neither adds, drops or nests anything."""
    return LEXEME.sub(lambda m: "''" if m.group(0)[0] == "'" else "" if m.group(0)[0] == "/" else '""', text)


def split_inline_tests(text):
    """(production text, test text): each `#[cfg(test)] mod x { .. }` block moves to the test text."""
    prod, test, i = [], [], 0
    for m in INLINE_TEST.finditer(text):
        if m.start() < i:
            continue
        depth, j = 1, m.end()
        while j < len(text) and depth:
            depth += {"{": 1, "}": -1}.get(text[j], 0)
            j += 1
        prod.append(text[i:m.start()])
        test.append(text[m.start():j])
        i = j
    prod.append(text[i:])
    return "".join(prod), "".join(test)


def expand_use(tree):
    """`a::{b, c::{d}}` as its paths `a::b` and `a::c::d`."""
    tree = re.sub(r"\s+", "", tree)
    m = re.match(r"^(.*?)\{(.*)\}$", tree)
    if not m:
        return [tree]
    prefix, inner = m.groups()
    out, depth, cur = [], 0, ""
    for ch in inner:
        depth += {"{": 1, "}": -1}.get(ch, 0)
        if ch == "," and depth == 0:
            out += [prefix + p for p in expand_use(cur)]
            cur = ""
        else:
            cur += ch
    return out + ([prefix + p for p in expand_use(cur)] if cur else [])


def resolve(mods, segs):
    """The deepest module the path names; the rest of the path is an item inside it."""
    for i in range(len(segs), -1, -1):
        if "::".join(segs[:i]) in mods:
            return mods["::".join(segs[:i])]
    return None


class Crates:
    """Module trees of every Cargo package, followed from each crate root through its `mod` declarations."""

    def __init__(self, repo, graph, cargo):
        self.repo, self.graph, self.cargo = repo, graph, cargo
        self.file_mod, self.mods, self.test_ctx = {}, defaultdict(dict), set()
        self.dir_of, self.lib = {}, {}
        for d, info in sorted(cargo.items()):
            ident = info["name"].replace("-", "_")
            self.dir_of[ident] = d
            tops = (d + "/src/bin", d + "/tests", d + "/examples", d + "/benches")
            roots = [d + "/src/lib.rs", d + "/src/main.rs"]
            roots += sorted(f for f in graph.nodes if f.endswith(".rs") and os.path.dirname(f) in tops)
            for r in roots:
                if r in graph.nodes:
                    self._walk(r, r, [], True, "/src/" not in r)
            info["root"] = next((r for r in roots[:2] if r in graph.nodes), None)
            self.lib[ident] = d + "/src/lib.rs" if d + "/src/lib.rs" in graph.nodes else info["root"]

    def _walk(self, f, root, segs, is_root, in_test):
        if f in self.file_mod:
            return
        self.file_mod[f] = (root, segs)
        self.mods[root]["::".join(segs)] = f
        if in_test:
            self.test_ctx.add(f)
        d = os.path.dirname(f) if is_root or f.endswith("/mod.rs") else f[:-3]
        for m in MOD_DECL.finditer(strip_comments(self.repo.read(f))):
            for child in (f"{d}/{m.group(2)}.rs", f"{d}/{m.group(2)}/mod.rs"):
                if child in self.graph.nodes:
                    self.graph.edge(f, child, "test" if m.group(1) else "mod")
                    self._walk(child, root, segs + [m.group(2)], False, in_test or bool(m.group(1)))
                    break

    def targets(self, f, text, inline):
        """(target, kind) for every path the text names; `inline` text sits in a test module inside f."""
        info = self.cargo[self.graph.nodes[f]["pkg"]]
        own = info["name"].replace("-", "_")
        root, here = self.file_mod.get(f, (None, []))
        paths = [p for u in re.findall(r"\buse\s+([^;]+);", text, re.S) for p in expand_use(u)]
        for segs in (p.split("::") for p in paths + PATH.findall(text)):
            segs = [s for s in segs if s]
            if not segs:
                continue
            head = segs[0]
            if head == "crate" and root:
                yield resolve(self.mods[root], segs[1:]), "use"
            elif head in ("super", "self") and root:
                cur = here + ([""] if inline else [])  # inside an inline module, `super` is this file
                while segs and segs[0] in ("super", "self"):
                    if segs.pop(0) == "super" and cur:
                        cur = cur[:-1]
                yield resolve(self.mods[root], [s for s in cur if s] + segs), "use"
            elif head == own and root != self.lib[own]:
                yield resolve(self.mods[self.lib[own]], segs[1:]), "use"
            elif head in info["deps"]:
                yield self._dependency(info["deps"][head], segs)

    def _dependency(self, dep, segs):
        real, kind = dep
        crate = real.replace("-", "_")
        if crate in self.dir_of:
            target = resolve(self.mods[self.lib[crate]], segs[1:]) or self.lib[crate]
        else:
            target = self.graph.external("crates.io", "Rust", real)
        return target, "dev" if kind == "dev" else "use"


def add_rust_edges(repo, graph, cargo):
    """Every Rust file's edges; returns each package's entry file for path references."""
    crates = Crates(repo, graph, cargo)
    for f in [n for n in graph.nodes if n.endswith(".rs") and graph.nodes[n]["pkg"] in cargo]:
        prod, test = split_inline_tests(strip_comments(repo.read(f)))
        for text, inline in ((prod, False), (test, True)):
            for target, kind in crates.targets(f, text, inline):
                in_test = (inline or f in crates.test_ctx) and graph.nodes[f]["cat"] != "Tests"
                if target:
                    graph.edge(f, target, "test" if in_test and kind == "use" else kind)
    add_unnamed_dependencies(graph, cargo, crates)
    return crates


def add_unnamed_dependencies(graph, cargo, crates):
    """A declared dependency no source names (a probe's wasm-bindgen) still shows, from the package root."""
    for d, info in sorted(cargo.items()):
        root = info["root"]
        if not root:
            continue
        for real, kind in info["deps"].values():
            crate = real.replace("-", "_")
            target_pkg = crates.dir_of.get(crate, "ext:crates.io")
            target = crates.lib.get(crate) or graph.external("crates.io", "Rust", real)
            named = any(graph.nodes[s]["pkg"] == d and (t == target or crate in crates.dir_of and graph.nodes[t]["pkg"] == target_pkg)
                        for s, t, _ in graph.edges)
            if not named:
                graph.edge(root, target, "dev" if kind == "dev" else "manifest")
