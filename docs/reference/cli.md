# Command line (`openplush-cli`)

The headless shell: every engine capability as file-in, file-out (Constitution II). This page lists what exists **today**. The planned surface is in the [quickstart](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/quickstart.md#1-engine-headless-constitution-ii).

```zsh
cargo run -p openplush-cli -- --help
```

## Status

`validate` and `repair` run their stages. `pattern` parses and checks its arguments, then exits 1 with `openplush-cli: the pattern stage is not built yet` on stderr and writes nothing. Which stages are built is on the [status page](../start/status.md).

## Subcommands

| Subcommand | Arguments | Stage |
|---|---|---|
| `validate <MODEL>` | `--json <PATH>`: where to write the health report | import, validate |
| `repair <MODEL>` | `--no-weld`, `--no-drop-degenerates`, `--no-split-non-manifold-edges`, `--no-split-bowties`, `--no-fix-orientation`; `--fill <LOOP>`, repeatable; `--plan-out <PATH>` (optional); `-o/--out <PATH>` | import, repair |
| `pattern <MODEL>` | `--seams <PATH>`, `--fabric <NAME>`, `--size-mm <MM>`, `--page a4\|letter`, `-o/--out <DIR>` | pattern |

### The model and the files beside it

`<MODEL>` is read with the reader its extension names. OBJ, STL, PLY and GLB are read alone. A `.gltf` (any case) is read with every regular file below its directory, recursively, so its buffers and images can be found. Those files are named by their path from that directory with `/` separators, in byte order of those names; symlinks are not followed. The model and those files together may be at most 1 GiB (2³⁰ bytes); a larger set exits 1 before the file that passes the limit is read.

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

Nothing is written unless repair succeeds.

Values checked while parsing `pattern`:

- `--size-mm`: a finite number above 0, the target finished size in millimetres. Which dimension it measures is decided by the task that builds scaling.
- `--page`: `a4` or `letter`.

`--fabric` is not checked yet. It takes any name until fabric presets exist in the engine.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | The engine refused, a file could not be read or written, the model's set is over the size limit, or the stage is not built yet. stderr says which: an engine refusal is `openplush-cli: <code>: <message>`, with the [error's code and message](engine-api.md#errors); a file is `openplush-cli: <path>: <reason>` |
| 2 | Usage error: an unknown flag or value, or a missing required argument. `clap` prints the usage |
| 101 | The engine panicked: Rust's default panic exit, which holds while no build profile sets `panic = "abort"`. A panic is a bug, never a response to bad input ([ADR 0009](../adr/0009-wasm-panic-is-a-worker-crash.md)); please report it with the message from stderr |
