# openplush-io

<!-- index: the glTF, FBX, OBJ, STL and PLY readers, what each reads and refuses, and its errors. -->

What `openplush-io` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

`Format` (OBJ, glTF, GLB, STL, PLY, FBX); `IoError` (`Parse`, `NonFiniteInput`, `MissingResource`, `Mesh`), deliberately not `#[non_exhaustive]`, so the pipeline's mapping to `EngineError` must name every variant and a new one fails to compile there; `Imported`, a scene reader's soup with the `Ignored` parts it read and did not apply (`Skin`, `MorphTargets`, `Animation`, `Primitive`, `Deformer`, `AnimationStack`, `Geometry`), returned by both `gltf` and `fbx`.

### glTF and GLB

`gltf::read(name, bytes, sidecar)` reads glTF or GLB into an `Imported`: the soup and the `Ignored` parts (`Skin`, `MorphTargets`, `Animation`, `Primitive`). It draws the file's `scene`, else scene 0, depth first with f64 world transforms, and reverses the winding under a mirroring one. Each drawn (node, triangle primitive) pair is one source primitive, and a node reached twice is refused, as is a node `rotation` whose squared length is further than `UNIT_QUATERNION_TOLERANCE` from 1; a `matrix` is read as written. The soup's `SourceMaterials` holds the file's materials list in order, names as written and an unnamed one called `material N`, including a material no primitive uses; a source primitive's material is its glTF primitive's own, or none for glTF's default material, and a material index past the list is `Parse`. A non-finite `TEXCOORD_0` value is `Parse` too, naming its vertex; the PLY reader also names the vertex, and the OBJ reader the line. `TEXCOORD_0` gives the corner UVs, and `extra_uv_sets` is the most sets beyond it that any drawn primitive has.

Accessors are read by its own checks, not the `gltf` crate's reader: their declared type must match the attribute (`POSITION` three floats; `TEXCOORD_n` two floats or normalized bytes or shorts; indices unsigned integers), they must lie inside their view, a view's stride must be at least its accessor's element size, and a sparse accessor is refused. A `POSITION` accessor past the accessors list is `Parse`, with the detail `mesh {m} primitive {p}: the POSITION accessor {a} does not exist`. Buffers come from GLB's chunk, a base64 `data:` URI, or the `sidecar` lookup by the primary-relative name.

### FBX

`fbx::read(bytes)` reads binary FBX 7.x into an `Imported`: the soup and the `Ignored` parts (`Deformer`, `AnimationStack`, `Geometry`), named by the file's object names. ASCII FBX (text starting `; FBX` or `FBXHeaderExtension:`) is `Parse`, saying so, as are bytes without the binary header and a version outside 7000 to 7999. Only `GlobalSettings`, `Objects` and the object-to-object `Connections` are read.

The models connected to the root are walked depth first, children in connection order, and a model reached twice is `Parse`. Each model's transform is the FBX SDK's stack, T Roff Rp Rpre R Rpost⁻¹ Rp⁻¹ Soff Sp S Sp⁻¹, from its `Properties70`, an absent property taking the SDK default; `R` turns in the model's `RotationOrder` (0 to 5; 6, spheric, is `Parse`), and pre-, post- and geometric rotation in XYZ. The geometric transform moves the model's own geometry, never its children's. The file's `UpAxis`, `FrontAxis`, `CoordAxis` and their signs become Y up, +Z front, +X right, absent settings meaning those axes already; an axis outside 0 to 2, an axis used twice, or a sign other than ±1 is `Parse`. `UnitScaleFactor` is not applied. Sines and cosines come from `libm`, so native and wasm32 agree to the bit. A transform that mirrors space reverses its triangles' winding.

A source primitive is one (model, material slot with polygons) pair of a `Mesh` geometry, numbered in walk order then slot order; `vertex` is the control-point index, and every primitive carries all of its mesh's control points. Slots come from the first `LayerElementMaterial` (`AllSame` or `ByPolygon`; without one every polygon is slot 0), and slot `k` is the model's `k`th connected material; a slot with no material is `Parse`. The soup's `SourceMaterials` holds every material in file order, an unnamed one called `material N`. Polygons are triangulated as OBJ's are, 3 to 1000 corners. The first `LayerElementUV` gives the corner UVs, `ByPolygonVertex` or `ByVertice`, `Direct` or `IndexToDirect`, and `extra_uv_sets` is the most further UV layers on any drawn mesh; a UV mapping or reference outside those, an index past its array or a non-finite UV is `Parse`. `NonFiniteInput` names every drawn control point that is not finite once moved into place. Normals, textures and the `Definitions` templates are not read.

### OBJ

`obj::read(bytes)`, OBJ to `TriangleSoup`. The OBJ reader reads `v`, `vt` (`v` defaults to 0) and `f` in all four corner forms, with negative indices counting back from the line and positive ones checked against the whole file. Every `v` line becomes a soup vertex in file order, used or not, and only its first three numbers are read. A corner's provenance is its primitive and its 0-based position index.

A primitive is a maximal run of `f` lines with no `o`, `g` or `usemtl` between them, numbered from 0, so one material each. The soup's `SourceMaterials` holds the `usemtl` names exactly as written (trailing blanks kept, a comment ends the name), each once in first-use order, including a name no face uses; a `\`-continued name keeps any blanks before the backslash and gains one space at the join; a primitive's material is the last `usemtl` before its faces, kept across `o` and `g`, or none if no `usemtl` came first. No MTL file is read, and `extra_uv_sets` is 0. A `v` line takes the primitive current where it appears. Corner UVs are present only if some face gives one.

A leading byte-order mark is skipped, `#` comments are stripped and `\`-continued lines joined. Normals, smoothing, lines and points are skipped; free-form geometry (`cstype`, `curv`, `curv2`, `surf`) is refused, since skipping it would lose surface.

Non-UTF-8 text (anywhere, comments included) and more than 2^32 − 1 vertices are refused as `Parse`; so is any statement that cannot be read, with a detail starting `line N:`: a non-number, a vertex with under three coordinates, a non-finite UV, a `usemtl` with no name, a face with under three corners, an index of 0 or out of range, and a polygon that crosses itself. `NonFiniteInput` lists every NaN or infinite vertex, used or not, in file order.

Triangulation never moves or adds a vertex: a quad splits on its shorter diagonal unless that diagonal lies outside; a larger polygon is ear-clipped on the plane its Newell normal faces, with exact predicates, and a corner that coincides with an ear's corner does not block it, so a hole joined by a doubled bridge edge clips. A polygon with no split whose triangles all turn its way is refused, unless its corners are collinear: then it is split and validation reports the zero-area triangles. Ear-clipping is quadratic in the polygon's corner count, so an OBJ face of more than 1000 corners is `Parse` (`line N:`).

### STL

`stl::read(bytes)`, STL to `TriangleSoup`: a file is binary when its length is exactly `84 + 50 * n` for the `n` its header declares, otherwise ASCII text starting with `solid` (after blanks and a byte-order mark). Each triangle gives three soup vertices in file order, joined by the weld; a corner's provenance is primitive 0 for binary and the solid's 0-based index for ASCII, with its vertex position; no UVs, and one material-less primitive per solid. Facet normals and attribute bytes are not read.

`Parse` details for ASCII start `line N:`; a file that is neither form names its length and the length its header declares; `NonFiniteInput` names every NaN or infinite vertex; more than `u32` vertices is `Parse`.

### PLY

`ply::read(bytes)`, PLY to `TriangleSoup`, ASCII or binary little or big endian. The `vertex` element (before `face`, each at most once) gives the soup's vertices in file order from scalar `x`, `y`, `z` of any numeric type and, if it has both, a texture pair (`s`/`t`, `u`/`v`, `texture_s`/`texture_t`, `texture_u`/`texture_v`) that each corner copies from its vertex; the `face` element's `vertex_indices` (or `vertex_index`) list gives polygons, triangulated, of 3 to 1000 corners. List count and item types must be integers, and an ASCII integer must fit its declared type; an ASCII `float` is rounded to 32 bits as a binary one is. A corner's provenance is primitive 0 and its vertex position; other properties and elements are read past.

`Parse` details start `line N:` or `byte N:` where the reader can say where; `NonFiniteInput` names every NaN or infinite vertex.

## Depends on

`openplush-geom`, `openplush-mesh`, `thiserror`, `gltf` (`names` only), `base64`, `fbxcel` (no default features), `libm`
