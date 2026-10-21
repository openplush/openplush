# openplush-pattern

<!-- index: seam parity, outlines, seam allowance, pieces and cut counts, fabric profiles and sizing. -->

What `openplush-pattern` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

### Seam parity

`parity::check(chain, a, b)` passes two seam-side lengths (`Mm`) if they differ by at most `SeamParity::DEFAULT` for the longer side, else `PatternError::SeamParity { chain, a, b, tolerance }`.

### Outlines

`Outline::new(points)` builds a closed, simple, counter-clockwise polygon in mm or refuses it (`OutlineTooShort`, `OutlineNotFinite`, `OutlineNotSimple { edge, other }` for a repeated point, crossing, touching, overlap or fold, `OutlineClockwise`, `OutlineUnjudgeable`); it never reverses or repairs.

### Seam allowance

`allowance::cut_line(piece, sewing, allowances)` grows a sewing line into its cut line with one `Mm` allowance per edge (edge `i` runs from point `i` to the next): the outline unioned with a rectangle on the outside of each edge as deep as its allowance, minus a guard over each edge with zero allowance (a fold or an edge-stitched edge), 4 × the widest allowance deep, so no fabric lies past it.

Each convex corner gets a miter to where its two offsets meet, or a bevel when that point is more than 4 × the wider allowance away (corners sharper than about 29°) or behind either offset's end, where a miter would cross itself; reflex corners need nothing, as their strips overlap. All zero returns the sewing line itself. Edges shorter than 1e-6 of the piece's bounding-box diagonal are first merged into the next edge, which keeps its own allowance.

It is one `i_overlay` boolean on the `i64` engine, and the result is checked rather than trusted:

- `AllowanceCount { piece, edges, allowances }` (not one allowance per edge)
- `FoldConflict { piece, edge }` (a zero edge's guard overlaps the piece by more than 1e-6 of the diagonal squared)
- `AllowanceSplitsPiece` (the cut line comes out in more than one part)
- `AllowanceEnclosesGap` (it closes off a hole, such as a pocket's mouth)
- `AllowanceNotSimple` (it touches itself, or merging made the outline do so)

### Pieces

`PieceId`; `CutCount::new(as_printed, flipped)` holds how many copies to cut as the template is printed and how many with it flipped, and refuses a total of zero (`ZeroCutQuantity`) or over 100 (`TooManyCuts { total }`); `PatternPiece::new(id, sewing_line, cut_line, cuts, rotations_deg)` refuses `NoRotations` and `NonFiniteRotation { index }`, and reads back through accessors.

### Fabrics

`Fabric::new(FabricSpec)` checks a fabric profile (`FabricId`, name, `FabricCategory` `Woven`/`Knit`/`Pile`/`Felt`, grain, cross and bias stretch in percent, nap, pile length, thickness, frays, default allowance, roll width) and refuses `BadStretch { fabric, direction, percent }` (negative or not finite; `StretchDirection` `Grain`/`Cross`/`Bias`), `ZeroRollWidth`, `ZeroAllowanceFrays` and `PileWithoutNap` (pile with a length but no nap); it never adjusts a value.

Derived, never stored: `rotations_deg()` is `[0]` with a nap, else `[0, 90, 180, 270]` for felt, else `[0, 180]`; `asks_mirroring()` is true for pile, whose export asks print-mirrored or as-is every time.

### Sizing

`scale::size(model_longest, target)` returns the `Scale` and a `SizeRange` (`Below`, `Within`, `Above`, against 75 mm to 1000 mm, both ends within) for a `SizeTarget`: `LongestDimension(Mm)`, judged on that length itself, or `Reference { model_length, finished }`, judged on the model's longest dimension once scaled. The bounds are `scale::MIN_MM` and `scale::MAX_MM`. Out of range is a `SizeRange`, never an error; it refuses `BadModelLength { model_length }` (zero, negative or not finite) and `BadScale { source }` (a scale or finished length that is zero or not finite).

## Depends on

`openplush-geom`, `openplush-mesh` (`id_newtype!`), `openplush-seams`, `i_overlay` (exactly 9.0.1, no default features), `thiserror`
