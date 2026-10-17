# Research Lane — `research/`

Applies in addition to the root `AGENTS.md`. Spikes here are non-shipping reference code: nothing in `crates/`, `apps/`, or `packages/` imports them, and their findings land in the spike's `README.md`/`results.md` and the plan's Decision Ledger.

## Python rules
- Python 3.11+. Every function is fully type-hinted: `X | None`, built-in generics (`list[int]`), `type` aliases. No `Optional`, `Union`, `List`, `Dict`, `Tuple`, or implicit `Any`.
- Data read from disk is typed `object` until checked. Fail loudly with a raised exception; never fall back to a default.
- Deterministic output: seed every RNG, sort before printing, record interpreter and library versions in `results.md`.
- Dependencies pinned (`==`) in the spike's `requirements.txt`, license-allowlisted like the engine's. One exception, once [ADR 0011](../docs/adr/0011-blender-research-tool.md) is accepted: Blender's `bpy` (GPL-3.0), only in a spike directory that is itself GPL-licensed and that nothing else imports.
- Tests are `unittest` oracles independent of the implementation; each test names the failure it prevents. Run: `python3 -m unittest -v` from the spike directory.
- The root engineering and comment rules apply. Citing decisions and papers in a spike's docstrings is expected; that is what a spike is for.
- `.specify/**` Python is Spec Kit-managed: never hand-edit it.
