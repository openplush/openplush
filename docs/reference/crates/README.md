# Crates

<!-- index: one page per engine crate on what it contains today and what it depends on, and which crate owns what. -->

What each engine crate contains **today**, and what it actually depends on, one page per crate. Which crate *should* own a new piece of code is in [Which crate owns what](#which-crate-owns-what); the allowed dependency graph is in the [plan](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/plan.md#crate-dependency-dag-enforced) and enforced by `scripts/check-boundary.sh`. Signatures and item docs: the [Rust API](../api.md).

## Crates with code

- [openplush-geom](openplush-geom.md)
- [openplush-mesh](openplush-mesh.md)
- [openplush-seams](openplush-seams.md)
- [openplush-flatten](openplush-flatten.md)
- [openplush-io](openplush-io.md)
- [openplush-pattern](openplush-pattern.md)
- [openplush-layout](openplush-layout.md)
- [openplush-export](openplush-export.md)
- [openplush-testkit](openplush-testkit.md)
- [openplush-project](openplush-project.md)
- [openplush-pipeline](openplush-pipeline.md)

## Crates with no public items

`openplush-fields` and `openplush-decal` exist so the workspace and its boundary checks are in place. Each crate's purpose is in [Which crate owns what](#which-crate-owns-what); whether it is built is on the [status page](../../start/status.md).

## Which crate owns what

New code goes in the crate that owns its concern, whether or not that crate has code yet. The reasons behind the placement rules are in [Code rules](../code-rules.md#where-code-goes).

| Crate | Owns |
|---|---|
| `openplush-geom` | f64 vectors, robust predicates, the tolerance policy (`epsilon`), millimetre units |
| `openplush-mesh` | `TriangleSoup`, validation, repair, half-edge `ManifoldMesh`, and `id_newtype!`, which defines `u32` ID newtypes |
| `openplush-io` | bytes → `TriangleSoup` (OBJ, glTF, FBX, STL, PLY) |
| `openplush-seams` | `SeamSet`, cut-graph completion, darts, symmetry |
| `openplush-fields` | curvature, nap/grain direction fields |
| `openplush-flatten` | charts, Tutte/LSCM init, SLIM/ARAP, distortion metrics |
| `openplush-pattern` | allowance, notches, labels, grain, piece dedupe |
| `openplush-decal` | markings, detail parts, texture transfer |
| `openplush-layout` | page tiling, nesting adapter |
| `openplush-export` | SVG/PDF → bytes |
| `openplush-project` | project document, schema versions, migrations |
| `openplush-pipeline` | the only public engine API; it re-exports the project document API |

## Bindings

Shells in `bindings/` may depend on `openplush-pipeline` and nothing else from `crates/` (`scripts/check-boundary.sh`).

- [openplush-cli](openplush-cli.md)
- [openplush-wasm](openplush-wasm.md)
