# Roadmap

What comes next, in order. This is the one place that answers "what do we work on now?" Plain words, no IDs. When a feature is picked up, its design detail lives in its spec under [`specs/`](specs/); this page only says what and when. Edit it freely: move items, cut them, add new ones.

## Now: small housekeeping

- Switch GitHub Pages to deploy from GitHub Actions, so the docs workflow is the only thing publishing the site.
- Go through the Dependabot alerts on `main`: fix or dismiss each one.
- The maker FAQ says only OBJ, STL and PLY open; glTF and FBX open too.
- Two command-line tests fail on Windows only (letter case in file names, and a 1 TB test file): skip them on Windows or change their test data.

## Next: prove it works on a real plush

- Get one real plush model with a known-good, hand-made pattern into `fixtures/`.
- Run it through the command line, print the PDF, measure the 50 mm square and a few pieces with a ruler, and sew it. Write down everything that went wrong.
- Give each fabric its own stretch limit, so the "squeezed piece" warning uses the fabric instead of a fixed 15%.

## Then: a pattern a maker can sew from

- Notches, so matching edges line up when sewing.
- Piece labels with cut counts, and identical or mirror-image pieces merged into one template.
- Draw the stuffing opening on its piece, and accept darts in the seams file once they are drawn.
- Suggest a dart where a piece is squeezed.
- The command line also writes the pieces, metrics and warnings as JSON files.

## Then: the web app

- The real browser worker and the engine's WebAssembly operations.
- Load a model, set its size, check its health, draw seams, export.
- Save and open a project file.

## Later

- Fabric grain and nap arrows, and fur fabrics.
- Markings, eyes, and painted fabric regions.
- Seam-placement hints: show where seams belong before the maker draws them.
- Use the seams already in a model's UV map, in the command line and the app.
- Speed on large models in the browser: measure first, then fix the slow steps.
