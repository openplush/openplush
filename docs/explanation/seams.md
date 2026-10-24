# Seams, charts, darts and openings

<!-- index: how seams cut a mesh into charts, what darts and openings are, and why a chart must be a disk. -->

This page explains how OpenPlush represents the maker's seams, how seams cut a mesh into charts that become pattern pieces, what darts and openings are, and why every chart must be a topological disk before it can flatten. Read it before changing `openplush-seams`, or when the chart stage refuses a seam layout.

## The short answer

A seam is a flag on a mesh edge, with a kind saying what it is for; drawing or erasing a seam never changes the mesh. The seams cut the surface into charts, the regions you can walk across without crossing a seam, and each chart becomes one pattern piece. A piece of fabric is a disk, one connected sheet with one outer edge and no holes or handles, so a chart must be a disk too, or it cannot be laid flat as one piece. When a chart is not, the engine says why and proposes the fewest extra seams that would make it one, for the maker to accept or not.

## How it works

### A seam is a flag on an edge

`SeamSet` maps edges of the repaired mesh to a `SeamKind`:

- `Cut`: an ordinary seam between two pieces, or between two edges of one piece.
- `Dart`: a seam that ends inside a piece.
- `Opening`: a stretch of seam left unsewn, for turning and stuffing.
- `FabricBoundary`: where one fabric meets another.
- `Mandatory`: an edge repair had to split, such as one shared by three faces. It is never removed and never changes kind, so the maker's undo can never take away a cut the geometry needs.

Because seams are only flags, the repaired mesh and every ID in it stay the same whatever the maker draws. A seam edit is a change to the set, and only the stages from charting on depend on it.

Seam edges meet at vertices to form a seam graph (`SeamGraph`). Its nodes are where something changes: a junction of three or more seam edges, a seam reaching the mesh boundary, a dart's tip, the ends of a split edge's copies, or a change of seam kind. A seam chain is a run of seam edges between two nodes, or a closed loop with none. A chain is the unit the sewing map pairs: each chain's sides are matched up so the edges that sew together are known.

### Charts

`charts` floods across faces without crossing a seam; each region it finds is a `Chart`, named by its lowest face. A seam that separates nothing, such as a dart or a seam that stops short, slits the chart open instead of splitting it in two.

`ChartMesh::new` then cuts the chart open along its seams into the flat arrays the flattener reads. A mesh vertex on a seam gets one copy for each side of the cut, so a seam running through a chart separates its two sides; a dart's tip stays a single vertex, which is what makes a dart a slit rather than a cut that goes all the way through.

The `SewingMap` records, for each seam chain, the chart edges that sew together along it. Usually a chain has two sides, one in each piece it joins. An edge repair split, such as a fin where three faces met, joins its copies into one bundle with three or four sides. A chain along the mesh's own boundary with nothing to sew to is a hole's rim and is refused (`UnpairedBoundarySeam`), never paired with itself.

### Why a chart must be a disk

A connected surface is classified by two numbers: how many boundary loops it has, and how many handles (its genus). They are tied to the mesh counts by Euler's formula, V − E + F = 2 − 2·genus − loops. A flat piece of fabric has one boundary loop and no handles, so V − E + F = 1. The flattener depends on this: it lays the chart's one boundary loop flat at its 3D lengths and fills the inside (see [flattening](flattening.md)). With no boundary there is nothing to lay out; with two loops, one of them would have to sit inside the piece as a hole; with a handle, the surface cannot be laid in the plane without cutting it.

`check_disk` counts the chart as it would be once cut open, without building it, and names its failure as a `ChartFailure`:

| Failure | Example | The cut `close_chart` proposes |
|---|---|---|
| `Closed`: no boundary | A ball with no seam | The shortest path between the two most curved vertices |
| `MultipleBoundaries`: more than one loop | A sleeve cut off at both ends | Shortest paths joining the loops, one at a time, until one remains |
| `PositiveGenus`: one or more handles | An arm that touches the body | Cuts around each handle, found with a tree-cotree search |

Paths are measured by 3D length, with ties going to the lower ID, so the same chart always gets the same proposal. The proposal is checked to leave one disk before it is offered, and it is only offered: the chart stage returns it inside `ChartNotDisk` and changes nothing. Adding those seams is the maker's choice.

Before the disk check, `check_fold` refuses a chart in which two neighbouring faces, not separated by a seam, lie back over each other (their normals within 5° of opposite). No flattener can open such a fold without overlapping fabric, and no seam proposal is offered for it, because the fix is in the model.

### Darts

A dart is a short seam that ends inside a piece. Sewn closed, it takes up fabric and gives a flat piece a curve, as at the top of a head. `mark_dart` marks a walk of edges a dart only if one end is anchored, on a seam or the mesh boundary, and every other vertex touches neither, so the dart is a slit from the piece's edge into its middle. A slit only lengthens the chart's one boundary loop, so a dart keeps a disk a disk. When flattening shows a region compressed beyond what the fabric can take, a dart there is the usual remedy.

### Openings

A plush is sewn inside out, turned through a gap, stuffed, and the gap closed by hand. `mark_opening` turns a seam chain into an `Opening`: it is still a seam, and still pairs two edges, but the maker leaves it unsewn. A mandatory seam is refused, since it is a cut the geometry needs rather than one the maker chose. Because a change of kind is a node, the opening is a chain of its own.

## Why not the alternative

The obvious alternative is to cut the mesh itself along each seam, so that charts are separate meshes. That makes every seam edit change the geometry, renumbering vertices and faces, so anything attached to them (markings, saved seams, undo history) would have to be remapped on each edit. Keeping seams as flags on an unchanging mesh makes undo a change to a set, and leaves the cut-open copy as a view built for the flattener to read.

## Decisions

[Seams, charts and sewing correspondence (data model)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/data-model.md#4-seams-charts-and-sewing-correspondence) · [seam requirements (spec)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/spec.md#seams) · [Constitution, Principle VII](https://github.com/openplush/openplush/blob/main/CONSTITUTION.md)
