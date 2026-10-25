# openplush-pattern

<!-- index: seam parity, outlines, seam allowance, pieces and cut counts, fabric profiles and sizing. -->

What `openplush-pattern` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

### Seam parity

`parity::check(chain, a, b)` passes two seam-side lengths (`Mm`) if they differ by at most `SeamParity::DEFAULT` for the longer side, else `PatternError::SeamParity { chain, a, b, tolerance }`.

### Outlines

`Outline::new(points)` builds a closed, simple, counter-clockwise polygon in mm or refuses it (`OutlineTooShort`, `OutlineNotFinite`, `OutlineNotSimple { edge, other }` for a repeated point, crossing, touching, overlap or fold, `OutlineClockwise`, `OutlineUnjudgeable`); it never reverses or repairs. `outward_at(i)` is the unit direction out of the outline at corner `i`, halfway between its two edges' outward normals, so it points out at reflex corners too. `label_point()` is a point inside the outline as far from its edges as a 32 × 32 grid over its bounding box finds (the first such in row order); an outline too thin for the grid gets a point just inside its lowest, leftmost corner, along its bisector, a quarter of its shorter edge in or half that again until the point is inside.

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

`PieceId`; `CutCount::new(as_printed, flipped)` holds how many copies to cut as the template is printed and how many with it flipped, and refuses a total of zero (`ZeroCutQuantity`) or over 100 (`TooManyCuts { total }`); `PatternPiece::new(id, sewing_line, cut_line, cuts, rotations_deg)` refuses `NoRotations` and `NonFiniteRotation { index }`, and reads back through accessors; `with_notches(notches)` gives it its match notches, read back by `notches()`, and `with_label(label)` its label, read back by `label()`.

### Notches

`Notch::new(at, outward, chain, marks)` is a match notch on a sewing line in mm: the point, the direction out of the piece (scaled to unit length), the lead `SeamChainId` of the seam it matches along, and how many marks are cut side by side. It refuses a point that is not finite, a zero or non-finite direction, or zero marks (`BadNotch { chain }`).

`notch::fractions(chain, length)` gives the fractions of a seam side's length, strictly between its ends, where every side of the seam is notched: evenly spaced at most `notch::SPACING_MM` (50 mm) apart, and at least the midpoint. Even spacing reads the same from either end. Past 1000 notches it refuses `TooManyNotches { chain, length }`.

`notch::mark_counts(meets)` gives each chain the least mark count, from 1, that no chain it meets has taken, chains taken in ascending order, so two seams that meet on a piece never carry the same count.

### Labels

`Label::new(at, lines, seams)` is a piece's label: its block of lines, top first, centred on `at`, and one `SeamLabel::new(at, text)` per seam side on the piece, its text centred on its point. Both refuse a point that is not finite (`BadLabelPoint`).

`label::block(id, cuts, fabric, sews_to)` gives the block's lines: `Piece N`, the cut line (`Cut 1`, `Cut 2 flipped`, `Cut 3 (2 + 1 flipped)`), the fabric's name, and, when there are any, `Sews to piece N` or `Sews to pieces N, M`, numbered as the warning summary numbers pieces. `label::seam_text(chain)` is `Seam N`, numbered as the summary numbers seam chains.

### Fabrics

`Fabric::new(FabricSpec)` checks a fabric profile (`FabricId`, name, `FabricCategory` `Woven`/`Knit`/`Pile`/`Felt`, grain, cross and bias stretch in percent, nap, pile length, thickness, frays, default allowance, roll width) and refuses `BadStretch { fabric, direction, percent }` (negative or not finite; `StretchDirection` `Grain`/`Cross`/`Bias`), `ZeroRollWidth`, `ZeroAllowanceFrays` and `PileWithoutNap` (pile with a length but no nap); it never adjusts a value.

Derived, never stored: `rotations_deg()` is `[0]` with a nap, else `[0, 90, 180, 270]` for felt, else `[0, 180]`; `asks_mirroring()` is true for pile, whose export asks print-mirrored or as-is every time.

### Sizing

`scale::size(model_longest, target)` returns the `Scale` and a `SizeRange` (`Below`, `Within`, `Above`, against 75 mm to 1000 mm, both ends within) for a `SizeTarget`: `LongestDimension(Mm)`, judged on that length itself, or `Reference { model_length, finished }`, judged on the model's longest dimension once scaled. The bounds are `scale::MIN_MM` and `scale::MAX_MM`. Out of range is a `SizeRange`, never an error; it refuses `BadModelLength { model_length }` (zero, negative or not finite) and `BadScale { source }` (a scale or finished length that is zero or not finite).

## Depends on

`openplush-geom`, `openplush-mesh` (`id_newtype!`), `openplush-seams`, `i_overlay` (exactly 9.0.1, no default features), `thiserror`
