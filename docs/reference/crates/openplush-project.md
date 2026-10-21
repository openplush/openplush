# openplush-project

<!-- index: provenance keys for vertices, edges and faces, and how they resolve against a repaired mesh. -->

What `openplush-project` contains **today**, and what it actually depends on. What it *should* own is in [Which crate owns what](README.md#which-crate-owns-what).

## Public items

### Provenance keys

A `VertexKey` is a source vertex, written `<primitive>.<vertex>` (`0.12`), or vertex `n` of a filled loop's patch, written `<loop>+<n>` (`0.12+3`); every source vertex sorts before every patch vertex, and patch vertices sort by loop, then `n`. `EdgeKey` names an edge by its two ends and `FaceKey` a face by its three corners, each sorted in `VertexKey` order and joined by `:` (`0.9:0.12`). Their `FromStr` takes only that canonical text, else `ProjectError::BadKey`.

### Provenance

`Provenance::new(mesh, holes)` indexes one repaired mesh, with `holes` from the report of the repair that made it: a filled loop's patch vertices, the vertices of its added faces with no source corner, are numbered from 0 in `VertexId` order. `edge_key` names an edge by each end's lowest source vertex, or its patch place; `face_key` names a face by its own three corners, a patch corner by its vertex's key.

`resolve_edge` returns every edge between a vertex holding one end and a vertex holding the other: one edge usually, every copy of a split edge, none when the mesh has no such edge. `resolve_face` returns every face with exactly those corners: one, or both layers of a double-sided sheet. A key written against one repair resolves against another, as long as the faces that carry it survive.

## Depends on

`openplush-mesh`, `thiserror`
