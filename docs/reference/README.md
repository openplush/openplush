# Reference

Information-oriented: precise, complete, and dry. CLI commands and flags, warning and error codes, the project file format.

- [Engine API](engine-api.md): the public `openplush-pipeline` surface as built, error messages, warning order, wire forms.
- [Crates](crates.md): what each engine crate contains today and what it depends on.
- [Command line](cli.md): `openplush-cli` subcommands, flags, and exit codes.

Signatures and item docs are generated from the code: `cargo doc --workspace --no-deps --open`. Parts not built yet (the worker protocol, the project file format) are specified in [`specs/001-pattern-pipeline/contracts/`](../../specs/001-pattern-pipeline/contracts/) and get a page here when they land.
