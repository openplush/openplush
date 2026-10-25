# Fixture generator: results

Run on 2026-10-14, Linux x86_64; rerun on 2026-10-16 when the source-seam fixtures were added, on 2026-10-19 when the symmetry and faceted fixtures were added, and on 2026-10-20 when the quadruped was added. Standard library only; no third-party packages.

On 2026-10-25 the non-orientable strip was added and generated on Windows x86_64 with CPython 3.15.1: 50 tests, OK, and regenerating left every earlier file byte-identical. The three interpreters below were not rerun then.

## Interpreters

The committed files were generated with Python 3.13.16. The tests, including the byte-for-byte regeneration check, pass unchanged on each of these, so all three produce identical bytes:

| Interpreter | `python3 -m unittest -v` |
|---|---|
| CPython 3.11.17 | 48 tests, OK |
| CPython 3.12.3 | 48 tests, OK |
| CPython 3.13.16 | 48 tests, OK |

## Commands

From `research/fixtures_gen/`:

```sh
python3 generate.py                 # writes fixtures/meshes, fixtures/defects, fixtures/seams
python3.11 -m unittest -v           # likewise with python3.12 and python3.13
mypy --strict --python-version 3.11 generate.py source_seams.py source_dead_end.py source_faceted.py source_quadruped.py source_reshaped.py source_symmetry.py test_fixtures.py test_source_seams.py test_source_materials.py test_dead_end.py test_faceted.py test_quadruped.py test_reshaped.py test_symmetry.py
```

`mypy` 2.3.1 (2026-10-20; 2.4.0 on the earlier runs) ran from a scratch virtual environment outside the repository; it is a check, not a dependency, so it is not in `requirements.txt`.

## glTF validity

`fixtures/meshes/source-seams/capsule-uv.glb` and `capsule-hardshade.glb` pass the Khronos glTF Validator (`gltf-validator` 2.0.0-dev.3.10, npm, run from a scratch directory) with 0 errors and 0 warnings each. The one info, `UNUSED_OBJECT`, is `TEXCOORD_0`: no material samples a texture, as in any untextured export.
