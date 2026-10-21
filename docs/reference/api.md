# Rust API

<!-- index: the rustdoc of every workspace crate, published with this site. -->

The [Rust API reference](../api/index.html) is rustdoc's output for every crate in the Cargo workspace, regenerated each time this site is built, so its signatures and item docs cannot drift from the code. What the pages are built from, and what the build checks, is in [Checks](checks.md#generated-api-pages-docs).

## Entries

- **[Crate index](../api/index.html)**: every workspace crate in name order, each linking to its rustdoc.
- **Links between crates** stay inside the site. An item from a dependency outside the workspace, or from the standard library, links to docs.rs or doc.rust-lang.org, or is not linked.
- **Locally**: `cargo doc --workspace --no-deps --open` opens one crate's rustdoc without building the site; `scripts/check.sh docs` builds the whole site, with these pages under `target/site/dev/api/`.
