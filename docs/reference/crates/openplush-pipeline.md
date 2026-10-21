# openplush-pipeline

<!-- index: the public engine API crate: its stages, errors and warnings, and what it depends on. -->

What `openplush-pipeline` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

The public API: `import` with `ImportFiles`, `validate`, `repair`, `chart`, `flatten` with `Parameterization`, `EngineError`, the warning types, `Staged`/`StageResult`, and re-exports. See [engine-api.md](../engine-api.md).

## Depends on

`openplush-flatten`, `-geom`, `-io`, `-mesh`, `-pattern`, `-seams`, `thiserror`
