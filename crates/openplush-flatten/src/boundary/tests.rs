use std::f64::consts::TAU;

use openplush_geom::{DVec2, DVec3};
use openplush_mesh::{FaceId, VertexId};
use openplush_testkit::oracles::outline_simple;

use super::fixtures::{Faces, faces, grid};
use super::{BoundaryError, curvature, prescribe, topology};

/// `fixtures::grid`'s faces with vertex `row * (cols + 1) + col` at `place(col, row)`.
fn surface(cols: u32, rows: u32, place: impl Fn(f64, f64) -> DVec3) -> (Vec<DVec3>, Faces) {
    let xyz = (0..=rows)
        .flat_map(|r| (0..=cols).map(move |c| (c, r)))
        .map(|(c, r)| place(f64::from(c), f64::from(r)))
        .collect();
    (xyz, grid(cols, rows))
}

/// A cap of the sphere of radius 2, curved enough that the boundary must turn to close.
fn cap() -> (Vec<DVec3>, Faces) {
    surface(6, 6, |c, r| {
        let p = DVec3::new(c / 6.0 - 0.5, r / 6.0 - 0.5, 1.0);
        p * (2.0 / p.length())
    })
}

/// A saddle, curved the other way from the cap.
fn saddle() -> (Vec<DVec3>, Faces) {
    surface(6, 5, |c, r| {
        let (x, y) = (c / 6.0 - 0.5, r / 5.0 - 0.5);
        DVec3::new(x, y, 0.8 * (x * x - y * y))
    })
}

/// Every distance between two flat boundary corners equals the same distance in `reference`.
fn congruent(flat: &[(VertexId, DVec2)], reference: impl Fn(VertexId) -> DVec2) -> bool {
    flat.iter().all(|&(a, pa)| {
        flat.iter().all(|&(b, pb)| {
            let want = (reference(a) - reference(b)).length();
            ((pa - pb).length() - want).abs() <= 1e-9
        })
    })
}

#[test]
fn a_flat_chart_keeps_its_boundary_shape() -> Result<(), BoundaryError> {
    // A plane tilted in space: the boundary needs no turning change, so it lies flat as it is.
    let (u, v) = (DVec3::new(0.6, 0.0, 0.8), DVec3::new(0.0, 1.0, 0.0));
    let (xyz, f) = surface(4, 3, |c, r| u * (0.7 * c) + v * (0.5 * r));
    let flat = prescribe(&xyz, &f)?;
    let id = |v: VertexId| (f64::from(v.index() % 5), f64::from(v.index() / 5));
    assert!(congruent(&flat, |v| DVec2::new(
        0.7 * id(v).0,
        0.5 * id(v).1
    )));
    Ok(())
}

#[test]
fn a_half_cylinder_unrolls_to_its_rectangle() -> Result<(), BoundaryError> {
    // Every quad of a prism is a flat rectangle, so the strip is exactly developable.
    let step = std::f64::consts::PI / 8.0;
    let (xyz, f) = surface(8, 3, |c, r| {
        DVec3::new((c * step).cos(), (c * step).sin(), 0.4 * r)
    });
    let chord = 2.0 * (step / 2.0).sin();
    let flat = prescribe(&xyz, &f)?;
    let id = |v: VertexId| (f64::from(v.index() % 9), f64::from(v.index() / 9));
    assert!(congruent(&flat, |v| DVec2::new(
        chord * id(v).0,
        0.4 * id(v).1
    )));
    Ok(())
}

#[test]
fn a_curved_chart_keeps_every_boundary_length_and_lies_flat_without_crossing()
-> Result<(), BoundaryError> {
    for (xyz, f) in [cap(), saddle()] {
        let flat = prescribe(&xyz, &f)?;
        let n = flat.len();
        for (i, &(a, pa)) in flat.iter().enumerate() {
            let (b, pb) = flat[(i + 1) % n];
            let want = (xyz[b.index() as usize] - xyz[a.index() as usize]).length();
            assert!(
                ((pb - pa).length() - want).abs() <= 1e-12,
                "edge from {a:?}"
            );
        }
        let outline: Vec<DVec2> = flat.iter().map(|&(_, p)| p).collect();
        assert_eq!(outline_simple(&outline), Ok(()));
    }
    Ok(())
}

#[test]
fn the_curvature_target_turns_once_in_total_before_any_closing() -> Result<(), BoundaryError> {
    // Gauss–Bonnet on a disk: boundary turning plus interior curvature is 2π, and laying the
    // interior flat moves all of that curvature onto the boundary.
    for (xyz, f) in [cap(), saddle()] {
        let ring = topology::boundary_loop(&f)?;
        let target = curvature::turning(&curvature::intrinsic(&xyz, &f)?, &ring)?;
        assert!((target.iter().sum::<f64>() - TAU).abs() <= 1e-9);
    }
    Ok(())
}

#[test]
fn mirror_image_corners_of_a_symmetric_cap_turn_alike() -> Result<(), BoundaryError> {
    // The cap and its diagonal triangulation are both symmetric under swapping column and
    // row, so a corner and its mirror image must get the same turning, each its own.
    let (xyz, f) = cap();
    let ring = topology::boundary_loop(&f)?;
    let target = curvature::turning(&curvature::intrinsic(&xyz, &f)?, &ring)?;
    let at = |v: u32| ring.iter().position(|&r| r == v).map(|i| target[i]);
    for &v in &ring {
        let (a, b) = (at(v), at((v % 7) * 7 + v / 7));
        assert!(
            matches!((a, b), (Some(a), Some(b)) if (a - b).abs() <= 1e-12),
            "{v}"
        );
    }
    Ok(())
}

#[test]
fn scaling_the_chart_scales_its_boundary() -> Result<(), BoundaryError> {
    let (xyz, f) = cap();
    let big: Vec<DVec3> = xyz.iter().map(|&p| p * 1000.0).collect();
    for ((_, small), (_, large)) in prescribe(&xyz, &f)?.iter().zip(&prescribe(&big, &f)?) {
        assert!((*small * 1000.0 - *large).length() <= 1e-9 * 1000.0);
    }
    Ok(())
}

#[test]
fn a_chart_with_no_interior_vertex_lays_its_triangles_flat() -> Result<(), BoundaryError> {
    let xyz = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 0.0),
        DVec3::new(3.0, 0.0, 4.0),
    ];
    let flat = prescribe(&xyz, &faces(&[[0, 1, 2]]))?;
    let flat_corners = [
        DVec2::new(0.0, 0.0),
        DVec2::new(3.0, 0.0),
        DVec2::new(3.0, 4.0),
    ];
    assert!(congruent(&flat, |v| flat_corners[v.index() as usize]));
    Ok(())
}

#[test]
fn the_same_chart_gives_the_same_bits() -> Result<(), BoundaryError> {
    let (xyz, f) = cap();
    let bits = |flat: Vec<(VertexId, DVec2)>| -> Vec<(u32, u64, u64)> {
        flat.iter()
            .map(|(v, p)| (v.index(), p.x.to_bits(), p.y.to_bits()))
            .collect()
    };
    assert_eq!(bits(prescribe(&xyz, &f)?), bits(prescribe(&xyz, &f)?));
    Ok(())
}

#[test]
fn faces_that_cannot_be_measured_are_refused() {
    let xyz = [
        DVec3::new(0.0, 0.0, 0.0),
        DVec3::new(1.0, 0.0, 0.0),
        DVec3::new(2.0, 0.0, 0.0),
        DVec3::new(f64::NAN, 0.0, 1.0),
        DVec3::new(0.0, 1.0, 0.0),
    ];
    let face = |f, t: [u32; 3]| vec![(FaceId::new(f), t.map(VertexId::new))];
    assert_eq!(prescribe(&xyz, &[]), Err(BoundaryError::Empty));
    assert_eq!(
        prescribe(&xyz, &face(7, [0, 1, 5])),
        Err(BoundaryError::BadVertex {
            face: FaceId::new(7)
        })
    );
    for (f, t) in [(3, [0, 1, 1]), (4, [0, 1, 2]), (5, [0, 1, 3])] {
        assert_eq!(
            prescribe(&xyz, &face(f, t)),
            Err(BoundaryError::Degenerate {
                face: FaceId::new(f)
            })
        );
    }
    // Valid triangles whose cross product or dot product overflows have no measurable angles.
    for far in [
        [
            DVec3::ZERO,
            DVec3::new(1e100, 0.0, 0.0),
            DVec3::new(0.0, 1e100, 0.0),
        ],
        [
            DVec3::ZERO,
            DVec3::new(1e160, 0.0, 0.0),
            DVec3::new(1e160, 1e-10, 0.0),
        ],
    ] {
        assert_eq!(
            prescribe(&far, &face(0, [0, 1, 2])),
            Err(BoundaryError::Degenerate {
                face: FaceId::new(0)
            })
        );
    }
}

#[test]
fn a_chart_that_is_not_a_disk_is_refused() {
    let (xyz, disk) = surface(3, 3, |c, r| DVec3::new(c, r, 0.0));
    let annulus: Faces = disk
        .into_iter()
        .filter(|(f, _)| f.index() / 2 != 4)
        .collect();
    assert_eq!(prescribe(&xyz, &annulus), Err(BoundaryError::NotADisk));
}
