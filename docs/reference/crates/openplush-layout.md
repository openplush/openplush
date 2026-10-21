# openplush-layout

<!-- index: the fabric plan and print layout on shelves, and how a sheet is tiled onto pages. -->

What `openplush-layout` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

### Shelf packing

`shelf::pack(pieces, opts)` is the fabric plan: it lays every copy of every `PatternPiece` on shelves across a roll, tallest first (ties by `PieceId`), and returns one `Sheet` as wide as the roll and as long as the shelves need. Copies cut as printed come first and copies cut flipped follow, mirrored; when the print is mirrored for pile (`LayoutOptions`'s `mirrored`) the whole plan is, so a flipped copy is then the unmirrored one.

`shelf::pack_templates(pieces, opts)` is the print: one template per piece (copy 0), mirrored when the print is, laid out the same way. Each piece uses its first allowed rotation, which must be a multiple of 90 degrees (the turn is exact, with no trigonometry).

- `LayoutOptions::new(roll_width, gap, mirrored)`
- `Sheet` (`width_mm`, `length_mm`, `placements`; y-up, so its top edge is `y = length_mm`)
- `Placement` (`piece`, `copy`, `rotation_deg`, `offset`, `mirrored`: mirror about the y-axis, then rotate counter-clockwise, then move by `offset`; `apply(point)` does exactly that to a point of the piece's outline, with no trigonometry)

### Page tiling

`pages::tile(sheet, paper)` covers a `Sheet` with the fewest portrait pages of `Paper::Letter` or `Paper::A4` (`size_mm()`, 215.9 × 279.4 and 210 × 297): each prints inside a 10 mm margin (`MARGIN_MM`) and repeats 15 mm of its right and bottom neighbours, so pages join by overlapping, never by trimming. It returns a `PageTiling` (`paper`, `rows`, `columns`, `pages`, row by row from the sheet's top edge, its highest y since sheets are y-up, each row left to right) of `Page`s (`row`, `column`, and the sheet window `window_min` to `window_max` in mm, which may run past the sheet's edge, and `label()`, row letter then column number from 1, such as "B3").

`PageTiling::marks()` lists registration mark centres in sheet mm: two on each edge two pages share, centred across the 15 mm band at a quarter and three quarters along the edge, so each prints on both pages; edges between columns row by row, then edges between rows.

`Page::scale_square_min()` is the lowest corner of the page's 50 mm scale-check square (`SCALE_SQUARE_MM`), which sits at the page's top left inset 15 mm, so no overlap band or mark covers it. `INCH_SQUARE_MM` (25.4) is the side of the labelled square drawn inside it; where inside, the labels and the drawing itself are export's.

### Errors

`LayoutError`: `BadRollWidth`, `NoPieces`, `PieceTooWide`, `TooManyPages` (over 26 pages across or down), `UnsupportedRotation`

## Depends on

`openplush-geom`, `openplush-pattern`, `thiserror`
