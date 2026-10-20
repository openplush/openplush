//! Keys over a filled hole's patch, whose corners have no source vertex: patch vertex `n` of
//! the loop `<loop>` is `<loop>+<n>`, numbered in `VertexId` order.

use std::collections::BTreeSet;

use openplush_io::obj;
use openplush_mesh::{
    EdgeId, FaceId, LoopRef, ManifoldMesh, RepairPlan, RepairReport, SourceCorner, TriangleSoup,
    VertexId, into_manifold,
};
use openplush_project::{EdgeKey, FaceKey, ProjectError, Provenance, VertexKey};
use openplush_testkit::fixtures::{self, Kind};

type Setup = Box<dyn std::error::Error>;

fn corner(vertex: u32) -> SourceCorner {
    SourceCorner {
        primitive: 0,
        vertex,
    }
}

fn patch(rim: u32, n: u32) -> VertexKey {
    VertexKey::Patch {
        rim: LoopRef::new(corner(rim)),
        n,
    }
}

#[test]
fn a_patch_vertex_prints_as_its_loop_and_number() {
    assert_eq!(patch(12, 3).to_string(), "0.12+3");
    assert_eq!(VertexKey::from(corner(12)).to_string(), "0.12");
}

#[test]
fn a_patch_vertex_sorts_after_every_source_vertex_then_by_loop_and_number() {
    let edge = EdgeKey::new(patch(0, 0), corner(900));
    assert_eq!(edge.to_string(), "0.900:0.0+0");
    let two = EdgeKey::new(patch(12, 1), patch(9, 5));
    assert_eq!(two.to_string(), "0.9+5:0.12+1");
    let same_loop = FaceKey::new([patch(9, 7), patch(9, 2), corner(4).into()]);
    assert_eq!(same_loop.to_string(), "0.4:0.9+2:0.9+7");
}

#[test]
fn patch_keys_parse_their_own_text_back() -> Result<(), ProjectError> {
    let edge = EdgeKey::new(patch(12, 1), corner(9));
    assert_eq!(edge.to_string().parse::<EdgeKey>()?, edge);
    let face = FaceKey::new([patch(9, 7), patch(9, 2), corner(4).into()]);
    assert_eq!(face.to_string().parse::<FaceKey>()?, face);
    Ok(())
}

#[test]
fn only_the_canonical_patch_text_parses() {
    for text in [
        "0.9:0.12+03",
        "0.9:0.12+",
        "0.9:0.12+3+1",
        "0.9:+3",
        "0.9:0.12+-1",
        "0.9:0.12++3",
        "0.12+3:0.9",
        "0.12+3:0.9+1",
        "0.9:0.12+4294967296",
    ] {
        assert_eq!(
            text.parse::<EdgeKey>(),
            Err(ProjectError::BadKey { text: text.into() }),
            "{text:?}"
        );
    }
}

/// The sphere fixture with the faces above `cut` (a fraction of its height) and below
/// `floor` removed; `floor` 0 removes none below.
fn cut_sphere(cut: f64, floor: f64) -> Result<TriangleSoup, Setup> {
    let text = String::from_utf8(fixtures::load(Kind::Mesh, "sphere")?)?;
    let z: Vec<f64> = text
        .lines()
        .filter_map(|l| l.strip_prefix("v "))
        .map(|l| {
            l.split_whitespace()
                .nth(2)
                .ok_or("a vertex without z")?
                .parse::<f64>()
                .map_err(Setup::from)
        })
        .collect::<Result<_, _>>()?;
    let (lo, hi) = z
        .iter()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    let (top, bottom) = (lo + cut * (hi - lo), lo + floor * (hi - lo));
    let mut kept = String::new();
    for line in text.lines() {
        if let Some(face) = line.strip_prefix("f ") {
            let heights = face
                .split_whitespace()
                .map(|t| {
                    let i: usize = t.split('/').next().ok_or("an empty corner")?.parse()?;
                    Ok(z[i - 1])
                })
                .collect::<Result<Vec<f64>, Setup>>()?;
            if heights.iter().all(|&h| h > top) || heights.iter().all(|&h| h < bottom) {
                continue;
            }
        }
        kept.push_str(line);
        kept.push('\n');
    }
    Ok(obj::read(kept.as_bytes())?)
}

/// `soup` repaired with every loop it has filled, and the open repair's vertex count.
fn filled(soup: &TriangleSoup) -> Result<(ManifoldMesh, RepairReport, usize), Setup> {
    let (open, report) = into_manifold(soup, &RepairPlan::default())?;
    let plan = RepairPlan {
        fill_holes: report.holes().iter().map(|h| h.loop_ref()).collect(),
        ..RepairPlan::default()
    };
    let (m, report) = into_manifold(soup, &plan)?;
    Ok((m, report, open.vertex_count()))
}

fn edges(m: &ManifoldMesh) -> Result<Vec<EdgeId>, Setup> {
    Ok((0..u32::try_from(m.edge_count())?)
        .map(EdgeId::new)
        .collect())
}

fn faces(m: &ManifoldMesh) -> Result<Vec<FaceId>, Setup> {
    Ok((0..u32::try_from(m.face_count())?)
        .map(FaceId::new)
        .collect())
}

#[test]
fn every_patch_edge_and_face_has_a_key_that_resolves_to_itself() -> Result<(), Setup> {
    let (m, report, open) = filled(&cut_sphere(0.5, 0.0)?)?;
    assert!(m.vertex_count() > open, "the fill added no vertices");
    let p = Provenance::new(&m, report.holes());
    for e in edges(&m)? {
        let key = p.edge_key(e).ok_or("an edge has no key")?;
        assert_eq!(p.resolve_edge(key), vec![e], "{key}");
    }
    for f in faces(&m)? {
        let key = p.face_key(f).ok_or("a face has no key")?;
        assert_eq!(p.resolve_face(key), [f], "{key}");
    }
    Ok(())
}

/// Every patch vertex name in `m`'s edge keys.
fn patch_names(m: &ManifoldMesh, p: &Provenance<'_>) -> Result<BTreeSet<VertexKey>, Setup> {
    let mut names = BTreeSet::new();
    for e in edges(m)? {
        let key = p.edge_key(e).ok_or("an edge has no key")?;
        names.extend(
            key.ends()
                .into_iter()
                .filter(|k| matches!(k, VertexKey::Patch { .. })),
        );
    }
    Ok(names)
}

#[test]
fn patch_vertices_are_numbered_from_zero_in_vertex_order() -> Result<(), Setup> {
    let (m, report, open) = filled(&cut_sphere(0.5, 0.0)?)?;
    let p = Provenance::new(&m, report.holes());
    let rim = report.holes()[0].loop_ref();
    let added = u32::try_from(m.vertex_count() - open)?;
    let expected: BTreeSet<VertexKey> = (0..added).map(|n| VertexKey::Patch { rim, n }).collect();
    assert_eq!(patch_names(&m, &p)?, expected);
    let first = VertexId::new(u32::try_from(open)?);
    let h = m.vertex_out(first);
    let key = p.edge_key(h.edge()).ok_or("an edge has no key")?;
    assert!(
        key.ends().contains(&VertexKey::Patch { rim, n: 0 }),
        "{key}"
    );
    Ok(())
}

#[test]
fn each_filled_loop_numbers_its_own_patch_from_zero() -> Result<(), Setup> {
    let (m, report, _) = filled(&cut_sphere(0.6, 0.4)?)?;
    let holes: Vec<LoopRef> = report.holes().iter().map(|h| h.loop_ref()).collect();
    assert_eq!(holes.len(), 2, "{holes:?}");
    let p = Provenance::new(&m, report.holes());
    let names = patch_names(&m, &p)?;
    for rim in holes {
        let ns: Vec<u32> = names
            .iter()
            .filter_map(|k| match *k {
                VertexKey::Patch { rim: r, n } if r == rim => Some(n),
                _ => None,
            })
            .collect();
        assert!(!ns.is_empty(), "{rim:?} added no vertices");
        assert_eq!(
            ns,
            (0..u32::try_from(ns.len())?).collect::<Vec<_>>(),
            "{rim:?}"
        );
    }
    Ok(())
}

#[test]
fn patch_keys_round_trip_and_resolve_when_the_same_plan_is_repaired_again() -> Result<(), Setup> {
    let soup = cut_sphere(0.5, 0.0)?;
    let (first, first_report, _) = filled(&soup)?;
    let (again, again_report, _) = filled(&soup)?;
    let (written, read) = (
        Provenance::new(&first, first_report.holes()),
        Provenance::new(&again, again_report.holes()),
    );
    for e in edges(&first)? {
        let key = written.edge_key(e).ok_or("an edge has no key")?;
        let text: EdgeKey = key.to_string().parse()?;
        assert_eq!(read.resolve_edge(text), vec![e], "{key}");
    }
    for f in faces(&first)? {
        let key = written.face_key(f).ok_or("a face has no key")?;
        let text: FaceKey = key.to_string().parse()?;
        assert_eq!(read.resolve_face(text), [f], "{key}");
    }
    Ok(())
}

#[test]
fn a_patch_key_resolves_to_nothing_when_its_loop_is_not_filled() -> Result<(), Setup> {
    let soup = cut_sphere(0.5, 0.0)?;
    let (m, report, _) = filled(&soup)?;
    let (open, open_report) = into_manifold(&soup, &RepairPlan::default())?;
    let (written, read) = (
        Provenance::new(&m, report.holes()),
        Provenance::new(&open, open_report.holes()),
    );
    let mut patch_keys = 0;
    for e in edges(&m)? {
        let key = written.edge_key(e).ok_or("an edge has no key")?;
        if key
            .ends()
            .iter()
            .any(|k| matches!(k, VertexKey::Patch { .. }))
        {
            patch_keys += 1;
            assert!(read.resolve_edge(key).is_empty(), "{key}");
        }
    }
    assert!(patch_keys > 0);
    Ok(())
}
