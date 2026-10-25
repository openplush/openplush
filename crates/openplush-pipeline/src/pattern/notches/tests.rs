//! The meets graph: seams matched through a merge are one seam in it, so neither takes a mark
//! count a seam meeting the other has.
#![allow(clippy::expect_used, clippy::panic, reason = "test setup")]

use openplush_geom::DVec2;
use openplush_mesh::VertexId;
use openplush_pattern::Outline;
use openplush_seams::SeamChainId;

use super::{Side, meets};
use crate::pattern::group::{EdgeSeam, alike, group};

fn side(chart: usize, vertices: &[u32]) -> Side {
    let vertices: Vec<VertexId> = vertices.iter().map(|&v| VertexId::new(v)).collect();
    let edges = vertices
        .windows(2)
        .map(|_| {
            (
                DVec2::new(0.0, 0.0),
                DVec2::new(1.0, 0.0),
                DVec2::new(0.0, -1.0),
            )
        })
        .collect();
    Side {
        chart,
        vertices,
        edges,
    }
}

#[test]
fn a_seam_matched_to_another_meets_what_either_meets() {
    // Two identical squares, edge 0 of each on chains 3 and 7: merged, 7 becomes 3.
    let square = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(40.0, 0.0),
        DVec2::new(40.0, 40.0),
        DVec2::new(0.0, 40.0),
    ])
    .expect("valid");
    let on = |chain| {
        let mut e = vec![EdgeSeam::default(); 4];
        e[0].chain = Some(SeamChainId::new(chain));
        e
    };
    let edges = [on(3), on(7)];
    let al = alike(&group(&[&square, &square], &edges), &edges);
    assert_eq!(al.of(SeamChainId::new(7)), SeamChainId::new(3));
    // Chain 7 ends where chain 5 does, on the member's chart.
    let bundles = vec![
        (SeamChainId::new(3), vec![side(0, &[0, 1])]),
        (SeamChainId::new(5), vec![side(1, &[1, 9])]),
        (SeamChainId::new(7), vec![side(1, &[8, 1])]),
    ];
    let graph = meets(&bundles, &al);
    assert!(
        graph[&SeamChainId::new(3)].contains(&SeamChainId::new(5)),
        "{graph:?}"
    );
    assert!(!graph.contains_key(&SeamChainId::new(7)), "{graph:?}");
}
