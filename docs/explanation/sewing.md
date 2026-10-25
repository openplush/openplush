# Sewing concepts for programmers

<!-- index: seam allowance, grain, nap and notches, what each means for a pattern and where it lives in the engine. -->

This page explains the four sewing ideas the engine has to get right: seam allowance, grain, nap and notches. For each one it says what it means for a pattern and where it lives in the engine. Read it before working on `openplush-pattern`, `openplush-layout` or `openplush-export` if you have never sewn.

## The short answer

A sewist cuts fabric along a **cut line** and sews along a **sewing line** a little inside it; the strip between them is the **seam allowance**. Woven fabric has a **grain**, the direction of its threads, and stretches differently along it, across it and diagonally. Pile fabrics such as faux fur and minky have a **nap**, a direction the pile lies in, so every piece must point the same way. **Notches** are small marks on matching seam edges that tell the sewist which edges go together and where they line up. In the engine, allowance is built in `openplush-pattern` and drawn by `openplush-export`; grain and nap are recorded in a fabric profile (`openplush-pattern`'s `Fabric`) and limit how `openplush-layout` may turn and mirror pieces; notches are placed on each piece by the `pattern` stage (`openplush-pattern`'s `Notch`).

## How it works

### Seam allowance

Two pieces are sewn face to face along their sewing lines, a few millimetres in from the cut edge, so the cut edges are hidden inside the toy once it is turned right side out. The pattern must therefore show both lines: the sewing line, where the flattened piece's outline is, and the cut line, outside it by the allowance. The allowance is not one number for the whole piece. A seam gets the usual allowance, the stuffing opening often a wider one so it can be closed by hand, and an edge that is folded or edge-stitched on a fabric that does not fray (felt, for example) gets none.

In the engine, `openplush-pattern`'s `allowance::cut_line` grows a sewing line into its cut line with one allowance per edge, in millimetres. Each edge's strip is unioned with the outline in one polygon boolean; a zero-allowance edge gets a guard so no fabric lies past it; sharp corners are beveled where a miter would reach too far. The result is checked rather than trusted: allowances that split the piece in two, close off a gap, or make the cut line touch itself are typed errors. A `Fabric` profile carries the fabric's `default_allowance`, and refuses a zero default on fabric that frays (`ZeroAllowanceFrays`). A `PatternPiece` holds both lines; `openplush-layout` packs pieces by their cut lines, since that is the fabric each one uses; `openplush-export` draws the cut line solid and the sewing line dashed, in SVG and PDF.

### Grain

Woven fabric is threads crossing at right angles. Along the threads (the grain, and across it) it barely stretches; on the diagonal (the bias) it stretches much more. Knits stretch too, usually more across than along. A pattern piece therefore has a direction: the printed grain arrow is laid parallel to the fabric's edge, and turning a piece changes how it stretches on the toy.

In the engine, `Fabric` records stretch in percent along the grain, across it and on the bias, refusing a negative or non-finite value (`BadStretch`); the values are kept for warnings about fabrics that stretch differently, and never change the flattening, which treats fabric as stretching the same in every direction. Grain also sets which turns a piece may be laid at: `Fabric::rotations_deg` gives 0° and 180° for most fabrics (turning a piece end for end keeps it on grain) and every quarter turn for felt, which has no grain. A `PatternPiece` carries its allowed rotations, and `openplush-layout`'s `shelf::pack` places each piece at the first one, which must be a multiple of 90°. Choosing each piece's grain direction from the model, and printing grain arrows, are not built yet ([status](../start/status.md)).

### Nap

Stroke faux fur one way and it lies smooth; stroke it the other way and it stands up and looks a different shade. That direction is the nap. On a plush, the fur should run the same way on every piece, usually from head to tail, so a napped piece may not be turned at all: upside down it would look like a different fabric.

In the engine, `Fabric` has a `nap` flag, and `rotations_deg` gives only 0° when it is set, whatever the category. A pile fabric with a pile length must have a nap (`PileWithoutNap`). Pile is also traced on its back, so the maker may want the print mirrored: `Fabric::asks_mirroring` is true for pile, meaning export must ask every time, with no default, and `openplush-layout`'s `LayoutOptions` takes the answer as `mirrored`, which mirrors the whole print and fabric plan. A piece cut as a left and right pair is a separate matter: `CutCount` records copies cut as printed and copies cut flipped, and layout mirrors the flipped ones.

### Notches

Long seams are hard to line up by eye, so each pair of matching seam edges carries notches: small marks at the same distance along both sides, with a code saying which edge sews to which. Pinning notch to notch keeps the seam from drifting. The registration crosshairs `openplush-layout` places on printed pages are a different thing: they line up the paper tiles, not the fabric pieces.

In the engine, the `pattern` stage gives every piece its notches. It walks each seam side that the `SewingMap` (`openplush-seams`) pairs, and puts a notch at each end and more evenly spaced between, at most 50 mm apart and at the same fractions of the length on every side. Exact seam lengths (see [flattening](flattening.md)) make the same fraction the same point of the seam on every piece. The notches between the ends carry a mark count, cut as that many marks side by side, that differs from every seam they meet on a piece, so a piece turned the wrong way does not line up. `openplush-export` prints each one as that many short ticks across the seam allowance, from the sewing line to the cut line.

## Why not the alternative

The obvious alternative for allowance is one offset distance for the whole piece, which any polygon library can do. It cannot express what a pattern needs: a wider allowance on the opening, none on a fold or an edge-stitched edge, and a different one on a seam the maker overrides. Building the cut line from one strip per edge, in a single boolean, handles all of these, and checking the result turns a bad combination into an error rather than a misprinted piece.

## Decisions

[Grainline and nap mapping (data model)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/data-model.md#6-grainline-and-nap-mapping) · [fabric profiles (data model)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/data-model.md#6a-fabric-profiles-fr-050-d-018) · [seam allowance (data model)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/data-model.md#7-seam-allowance) · [notches and labels (data model)](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/data-model.md#8-notches-and-labels) · [ADR 0006, allowance construction](../adr/0006-allowance-construction.md)
