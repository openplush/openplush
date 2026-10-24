# How to test a pattern on paper and fabric

<!-- index: print a generated pattern at true scale, measure it, sew it, and record the result. -->

The engine proves a pattern's seam lengths in code; only a printed and sewn copy shows whether the paper, the print and the fabric agree. Follow this page to test a [fixture](../reference/glossary.md) that way and leave a record beside it.

## You will

Have a committed `print-test-YYYY-MM-DD.md` that lists each measured length against the length the pattern promised, and your notes from sewing it.

## Before you start

- A model, a seam set and a fabric file to pattern. They live where fixtures always do: `fixtures/meshes/`, `fixtures/seams/` and `fixtures/fabrics/`. The formats are in the [command line reference](../reference/cli.md#pattern).
- A folder `fixtures/reference/<name>/` for the fixture (`<name>` is its name). It holds the hand-made pattern, the photo and the print-test records.
- A printer, a ruler or tape measure marked in millimetres, and the fabric the test fabric file describes, or the nearest you have.
- The Rust toolchain from [Set up and run every check](../start/setup.md), and Python 3 for the measuring script in step 4.

## Steps

1. Run the pattern with the fixture's own model, seam set and finished size. For the capsule:

   ```zsh
   cargo run -p openplush-cli -- pattern fixtures/meshes/capsule.obj \
     --seams fixtures/seams/capsule.json --fabric fixtures/fabrics/woven.json \
     --size-mm 200 --page a4 -o out/capsule
   ```

   Use `--page letter` for US Letter paper. The run writes `pattern.svg` and `pattern.pdf` into the folder after `-o`, or writes nothing and exits non-zero.

2. Print `pattern.pdf`. Set the print size to **Actual size**, 100%, never Fit to page; the cover page says the same. Write down the printer, the PDF viewer and the paper.

3. Measure the 50 mm square on the cover page with the ruler. If it is not 50 mm within the allowance in the paragraph after the template, the print is scaled: fix the print settings and print again before going on. A reprint replaces the print, so record only the print the pieces were measured on.

4. Choose three pieces, and for each one a longest edge and one sewn edge. An edge here is a run of consecutive points on the piece's sewing line between two corners you choose, named by its start and end point numbers. The tools:
   - Each copy of a piece is its own `sew` path in `pattern.svg`, in placement order. Open `pattern.svg` in a browser or vector viewer and match each printed piece to its path by shape and position. Name the pieces `piece 1`, `piece 2` and so on in path order.
   - A path's points are numbered from 0 in path order. One user unit in the file is 1 mm, so a run's expected length is the sum of the lengths of its segments. `measure.py` below lists each path's points, and sums the run between two point numbers; a run whose end number is below its start wraps past the last point.
   - The longest edge is the longest run between two corners that you can find on the piece.
   - A sewn edge is a run you know is sewn to another piece, from the hand-made pattern or from the model's seams.

   ```python
   import math, re, sys

   svg = open(sys.argv[1]).read()
   pieces = [[(float(x), float(y)) for x, y in re.findall(r"(-?\d+\.\d+) (-?\d+\.\d+)", d)]
             for d in re.findall(r'<path class="sew" d="([^"]*)"', svg)]
   if len(sys.argv) == 2:
       for n, pts in enumerate(pieces, 1):
           print(f"piece {n}")
           for i, (x, y) in enumerate(pts):
               print(f"  {i}: {x:.3f} {y:.3f}")
   else:
       n, a, b = map(int, sys.argv[2:])
       pts = pieces[n - 1]
       run = [pts[i % len(pts)] for i in range(a, b + (len(pts) if b < a else 0) + 1)]
       print(f"{sum(math.dist(p, q) for p, q in zip(run, run[1:])):.3f} mm")
   ```

   `python3 measure.py out/capsule/pattern.svg` lists the points; `python3 measure.py out/capsule/pattern.svg 1 0 3` prints the length of piece 1's run from point 0 to point 3.

5. Cut the pieces out, tape the tiled pages together if a piece spans several, and measure each chosen run on the paper along its sewing line (the dashed one). Write each length down.

6. Sew the plush from the pieces, and note anything that did not fit, pucker or pull while you did.

7. Copy the record template below into `fixtures/reference/<name>/print-test-YYYY-MM-DD.md`, with the date of the run. One file per run; never overwrite an earlier one. Fill it in.

## The record template

````markdown
# Print test: <name>, YYYY-MM-DD

- Pattern command: `<the exact command from step 1>`
- Commit: <git commit of the engine that ran it>
- Printer:
- PDF viewer:
- Paper:
- Fabric:

| Piece | Edge | Expected mm | Measured mm | Difference mm | Pass |
|---|---|---|---|---|---|
| cover | 50 mm square | 50.0 | | | |
| piece N | longest edge, points A to B | | | | |
| piece N | sewn edge, points A to B | | | | |

## Sewing notes

Free text.
````

Add a longest-edge row and a sewn-edge row for each of the three pieces, so the table ends with seven rows. Expected is the run's length from `pattern.svg`; difference is the absolute difference of expected and measured. A row passes when the difference is at most max(1.0 mm, 0.5% of the longer of the two lengths), which is the allowance the engine holds every seam to ([`SeamParity`](https://github.com/openplush/openplush/blob/main/crates/openplush-geom/src/epsilon.rs), and [seam parity](../reference/crates/openplush-pattern.md#seam-parity)). Write the notes below the table, whatever the rows say.

## Check it worked

- The record file is in `fixtures/reference/<name>/` and names the printer, viewer and paper.
- It has seven rows, each with a measured length, a difference and a pass or fail.
- A failing row is kept, not corrected: it is the result. Tell the maintainers where it failed in your pull request.
