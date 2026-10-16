# Fixture generator: results

Run on 2026-10-14, Linux x86_64; rerun on 2026-10-16 when the source-seam fixtures were added. Standard library only; no third-party packages.

## Interpreters

The committed files were generated with Python 3.13.16. The tests, including the byte-for-byte regeneration check, pass unchanged on each of these, so all three produce identical bytes:

| Interpreter | `python3 -m unittest -v` |
|---|---|
| CPython 3.11.17 | 13 tests, OK |
| CPython 3.12.3 | 13 tests, OK |
| CPython 3.13.16 | 13 tests, OK |

## Commands

From `research/fixtures_gen/`:

```sh
python3 generate.py                 # writes fixtures/meshes, fixtures/defects, fixtures/seams
python3.11 -m unittest -v           # likewise with python3.12 and python3.13
mypy --strict --python-version 3.11 generate.py source_seams.py test_fixtures.py test_source_seams.py
```

`mypy` 2.4.0 ran from a scratch virtual environment outside the repository; it is a check, not a dependency, so it is not in `requirements.txt`.

## glTF validity

`fixtures/meshes/source-seams/capsule-uv.glb` passes the Khronos glTF Validator (`gltf-validator` 2.0.0-dev.3.10, npm, run from a scratch directory) with 0 errors and 0 warnings. Its one info, `UNUSED_OBJECT`, is `TEXCOORD_0`: no material samples a texture, as in any untextured export.
