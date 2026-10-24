# Command line (`openplush-cli`)

<!-- index: `openplush-cli` subcommands, flags, and exit codes. -->

The headless shell: every engine capability as file-in, file-out (Constitution II). This page lists what exists **today**. The planned surface is in the [quickstart](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/quickstart.md#1-engine-headless-constitution-ii).

```zsh
cargo run -p openplush-cli -- --help
```

## Status

Which stages and outputs are built is on the [status page](../start/status.md).

## Subcommands

| Subcommand | Arguments | Stage |
|---|---|---|
| `validate <MODEL>` | `--json <PATH>`: where to write the health report | import, validate |
| `repair <MODEL>` | `--no-weld`, `--no-drop-degenerates`, `--no-split-non-manifold-edges`, `--no-split-bowties`, `--no-fix-orientation`; `--fill <LOOP>`, repeatable; `--plan-out <PATH>` (optional); `-o/--out <PATH>` | import, repair |
| `pattern <MODEL>` | `--seams <PATH>`, `--fabric <PATH>`, `--size-mm <MM>`, `--page a4\|letter` (optional, `a4`), `-o/--out <DIR>` | import, repair, chart, flatten, size, pattern, layout, export |

### The model and the files beside it

`<MODEL>` is read with the reader its extension names: OBJ, STL, PLY, GLB and binary FBX, each read alone, and glTF; any other type exits 1 with `UnknownFormat` ([status](../start/status.md)). A `.gltf` (any case) is read with every regular file below its directory, recursively, so the buffers and images it names can be found; one it names but that is not there exits 1 with `MissingResource`. Those files are named by their path from that directory with `/` separators, in byte order of those names; symlinks are skipped, whatever their names, and the name of a file or directory that would be read, if it is not UTF-8, exits 1 rather than be renamed. The model and those files together may be at most 1 GiB (2³⁰ bytes); a larger set exits 1 before the file that passes the limit is read.

### `validate`

Writes the model's `ValidationReport` to `--json` as one JSON object, and exits 0 whenever the model imports, defects or not. Each list is a key named as the report's accessor, and IDs are integers (the soup's face and vertex indices):

| Key | Value |
|---|---|
| `unmeasurable` | `null`; `"no_triangles"`; or `{"extent": {"diagonal": <number>}}` when the model's size is outside what tolerances allow, and every list is empty |
| `unwelded` | weld groups, each a list of vertices |
| `zero_area`, `repeated_index` | faces |
| `duplicates` | `{"face", "of", "orientation": "same" \| "opposite"}` |
| `non_manifold_edges` | `{"vertices": [a, b], "faces": [...]}` |
| `non_manifold_vertices` | vertices |
| `winding_conflicts`, `non_orientable` | vertex pairs |
| `inside_out`, `components` | lists of faces |
| `boundary_loops` | lists of vertices |
| `self_intersections` | face pairs |

### `repair`

Starts from the default plan, every step on; each `--no-<step>` turns one off. `--fill` lists a boundary loop to fill by its lowest source vertex, written `<primitive>.<vertex>` in decimal (`0.12`); a loop not listed stays an opening, since filling is never automatic. Any other text for `--fill` is a usage error.

- `-o` gets the repaired mesh as OBJ: one `v` per vertex, one `vt` per corner that has a UV, and one `f` per face in face order, with `v/vt` for a corner that has a UV. Normals and materials are not written.
- `--plan-out` gets the plan as the project schema's `repairPlan`: `weld`, `dropDegenerates`, `splitNonManifoldEdges`, `splitBowties`, `fixOrientation`, and `fillHoles` as loop strings.

Nothing is written if repair fails. `-o` is written before `--plan-out`, so if the plan cannot be written the command exits 1 with the mesh already on disk.

### `pattern`

Runs the model through every stage and writes `pattern.svg` and `pattern.pdf` into `-o`, which is created if missing. Nothing is written unless every stage succeeds.

- The model is read as above and repaired with the default plan, every step on. A model that needs another plan is repaired first with `repair -o`, and that OBJ is patterned.
- `--seams` is a seam-set file in the [`openplush-seam-set` format](https://github.com/openplush/openplush/blob/main/fixtures/README.md#seam-set-format), version 1. Each edge's two indices are OBJ vertex indices (the order of `v` lines), counted from `vertexIndexBase`, 0 or 1. Each is found on the repaired model through the corners repair kept from it. Exit 1, naming the seam by its place in the list (`seam 0` is the first):
  - an index below `vertexIndexBase`;
  - an index no vertex came from;
  - an index more than one vertex came from;
  - two vertices with no edge between them;
  - a `format`, `version`, `vertexIndexBase`, `edge` or `kind` the format does not allow;
  - a `kind` other than `cut` or `fabricBoundary`: `dart`, `opening` and `mandatory` are not supported in a seam-set file.
- Where repair split an edge into a boundary (a third face on an edge, or a cut where the faces' winding cannot agree), every copy of that edge is added as a `mandatory` seam, so the copies are sewn back together. If these seams and the seam file's together leave a piece that is not a disk, the run exits 1 with `ChartNotDisk` or `Flatten:`.
- `--fabric` is one JSON object in the project schema's `fabric` shape:
  - required fields `id`, `name` and `category` (`woven`, `knit`, `pile`, `felt`);
  - optional fields take the schema's defaults, including a 6 mm allowance and a 1500 mm roll;
  - an unknown field, a wrong type, a negative length or a zero roll width exits 1, naming the field, as does a fabric the engine refuses, with its reason.

  `fixtures/fabrics/woven.json` is a test fabric.
- `--size-mm` is the finished length of the model's longest dimension: a finite number above 0, in millimetres. Outside 75 mm to 1000 mm the run still succeeds, with a `size-out-of-range` warning.
- The print places one template of every piece on a roll as wide as the fabric's `rollWidthMm`, 10 mm apart, not mirrored; the sheet ends at the rightmost piece. `pattern.svg` is that sheet at true scale. `pattern.pdf` tiles it on `--page`.
- Every warning the stages raised prints in both files: under the SVG's sheet, and on the PDF's cover. Warnings never change the exit code.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | The engine refused, a file could not be read, used or written, or the model's set is over the size limit. stderr says which: an engine refusal is `openplush-cli: <code>: <message>`, with the [error's code and message](engine-api.md#errors); a file is `openplush-cli: <path>: <reason>` |
| 2 | Usage error: an unknown flag or value, or a missing required argument. `clap` prints the usage |
| 101 | The engine panicked: Rust's default panic exit, which holds while no build profile sets `panic = "abort"`. A panic is a bug, never a response to bad input ([ADR 0009](../adr/0009-wasm-panic-is-a-worker-crash.md)); please report it with the message from stderr |
