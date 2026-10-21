# openplush-cli

<!-- index: the command-line binding over the pipeline, and what it depends on. -->

What the `openplush-cli` binding is **today**, and what it actually depends on. Like every binding, it may depend on `openplush-pipeline` and nothing else from `crates/` ([Bindings](README.md#bindings)).

## What it is today

The commands over `openplush-pipeline`: `validate` and `repair` run their stages, with the CLI's own gathering of a `.gltf`'s files, JSON for the report and plan, and OBJ writer; `pattern` parses only. See [cli.md](../cli.md).

## Depends on

`openplush-pipeline`, `clap` ([ADR 0004](../../adr/0004-cli-argument-parsing.md)), `serde_json`
