//! The repaired mesh as OBJ text: positions, per-corner UVs and faces; no normals or materials.

use std::fmt::Write as _;

use openplush_pipeline::{FaceId, ManifoldMesh, VertexId};

/// One `v` per vertex, one `vt` per corner that has a UV, and one `f` per face in face order,
/// with `v/vt` for a corner that has a UV.
pub(crate) fn write(mesh: &ManifoldMesh) -> String {
    let mut text = String::new();
    for v in (0..mesh.vertex_count()).map(|i| VertexId::new(id(i))) {
        let p = mesh.position(v);
        // Writing to a `String` cannot fail.
        let _ = writeln!(text, "v {} {} {}", p.x, p.y, p.z);
    }
    let mut faces = String::new();
    let mut uvs = 0u64;
    for f in (0..mesh.face_count()).map(|i| FaceId::new(id(i))) {
        faces.push('f');
        let first = mesh.face_half_edge(f);
        for h in [first, mesh.next(first), mesh.next(mesh.next(first))] {
            let v = u64::from(mesh.origin(h).index()) + 1;
            match mesh.corner_uv(h) {
                Some(uv) => {
                    uvs += 1;
                    let _ = writeln!(text, "vt {} {}", uv.x, uv.y);
                    let _ = write!(faces, " {v}/{uvs}");
                }
                None => {
                    let _ = write!(faces, " {v}");
                }
            }
        }
        faces.push('\n');
    }
    text.push_str(&faces);
    text
}

fn id(i: usize) -> u32 {
    // Lossless: a mesh's element counts fit in u32 (checked at construction).
    i as u32
}
