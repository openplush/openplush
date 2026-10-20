//! Provenance keys: their text form, and how each resolves against one repaired mesh.

use openplush_io::obj;
use openplush_mesh::{
    EdgeId, FaceId, ManifoldMesh, RepairPlan, SourceCorner, VertexId, into_manifold,
};
use openplush_project::{EdgeKey, FaceKey, ProjectError, Provenance};

type Setup = Box<dyn std::error::Error>;

fn mesh(text: &str) -> Result<ManifoldMesh, Setup> {
    let soup = obj::read(text.as_bytes())?;
    Ok(into_manifold(&soup, &RepairPlan::default())?.0)
}

fn corner(vertex: u32) -> SourceCorner {
    SourceCorner {
        primitive: 0,
        vertex,
    }
}

const TETRA: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 1 4 3\n";

/// The tetrahedron with vertex 1 given again as vertex 5, which the last face uses: the weld
/// merges them, so the merged vertex holds corners from source vertices 0 and 4.
const UNWELDED: &str =
    "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nv 0 0 0\nf 1 3 2\nf 1 2 4\nf 2 3 4\nf 5 4 3\n";

/// One triangle given twice, once each way round: a double-sided sheet.
const SHEET: &str = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\nf 1 3 2\n";

#[test]
fn keys_sort_their_ends_in_numeric_order_and_print_as_text() {
    let edge = EdgeKey::new(corner(12), corner(9));
    assert_eq!(edge.ends(), [corner(9).into(), corner(12).into()]);
    assert_eq!(edge.to_string(), "0.9:0.12");
    let face = FaceKey::new([
        corner(40),
        SourceCorner {
            primitive: 1,
            vertex: 2,
        },
        corner(7),
    ]);
    assert_eq!(face.to_string(), "0.7:0.40:1.2");
}

#[test]
fn keys_parse_their_own_text_back() -> Result<(), ProjectError> {
    let edge = EdgeKey::new(corner(12), corner(9));
    assert_eq!(edge.to_string().parse::<EdgeKey>()?, edge);
    let face = FaceKey::new([corner(3), corner(1), corner(2)]);
    assert_eq!(face.to_string().parse::<FaceKey>()?, face);
    assert_eq!(
        "4294967295.0:4294967295.1".parse::<EdgeKey>()?.to_string(),
        "4294967295.0:4294967295.1"
    );
    Ok(())
}

#[test]
fn only_the_canonical_text_parses() {
    let edges = [
        "",
        "0.12",
        "0.12:0.9",
        "0.9:0.12:0.13",
        "0.09:0.12",
        "+0.9:0.12",
        " 0.9:0.12",
        "0.9:0.12 ",
        "0.9;0.12",
        "0:0.12",
        "a.9:0.12",
        "0..9:0.12",
        "4294967296.0:4294967296.1",
    ];
    for text in edges {
        assert_eq!(
            text.parse::<EdgeKey>(),
            Err(ProjectError::BadKey { text: text.into() }),
            "{text:?}"
        );
    }
    for text in ["0.1:0.2", "0.3:0.2:0.1", "0.1:0.2:0.3:0.4"] {
        assert_eq!(
            text.parse::<FaceKey>(),
            Err(ProjectError::BadKey { text: text.into() }),
            "{text:?}"
        );
    }
}

#[test]
fn a_bad_key_says_what_it_was_given() {
    assert_eq!(
        ProjectError::BadKey {
            text: "0.12:0.9".into()
        }
        .to_string(),
        "\"0.12:0.9\" is not a provenance key"
    );
}

#[test]
fn every_edge_and_face_resolves_to_itself() -> Result<(), Setup> {
    let m = mesh(TETRA)?;
    let p = Provenance::new(&m, &[]);
    for e in (0..u32::try_from(m.edge_count())?).map(EdgeId::new) {
        let key = p.edge_key(e).ok_or("an edge of a read mesh has no key")?;
        assert_eq!(p.resolve_edge(key), vec![e], "{key}");
    }
    for f in (0..u32::try_from(m.face_count())?).map(FaceId::new) {
        let key = p.face_key(f).ok_or("a face of a read mesh has no key")?;
        assert_eq!(p.resolve_face(key), [f], "{key}");
    }
    Ok(())
}

#[test]
fn a_vertex_is_named_by_its_lowest_source_vertex() -> Result<(), Setup> {
    let m = mesh(UNWELDED)?;
    let p = Provenance::new(&m, &[]);
    let merged = (0..u32::try_from(m.vertex_count())?)
        .map(VertexId::new)
        .find(|&v| m.position(v).length() == 0.0)
        .ok_or("the origin vertex is missing")?;
    let h = m.vertex_out(merged);
    let key = p.edge_key(h.edge()).ok_or("an edge has no key")?;
    assert!(key.ends().contains(&corner(0).into()), "{key}");
    assert!(!key.ends().contains(&corner(4).into()), "{key}");
    Ok(())
}

#[test]
fn a_key_written_with_any_source_vertex_of_a_merged_vertex_resolves() -> Result<(), Setup> {
    let m = mesh(UNWELDED)?;
    let p = Provenance::new(&m, &[]);
    // Source vertices 4 and 3 (OBJ's 5 and 4) share an edge in the last face.
    let by_other = p.resolve_edge(EdgeKey::new(corner(4), corner(3)));
    let by_lowest = p.resolve_edge(EdgeKey::new(corner(0), corner(3)));
    assert_eq!(by_other.len(), 1);
    assert_eq!(by_other, by_lowest);
    Ok(())
}

#[test]
fn a_key_with_no_such_edge_resolves_to_nothing() -> Result<(), Setup> {
    let m = mesh(TETRA)?;
    let p = Provenance::new(&m, &[]);
    assert!(
        p.resolve_edge(EdgeKey::new(corner(0), corner(9)))
            .is_empty()
    );
    assert!(
        p.resolve_edge(EdgeKey::new(corner(1), corner(1)))
            .is_empty()
    );
    assert!(
        p.resolve_face(FaceKey::new([corner(0), corner(1), corner(9)]))
            .is_empty()
    );
    Ok(())
}

#[test]
fn both_layers_of_a_double_sided_sheet_share_a_face_key() -> Result<(), Setup> {
    let m = mesh(SHEET)?;
    let p = Provenance::new(&m, &[]);
    let key = FaceKey::new([corner(0), corner(1), corner(2)]);
    assert_eq!(p.face_key(FaceId::new(0)), Some(key));
    assert_eq!(p.face_key(FaceId::new(1)), Some(key));
    assert_eq!(p.resolve_face(key), [FaceId::new(0), FaceId::new(1)]);
    Ok(())
}

#[test]
fn every_copy_of_a_split_edge_resolves() -> Result<(), Setup> {
    // Three faces on the edge between source vertices 0 and 1: repair splits it.
    let book = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 -1 0\nv 0 0 1\nf 1 2 3\nf 2 1 4\nf 1 2 5\n";
    let m = mesh(book)?;
    let p = Provenance::new(&m, &[]);
    let copies = p.resolve_edge(EdgeKey::new(corner(0), corner(1)));
    assert!(copies.len() >= 2, "{copies:?}");
    let mut grouped: Vec<EdgeId> = m.edge_copies().iter().flatten().copied().collect();
    grouped.sort();
    assert!(copies.iter().all(|e| grouped.contains(e)), "{copies:?}");
    Ok(())
}

#[test]
fn the_lowest_source_vertex_names_a_vertex_whatever_its_corners_order() -> Result<(), Setup> {
    // UNWELDED's faces in every cyclic order: the merged vertex's corners come round in a
    // different order each time, and its name stays source vertex 0.
    let faces = ["f 1 3 2", "f 1 2 4", "f 2 3 4", "f 5 4 3"];
    let vertices = "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 0 0 1\nv 0 0 0\n";
    let mut seen = None;
    for shift in 0..faces.len() {
        let mut text = vertices.to_owned();
        for k in 0..faces.len() {
            text.push_str(faces[(k + shift) % faces.len()]);
            text.push('\n');
        }
        let m = mesh(&text)?;
        let p = Provenance::new(&m, &[]);
        let mut keys: Vec<EdgeKey> = (0..u32::try_from(m.edge_count())?)
            .map(|e| p.edge_key(EdgeId::new(e)).ok_or("an edge has no key"))
            .collect::<Result<_, _>>()?;
        keys.sort();
        assert!(
            keys.iter().all(|k| !k.ends().contains(&corner(4).into())),
            "{keys:?}"
        );
        assert_eq!(
            *seen.get_or_insert_with(|| keys.clone()),
            keys,
            "shift {shift}"
        );
    }
    Ok(())
}

#[test]
fn every_resolution_is_strictly_ascending() -> Result<(), Setup> {
    use openplush_testkit::fixtures::{self, Kind};
    let plan = RepairPlan {
        drop_degenerates: false,
        ..RepairPlan::default()
    };
    for (kind, name) in [(Kind::Defect, "duplicate-face"), (Kind::Mesh, "capsule")] {
        let soup = obj::read(&fixtures::load(kind, name)?)?;
        for plan in [RepairPlan::default(), plan.clone()] {
            let (m, _) = into_manifold(&soup, &plan)?;
            let p = Provenance::new(&m, &[]);
            for e in (0..u32::try_from(m.edge_count())?).map(EdgeId::new) {
                let key = p.edge_key(e).ok_or("an edge has no key")?;
                let landed = p.resolve_edge(key);
                assert!(
                    landed.windows(2).all(|w| w[0] < w[1]),
                    "{name} {key}: {landed:?}"
                );
            }
        }
    }
    Ok(())
}
