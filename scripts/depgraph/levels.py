"""Degrees, dependency depth and cycles, and the package and subsystem levels built from the file graph."""

import os
from collections import Counter, defaultdict

CATEGORIES = ["Frontend", "Backend", "Core", "Database", "Tests", "Tools", "Research", "Docs", "External"]
PRODUCTION = ("use", "import", "codegen", "manifest")


def components(nodes, adj):
    """Strongly connected components (iterative Tarjan), as a map from node to its component's id."""
    index, low, on_stack, stack, comp, counter = {}, {}, set(), [], {}, 0
    for start in nodes:
        if start in index:
            continue
        work = [(start, iter(sorted(adj[start])))]
        index[start] = low[start] = counter
        counter += 1
        stack.append(start)
        on_stack.add(start)
        while work:
            v, children = work[-1]
            child = next(children, None)
            if child is None:
                work.pop()
                if work:
                    low[work[-1][0]] = min(low[work[-1][0]], low[v])
                if low[v] == index[v]:
                    while True:
                        w = stack.pop()
                        on_stack.discard(w)
                        comp[w] = v
                        if w == v:
                            break
            elif child not in index:
                index[child] = low[child] = counter
                counter += 1
                stack.append(child)
                on_stack.add(child)
                work.append((child, iter(sorted(adj[child]))))
            elif child in on_stack:
                low[v] = min(low[v], index[child])
    return comp


def annotate(graph, roots):
    """Each node's in and out degree and dependency depth; returns the import cycles.

    Depth and cycles read production imports only: `mod` declarations and a file's use of
    its own crate root (a re-export) would otherwise fold every crate into one cycle.
    """
    edges = sorted(graph.edges)
    nodes = graph.nodes

    def production(a, b, k):
        return k in PRODUCTION and not (b in roots and nodes[a]["pkg"] == nodes[b]["pkg"])

    adj = defaultdict(set)
    for a, b, k in edges:
        if production(a, b, k):
            adj[a].add(b)
    comp = components(list(nodes), adj)
    below = defaultdict(set)
    for a, b in ((a, b) for a, b, k in edges if production(a, b, k) and comp[a] != comp[b]):
        below[comp[a]].add(comp[b])
    depth = {}
    for c in sorted(set(comp.values())):
        depth_of(c, below, depth)
    indeg, outdeg = Counter(b for _, b, _ in edges), Counter(a for a, _, _ in edges)
    for v, n in nodes.items():
        n.update({"in": indeg[v], "out": outdeg[v], "depth": depth[comp[v]]})
    members = defaultdict(list)
    for v in nodes:
        members[comp[v]].append(v)
    return sorted(sorted(m) for m in members.values() if len(m) > 1)


def depth_of(c, below, depth):
    """Longest chain of components below c, without recursion."""
    work = [c]
    while work:
        top = work[-1]
        pending = [x for x in below[top] if x not in depth]
        if pending:
            work.extend(pending)
            continue
        work.pop()
        depth[top] = 1 + max((depth[x] for x in below[top]), default=-1)


def packages(graph, cargo, edges):
    """Level 2: one node per package, sized by (dependents × 3) + (lines / 100)."""
    acc = defaultdict(lambda: {"files": 0, "loc": 0, "langs": Counter(), "cats": Counter()})
    for n in graph.nodes.values():
        p, weight = acc[n["pkg"]], 0 if n["kind"] == "external" else n.get("files", 1)
        p["files"] += weight
        p["loc"] += n["loc"]
        p["langs"][n["lang"]] += max(n["loc"], 1)
        p["cats"][n["cat"]] += max(weight, 1)
    links = Counter((graph.nodes[a]["pkg"], graph.nodes[b]["pkg"]) for a, b, _ in edges
                    if graph.nodes[a]["pkg"] != graph.nodes[b]["pkg"])
    dependents, uses = Counter(b for _, b in links), Counter(a for a, _ in links)
    out = []
    for p, v in acc.items():
        name = cargo[p]["name"] if p in cargo else p.removeprefix("ext:")
        out.append({"id": p, "label": name, "parent": "external" if p.startswith("ext:") else p.split("/")[0],
                    "cat": package_category(p, cargo, v["cats"]), "lang": v["langs"].most_common(1)[0][0],
                    "files": v["files"], "loc": v["loc"], "in": dependents[p], "out": uses[p],
                    "score": round(dependents[p] * 3 + v["loc"] / 100, 1)})
    return out, [{"source": a, "target": b, "weight": w} for (a, b), w in links.items()]


def package_category(p, cargo, cats):
    if p.startswith("ext:"):
        return "External"
    if p in cargo and p.startswith("crates/"):
        return "Tests" if p.endswith("-testkit") else "Core"
    for prefix, cat in (("bindings/", "Backend"), ("research/", "Research"), ("scripts/tests", "Tests"),
                        ("fixtures", "Tests"), ("scripts", "Tools"), (".claude", "Tools"), (".specify", "Tools"),
                        (".github", "Tools"), ("docs", "Docs"), ("specs", "Docs")):
        if p.startswith(prefix):
            return cat
    return cats.most_common(1)[0][0]


def subsystems(graph, edges):
    """Level 1: one node per category, with the file-level edges between categories summed."""
    acc = {c: {"files": 0, "loc": 0, "modules": 0, "langs": Counter(), "dirs": Counter()} for c in CATEGORIES}
    for n in graph.nodes.values():
        s, weight = acc[n["cat"]], n.get("files", 1)
        s["files"] += weight
        s["loc"] += n["loc"]
        s["modules"] += 1
        s["langs"][n["lang"]] += max(n["loc"], 1)
        s["dirs"][n["path"].split("/")[0] + ("/" if "/" in n["path"] else "")] += weight
    links = Counter((graph.nodes[a]["cat"], graph.nodes[b]["cat"]) for a, b, _ in edges
                    if graph.nodes[a]["cat"] != graph.nodes[b]["cat"])
    nodes = [{"id": c, "label": c, "cat": c, "files": s["files"], "loc": s["loc"], "modules": s["modules"],
              "lang": s["langs"].most_common(1)[0][0] if s["langs"] else "",
              "langs": s["langs"].most_common(4), "dirs": s["dirs"].most_common(6)} for c, s in acc.items()]
    return nodes, [{"source": a, "target": b, "weight": w} for (a, b), w in links.items()]


def levels(graph, cargo, roots, manifests):
    """The three levels the page draws, plus the project facts its sidebar shows."""
    cycles = annotate(graph, roots)
    edges = sorted(graph.edges)
    l2_nodes, l2_links = packages(graph, cargo, edges)
    l1_nodes, l1_links = subsystems(graph, edges)
    return {
        "detected": sorted({os.path.basename(m) for m in manifests}),
        "manifests": sorted(manifests),
        "cycles": cycles,
        "l1": {"nodes": l1_nodes, "links": l1_links},
        "l2": {"nodes": l2_nodes, "links": l2_links},
        "l3": {"nodes": list(graph.nodes.values()), "links": [{"source": a, "target": b, "kind": k} for a, b, k in edges]},
    }
