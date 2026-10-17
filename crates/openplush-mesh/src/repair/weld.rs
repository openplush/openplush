use std::collections::BTreeMap;

use openplush_geom::DVec3;
use openplush_geom::epsilon::ModelTolerances;

use crate::disjoint::DisjointSet;
use crate::{TriangleSoup, VertexId};

/// Used vertices merged within the weld distance: each vertex's representative (the
/// smallest index it is welded to) and every group of two or more.
pub(crate) struct Weld {
    pub(crate) rep: Vec<u32>,
    pub(crate) groups: Vec<Vec<VertexId>>,
}

/// Joins any two used vertices within `distance`, transitively; validation calls it
/// too, so it judges topology as repair welds it. Vertices are bucketed into cubic
/// cells `distance` wide, measured from the used vertices' lowest corner, and each
/// is compared only with its own and the 26 neighbouring cells: a sort along one
/// axis would compare every pair in a flat, axis-aligned model.
// INVARIANT: soup indices are in range (`TriangleSoup::new`).
pub(crate) fn weld(soup: &TriangleSoup, distance: f64) -> Weld {
    let positions = soup.positions();
    let mut used = vec![false; positions.len()];
    for &i in soup.triangle_indices().iter().flatten() {
        used[i as usize] = true;
    }
    // Lossless: the vertex count fits in u32 (`TriangleSoup::new`).
    let order: Vec<u32> = (0..positions.len() as u32)
        .filter(|&i| used[i as usize])
        .collect();
    let low = order.iter().map(|&i| positions[i as usize]).fold(
        DVec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
        |m, p| DVec3::new(m.x.min(p.x), m.y.min(p.y), m.z.min(p.z)),
    );
    // Every coordinate offset is at most the diagonal, about 1e6 weld distances, so
    // the cell coordinates are small integers.
    let cell = |p: DVec3| {
        let d = (p - low) * (1.0 / distance);
        [d.x.floor() as i64, d.y.floor() as i64, d.z.floor() as i64]
    };
    let mut cells: BTreeMap<[i64; 3], Vec<u32>> = BTreeMap::new();
    for &i in &order {
        cells
            .entry(cell(positions[i as usize]))
            .or_default()
            .push(i);
    }
    let mut set = DisjointSet::new(positions.len());
    for &a in &order {
        let pa = positions[a as usize];
        let [x, y, z] = cell(pa);
        for neighbour in neighbours([x, y, z]) {
            for &b in cells.get(&neighbour).into_iter().flatten() {
                if b > a && (positions[b as usize] - pa).length() <= distance {
                    set.union(a, b);
                }
            }
        }
    }
    let rep: Vec<u32> = (0..positions.len() as u32).map(|i| set.find(i)).collect();
    let mut groups: BTreeMap<u32, Vec<VertexId>> = BTreeMap::new();
    for &i in &order {
        groups
            .entry(rep[i as usize])
            .or_default()
            .push(VertexId::new(i));
    }
    let groups = groups.into_values().filter(|g| g.len() > 1).collect();
    Weld { rep, groups }
}

/// Points every corner of a weld group at the group's smallest vertex ID, which
/// keeps its own position. Corner sources and UVs, and faces the weld collapses,
/// are left as they are. Returns the welded soup and each group merged.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "called by into_manifold, not yet written")
)]
pub(crate) fn merge(
    soup: &TriangleSoup,
    tol: &ModelTolerances,
) -> (TriangleSoup, Vec<Vec<VertexId>>) {
    let Weld { rep, groups } = weld(soup, tol.weld());
    (soup.reindexed(&rep), groups)
}

fn neighbours([x, y, z]: [i64; 3]) -> impl Iterator<Item = [i64; 3]> {
    (-1..=1).flat_map(move |dx| {
        (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| [x + dx, y + dy, z + dz]))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use openplush_geom::epsilon::ModelTolerances;
    use openplush_geom::{DVec2, DVec3};

    use super::merge;
    use crate::{SourceCorner, TriangleSoup, VertexId, validate};

    fn soup(positions: Vec<DVec3>, triangles: Vec<[u32; 3]>) -> TriangleSoup {
        let corners = (0..triangles.len() * 3)
            .map(|i| SourceCorner {
                primitive: 0,
                vertex: i as u32,
            })
            .collect();
        let uvs = (0..triangles.len() * 3)
            .map(|i| Some(DVec2::new(i as f64, 0.5)))
            .collect();
        TriangleSoup::new(positions, triangles, corners, Some(uvs)).unwrap()
    }

    fn tol(s: &TriangleSoup) -> ModelTolerances {
        ModelTolerances::for_diagonal(s.bbox_diagonal().unwrap()).unwrap()
    }

    fn v(i: u32) -> VertexId {
        VertexId::new(i)
    }

    /// A unit cube as a glTF exporter writes it with a UV island per side: each
    /// side has its own four vertices, wound outward.
    fn split_cube() -> TriangleSoup {
        let corner =
            |i: u32| DVec3::new(f64::from(i & 1), f64::from(i >> 1 & 1), f64::from(i >> 2));
        let sides: [[u32; 4]; 6] = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let positions = sides.iter().flatten().map(|&i| corner(i)).collect();
        let triangles = (0..6)
            .flat_map(|s| [[4 * s, 4 * s + 1, 4 * s + 2], [4 * s, 4 * s + 2, 4 * s + 3]])
            .collect();
        soup(positions, triangles)
    }

    #[test]
    fn a_cube_split_at_every_side_welds_closed_with_its_corners_untouched() {
        let split = split_cube();
        assert_eq!(validate(&split).unwelded().len(), 8);
        let (welded, merged) = merge(&split, &tol(&split));
        assert!(validate(&welded).unwelded().is_empty());
        // Closed and consistently wound by its raw indices alone: each directed
        // edge appears once, and so does its reverse.
        let directed: BTreeSet<[u32; 2]> = welded
            .triangle_indices()
            .iter()
            .flat_map(|&[a, b, c]| [[a, b], [b, c], [c, a]])
            .collect();
        assert_eq!(directed.len(), 36);
        assert!(directed.iter().all(|&[a, b]| directed.contains(&[b, a])));
        assert_eq!(merged.len(), 8);
        assert!(merged.iter().all(|g| g.len() == 3));
        assert_eq!(welded.positions(), split.positions());
        assert_eq!(welded.corner_sources(), split.corner_sources());
        assert_eq!(welded.corner_uvs(), split.corner_uvs());
        for (w, s) in welded
            .triangle_indices()
            .iter()
            .zip(split.triangle_indices())
        {
            for (&a, &b) in w.iter().zip(s) {
                assert_eq!(
                    welded.positions()[a as usize],
                    split.positions()[b as usize]
                );
            }
        }
    }

    #[test]
    fn a_weld_group_takes_its_smallest_id_wherever_it_first_appears() {
        let positions = vec![
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
            DVec3::new(1.0, 1e-7, 0.0),
            DVec3::new(1e-7, 1.0, 0.0),
        ];
        let s = soup(positions, vec![[2, 0, 1], [5, 4, 3]]);
        let (welded, merged) = merge(&s, &tol(&s));
        assert_eq!(welded.triangle_indices(), [[2, 0, 1], [1, 0, 3]]);
        assert_eq!(merged, [vec![v(0), v(4)], vec![v(1), v(5)]]);
        assert_eq!(welded.positions(), s.positions());
    }

    #[test]
    fn a_face_the_weld_collapses_is_kept_for_the_degenerate_step() {
        let positions = vec![
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(1e-7, 0.0, 0.0),
        ];
        let s = soup(positions, vec![[0, 1, 2], [0, 3, 2]]);
        let (welded, merged) = merge(&s, &tol(&s));
        assert_eq!(welded.triangle_indices(), [[0, 1, 2], [0, 0, 2]]);
        assert_eq!(merged, [vec![v(0), v(3)]]);
    }

    #[test]
    fn a_soup_with_nothing_to_weld_comes_back_unchanged() {
        let s = soup(
            vec![
                DVec3::new(0.0, 0.0, 0.0),
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
            ],
            vec![[0, 1, 2]],
        );
        let (welded, merged) = merge(&s, &tol(&s));
        assert_eq!(welded, s);
        assert!(merged.is_empty());
    }

    /// The weld distance of a soup spanning (0, 0, 0) to (3, 4, 0): diagonal 5.
    fn w() -> f64 {
        ModelTolerances::for_diagonal(5.0).unwrap().weld()
    }

    /// Two triangles sharing an edge, with vertices 0 and 3 at `a` and `b`, plus any
    /// `extra` vertices each in its own triangle on that edge.
    fn near(a: DVec3, b: DVec3, extra: &[DVec3]) -> TriangleSoup {
        let mut positions = vec![a, DVec3::new(3.0, 0.0, 0.0), DVec3::new(0.0, 4.0, 0.0), b];
        positions.extend_from_slice(extra);
        let triangles = std::iter::once(0)
            .chain(3..positions.len() as u32)
            .map(|i| [i, 1, 2])
            .collect();
        soup(positions, triangles)
    }

    #[test]
    fn welds_a_pair_across_a_cell_boundary_on_every_axis_and_side() {
        let axes = [DVec3::X, DVec3::Y, DVec3::Z];
        for axis in axes {
            for (a, b) in [(1.25, 0.75), (0.75, 1.25)] {
                let s = near(axis * (a * w()), axis * (b * w()), &[]);
                let (_, merged) = merge(&s, &tol(&s));
                assert_eq!(merged, [vec![v(0), v(3)]], "axis {axis:?}, {a} and {b}");
            }
        }
    }

    #[test]
    fn leaves_a_pair_in_neighbouring_cells_farther_apart_than_the_weld_distance() {
        let s = near(DVec3::ZERO, DVec3::new(1.5 * w(), 0.0, 0.0), &[]);
        let (welded, merged) = merge(&s, &tol(&s));
        assert!(merged.is_empty());
        assert_eq!(welded, s);
    }

    #[test]
    fn a_chain_welds_to_its_smallest_id_though_it_joins_through_a_larger_one() {
        let s = near(
            DVec3::ZERO,
            DVec3::new(1.2 * w(), 0.0, 0.0),
            &[DVec3::new(0.6 * w(), 0.0, 0.0)],
        );
        let (welded, merged) = merge(&s, &tol(&s));
        assert_eq!(merged, [vec![v(0), v(3), v(4)]]);
        assert_eq!(welded.triangle_indices(), [[0, 1, 2]; 3]);
    }
}
