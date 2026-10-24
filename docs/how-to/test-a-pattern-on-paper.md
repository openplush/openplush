# How to test a pattern on paper and fabric

<!-- index: print a generated pattern at true scale, measure it, sew it, and record the result. -->

The engine proves a pattern's seam lengths in code; only a printed and sewn copy shows whether the paper, the print and the fabric agree. Follow this page to test a [fixture](../reference/glossary.md) that way and leave a record beside it.

## You will

Have a committed `print-test-YYYY-MM-DD.md` that lists each measured length against the length the pattern promised, and your notes from sewing it.

## Before you start

- A model, a seam set and a fabric file to pattern. Fixture models go in `fixtures/reference/<name>/` (`<name>` is the fixture's name), beside the record. The formats are in the [command line reference](../reference/cli.md#pattern).
- A printer, a ruler or tape measure marked in millimetres, and the fabric the test fabric file describes, or the nearest you have.
- The Rust toolchain from [Set up and run every check](../start/setup.md).

## Steps

1. Run the pattern. Use the fixture's own model, seam set and finished size:

   ```zsh
   cargo run -p openplush-cli -- pattern <MODEL> \
     --seams <SEAMS> --fabric <FABRIC> --size-mm <MM> \
     --page a4 -o out/<name>
   ```

   Use `--page letter` for US Letter paper. The run writes `pattern.svg` and `pattern.pdf` into `out/<name>`, or writes nothing and exits non-zero.

2. Print `pattern.pdf`. Set the print size to **Actual size**, 100%, never Fit to page; the cover page says the same. Write down the printer, the PDF viewer and the paper.

3. Measure the 50 mm square on the cover page with the ruler. If it is not 50 mm within the allowance under the template, the print is scaled: fix the print settings and print again before going on.

4. Choose three pieces. For each, pick its longest edge and one edge that is sewn to another piece. In `pattern.svg` the pieces are the `sew` paths, in file order, and name them `piece 1`, `piece 2` and so on in that order. One user unit in the file is 1 mm, so an edge's expected length is the distance between two consecutive points of its path.

5. Cut the pieces out, tape the tiled pages together if the piece spans several, and measure each chosen edge on the paper along its sewing line (the dashed one). Write each length down.

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
| piece N | longest edge | | | | |
| piece N | seam edge | | | | |

## Sewing notes

Free text.
````

Add a row for each of the three pieces' two edges, so the table ends with seven rows. Expected is the length from `pattern.svg`; difference is the absolute difference of expected and measured. A row passes when the difference is at most max(1.0 mm, 0.5% of the longer of the two lengths), which is the allowance the engine holds every seam to ([`SeamParity`](https://github.com/openplush/openplush/blob/main/crates/openplush-geom/src/epsilon.rs), and [seam parity](../reference/crates/openplush-pattern.md#seam-parity)). Write the notes below the table, whatever the rows say.

## Check it worked

- The record file is in `fixtures/reference/<name>/` and names the printer, viewer and paper.
- It has seven rows, each with a measured length, a difference and a pass or fail.
- A failing row is kept, not corrected: it is the result. Tell the maintainers where it failed in your pull request.
