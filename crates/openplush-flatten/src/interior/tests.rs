use std::f64::consts::FRAC_PI_2;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;
use openplush_testkit::oracles::{Chart, developable, injective};

use super::energy::{energy, rest_shapes};
use super::fixtures::half_cylinder;
use super::{Solve, Step, refuse, solve_failed};
use crate::FlattenError;
use crate::boundary::BoundaryError;
use crate::boundary::fixtures::{Faces, faces, grid, rings};
use crate::injectivity;
use crate::solver::SolveError;

mod gradient;
mod stops;

const CHART: ChartId = ChartId::new(3);

/// The chart's layout once the iteration settles.
fn flatten(
    chart: ChartId,
    xyz: &[DVec3],
    faces: &[(FaceId, [VertexId; 3])],
) -> Result<Vec<(VertexId, DVec2)>, FlattenError> {
    let mut solve = Solve::start(chart, xyz, faces.to_vec())?;
    while solve.iterate()? == Step::Improved {}
    Ok(solve.layout())
}

/// The northern hemisphere of the unit sphere, 16 rings (1536 triangles), as in the
/// `research/boundary_pinned/` corpus's `cap-90`.
fn hemisphere() -> (Vec<DVec3>, Faces) {
    rings(16, |r, a| {
        let theta = FRAC_PI_2 * r;
        DVec3::new(
            libm::sin(theta) * libm::cos(a),
            libm::sin(theta) * libm::sin(a),
            libm::cos(theta),
        )
    })
}

fn dense(layout: &[(VertexId, DVec2)], n: usize) -> Vec<DVec2> {
    let mut uv = vec![DVec2::ZERO; n];
    for &(v, p) in layout {
        uv[v.index() as usize] = p;
    }
    uv
}

fn corners(f: &Faces) -> Vec<[VertexId; 3]> {
    f.iter().map(|&(_, t)| t).collect()
}

fn mean_energy(xyz: &[DVec3], f: &Faces, uv: &[DVec2]) -> Option<f64> {
    energy(&rest_shapes(xyz, f).ok()?, f, uv)
}

#[test]
fn a_half_cylinder_unrolls_with_no_distortion() -> Result<(), FlattenError> {
    let (xyz, f) = half_cylinder();
    let uv = dense(&flatten(CHART, &xyz, &f)?, xyz.len());
    let c = corners(&f);
    let chart = Chart {
        xyz: &xyz,
        uv: &uv,
        faces: &c,
    };
    assert_eq!(injective(&chart), Ok(()));
    assert_eq!(developable(&chart, 1e-6), Ok(()));
    assert!(matches!(mean_energy(&xyz, &f, &uv), Some(e) if (e - 4.0).abs() <= 5e-4));
    Ok(())
}

#[test]
fn a_hemisphere_pinned_to_its_boundary_settles_near_the_spike_energy() -> Result<(), FlattenError> {
    // research/boundary_pinned/results.md, cap-90: pinned SD mean 5.200, no face flipped.
    let (xyz, f) = hemisphere();
    let uv = dense(&flatten(CHART, &xyz, &f)?, xyz.len());
    let c = corners(&f);
    let chart = Chart {
        xyz: &xyz,
        uv: &uv,
        faces: &c,
    };
    assert_eq!(injective(&chart), Ok(()));
    assert!(matches!(mean_energy(&xyz, &f, &uv), Some(e) if (e - 5.2).abs() <= 0.01));
    Ok(())
}

#[test]
fn the_energy_never_rises_and_no_iterate_flips_a_face() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f.clone())?;
    let mut last = solve.energy();
    let mut iterations = 0;
    loop {
        let step = solve.iterate()?;
        assert!(solve.energy() <= last, "iteration {iterations}");
        last = solve.energy();
        let uv = dense(&solve.layout(), xyz.len());
        let tris: Vec<_> = f
            .iter()
            .map(|&(id, t)| (id, t.map(|v| uv[v.index() as usize])))
            .collect();
        assert_eq!(injectivity::verify(CHART, &tris), Ok(()));
        iterations += 1;
        if step == Step::Settled {
            break;
        }
        assert!(iterations < 200, "the cap must settle the solve");
    }
    Ok(())
}

#[test]
fn a_settled_solve_stays_settled_and_unchanged() -> Result<(), FlattenError> {
    let (xyz, f) = half_cylinder();
    let mut solve = Solve::start(CHART, &xyz, f.clone())?;
    while solve.iterate()? == Step::Improved {}
    let before = solve.layout();
    assert_eq!(solve.iterate()?, Step::Settled);
    assert_eq!(solve.layout(), before);
    Ok(())
}

#[test]
fn the_boundary_keeps_every_edge_at_its_3d_length() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let uv = dense(&flatten(CHART, &xyz, &f)?, xyz.len());
    // The rim is the last ring: vertices 1 + 3·16·15 = 721 on.
    let rim: Vec<usize> = (721..xyz.len()).collect();
    for (i, &a) in rim.iter().enumerate() {
        let b = rim[(i + 1) % rim.len()];
        let (flat, solid) = ((uv[b] - uv[a]).length(), (xyz[b] - xyz[a]).length());
        assert!((flat - solid).abs() <= 1e-12, "edge {a}-{b}");
    }
    Ok(())
}

#[test]
fn a_flat_chart_comes_back_as_itself() -> Result<(), FlattenError> {
    let (xyz, f) = (
        (0..=4)
            .flat_map(|r| (0..=4).map(move |c| (c, r)))
            .map(|(c, r)| DVec3::new(0.5 * f64::from(c), 0.5 * f64::from(r), 2.0))
            .collect::<Vec<_>>(),
        grid(4, 4),
    );
    let uv = dense(&flatten(CHART, &xyz, &f)?, xyz.len());
    // Equal up to a rigid motion: every distance between vertices is preserved.
    for (a, pa) in uv.iter().enumerate() {
        for (b, pb) in uv.iter().enumerate() {
            let want = (xyz[a] - xyz[b]).length();
            assert!(((*pa - *pb).length() - want).abs() <= 1e-9);
        }
    }
    Ok(())
}

#[test]
fn a_chart_with_no_interior_vertex_is_its_prescribed_boundary() -> Result<(), FlattenError> {
    let xyz = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 4.0),
    ];
    let layout = flatten(CHART, &xyz, &faces(&[[0, 1, 2]]))?;
    let uv = dense(&layout, 3);
    for (a, b, want) in [(0, 1, 3.0), (1, 2, 4.0), (0, 2, 5.0)] {
        assert!(((uv[a] - uv[b]).length() - want).abs() <= 1e-12);
    }
    Ok(())
}

#[test]
fn scaling_the_chart_scales_the_layout() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let big: Vec<DVec3> = xyz.iter().map(|&p| p * 1000.0).collect();
    let (small, large) = (flatten(CHART, &xyz, &f)?, flatten(CHART, &big, &f)?);
    for ((_, a), (_, b)) in small.iter().zip(&large) {
        assert!((*a * 1000.0 - *b).length() <= 1e-6);
    }
    Ok(())
}

#[test]
fn the_same_chart_gives_the_same_bits() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let bits = |layout: Vec<(VertexId, DVec2)>| -> Vec<(u32, u64, u64)> {
        layout
            .iter()
            .map(|(v, p)| (v.index(), p.x.to_bits(), p.y.to_bits()))
            .collect()
    };
    assert_eq!(
        bits(flatten(CHART, &xyz, &f)?),
        bits(flatten(CHART, &xyz, &f)?)
    );
    Ok(())
}

#[test]
fn the_layout_lists_each_used_vertex_once_in_vertex_order() -> Result<(), FlattenError> {
    let (xyz, f) = half_cylinder();
    let mut padded = xyz.clone();
    padded.push(DVec3::new(9.0, 9.0, 9.0));
    let layout = flatten(CHART, &padded, &f)?;
    let ids: Vec<u32> = layout.iter().map(|(v, _)| v.index()).collect();
    assert_eq!(ids, (0..xyz.len() as u32).collect::<Vec<_>>());
    Ok(())
}

#[test]
fn charts_that_cannot_be_flattened_are_refused_with_their_chart() {
    let (xyz, disk) = (
        (0..=3)
            .flat_map(|r| (0..=3).map(move |c| DVec3::new(f64::from(c), f64::from(r), 0.0)))
            .collect::<Vec<_>>(),
        grid(3, 3),
    );
    let annulus: Faces = disk
        .into_iter()
        .filter(|(f, _)| f.index() / 2 != 4)
        .collect();
    assert_eq!(
        flatten(CHART, &xyz, &annulus),
        Err(FlattenError::NotADisk { chart: CHART })
    );
    assert_eq!(
        flatten(CHART, &xyz, &[]),
        Err(FlattenError::EmptyChart { chart: CHART })
    );
    let sliver = faces(&[[0, 1, 1]]);
    assert_eq!(
        flatten(CHART, &xyz, &sliver),
        Err(FlattenError::Degenerate {
            chart: CHART,
            face: FaceId::new(0)
        })
    );
}

#[test]
fn each_refusal_maps_to_the_variant_that_names_it() {
    let (xyz, f) = half_cylinder();
    let face = FaceId::new(5);
    for (from, to) in [
        (
            BoundaryError::Empty,
            FlattenError::EmptyChart { chart: CHART },
        ),
        (
            BoundaryError::BadVertex { face },
            FlattenError::Degenerate { chart: CHART, face },
        ),
        (
            BoundaryError::Degenerate { face },
            FlattenError::Degenerate { chart: CHART, face },
        ),
        (
            BoundaryError::NotADisk,
            FlattenError::NotADisk { chart: CHART },
        ),
        (
            BoundaryError::Solve(SolveError::NotFinite),
            FlattenError::SolveFailed { chart: CHART },
        ),
    ] {
        assert_eq!(refuse(CHART, &xyz, &f, from.clone()), to, "{from:?}");
    }
    assert_eq!(
        solve_failed(CHART, SolveError::NotPositiveDefinite),
        FlattenError::SolveFailed { chart: CHART }
    );
}

#[test]
fn the_iteration_cap_settles_the_solve_on_that_iteration_and_no_later_one_moves_it()
-> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    for cap in [1, 2] {
        let mut solve = Solve::start(CHART, &xyz, f.clone())?;
        let start = solve.energy();
        solve.cap = cap;
        for _ in 1..cap {
            assert_eq!(solve.iterate()?, Step::Improved);
        }
        assert_eq!(solve.iterate()?, Step::Settled, "cap {cap}");
        assert!(solve.energy() < start);
        let (layout, energy) = (solve.layout(), solve.energy());
        assert_eq!(solve.iterate()?, Step::Settled);
        assert_eq!((solve.layout(), solve.energy()), (layout, energy));
    }
    Ok(())
}

#[test]
fn the_solve_keeps_going_until_the_gain_is_negligible() -> Result<(), FlattenError> {
    let (xyz, f) = hemisphere();
    let mut solve = Solve::start(CHART, &xyz, f.clone())?;
    assert_eq!(solve.iterate()?, Step::Improved);
    let after_one = solve.energy();
    while solve.iterate()? == Step::Improved {}
    assert!(solve.energy() < after_one);
    // Pinned from the settled run: a looser stop would leave the energy visibly higher.
    assert!(
        (solve.energy() - 5.200_134_098).abs() <= 1e-6,
        "{}",
        solve.energy()
    );
    Ok(())
}

#[test]
fn a_boundary_that_will_not_close_is_reported_with_its_gap_and_a_dart() {
    let (xyz, f) = hemisphere();
    for refusal in [BoundaryError::NotClosed, BoundaryError::BadInput] {
        assert!(matches!(
            refuse(CHART, &xyz, &f, refusal),
            FlattenError::BoundaryNotRealizable { chart, residual, dart: Some(_) }
                if chart == CHART && residual.is_finite() && residual >= 0.0
        ));
    }
}
