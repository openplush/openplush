# Command line (`openplush-cli`)

The headless shell: every engine capability as file-in, file-out (Constitution II). This page lists what exists **today**. The planned surface is in the [quickstart](https://github.com/openplush/openplush/blob/main/specs/001-pattern-pipeline/quickstart.md#1-engine-headless-constitution-ii).

```zsh
cargo run -p openplush-cli -- --help
```

## Status

The arguments parse and are validated. **No subcommand runs a stage yet**: each one exits 1 with `openplush-cli: the {stage} stage is not built yet` on stderr and writes nothing. Each one is wired up by the task that builds its stage. Which stages are built is on the [status page](../start/status.md).

## Subcommands

| Subcommand | Arguments | Stage |
|---|---|---|
| `validate <MODEL>` | `--json <PATH>`: the health report | validate |
| `repair <MODEL>` | `--weld`, `--fix-orientation`, `--fill-holes auto` (optional), `--plan-out <PATH>`, `-o/--out <PATH>` | repair |
| `pattern <MODEL>` | `--seams <PATH>`, `--fabric <NAME>`, `--size-mm <MM>`, `--page a4\|letter`, `-o/--out <DIR>` | pattern |

Values checked while parsing:

- `--size-mm`: a finite number above 0, the target finished size in millimetres. Which dimension it measures is decided by the task that builds scaling.
- `--page`: `a4` or `letter`.
- `--fill-holes`: `auto`. The engine classifies each boundary loop as an opening or an accidental hole.

`--fabric` is not checked yet. It takes any name until fabric presets exist in the engine.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. Today only `--help` and `--version` (prints `openplush-cli <version>`) |
| 1 | The engine refused, or the stage is not built yet. The reason is on stderr |
| 2 | Usage error: an unknown flag or value, or a missing required argument. `clap` prints the usage |
| 101 | The engine panicked: Rust's default panic exit, which holds while no build profile sets `panic = "abort"`. A panic is a bug, never a response to bad input ([ADR 0009](../adr/0009-wasm-panic-is-a-worker-crash.md)); please report it with the message from stderr |
