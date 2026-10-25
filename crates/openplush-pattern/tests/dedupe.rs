//! `dedupe::fit`: how one sewing line lies on another, as printed or flipped, after the best
//! turn and shift, with corners matched in order.

use openplush_geom::DVec2;
use openplush_pattern::Outline;
use openplush_pattern::dedupe::{Facing, fit, fits};

type Setup = Box<dyn std::error::Error>;

/// An L, so a flipped copy cannot lie on it as printed.
fn ell() -> Vec<DVec2> {
    [
        (0.0, 0.0),
        (90.0, 0.0),
        (90.0, 30.0),
        (30.0, 30.0),
        (30.0, 60.0),
        (0.0, 60.0),
    ]
    .iter()
    .map(|&(x, y)| DVec2::new(x, y))
    .collect()
}

/// `points` turned by `deg` about the origin, moved by `by`, and started at corner `start`.
fn moved(points: &[DVec2], deg: f64, by: DVec2, start: usize) -> Vec<DVec2> {
    let (s, c) = (libm::sin(deg.to_radians()), libm::cos(deg.to_radians()));
    let n = points.len();
    (0..n)
        .map(|i| {
            let p = points[(i + start) % n];
            DVec2::new(c * p.x - s * p.y, s * p.x + c * p.y) + by
        })
        .collect()
}

/// `points` mirrored left to right, reversed so it still runs counter-clockwise.
fn mirrored(points: &[DVec2]) -> Vec<DVec2> {
    points.iter().rev().map(|p| DVec2::new(-p.x, p.y)).collect()
}

#[test]
fn a_turned_moved_copy_lies_on_the_original_as_printed() -> Result<(), Setup> {
    let a = Outline::new(ell())?;
    let b = Outline::new(moved(&ell(), 37.0, DVec2::new(200.0, -40.0), 2))?;
    let f = fit(&a, &b, Facing::AsPrinted).ok_or("same corner count")?;
    assert!(f.gap_mm() <= 1e-9, "gap {}", f.gap_mm());
    // `b` starts at the L's corner 2, so the L's corner 2 is `b`'s corner 0.
    assert_eq!(f.corner(2), 0);
    assert_eq!(f.corner(0), 4);
    for i in 0..6 {
        assert_eq!(
            f.edge(i),
            f.corner(i),
            "as printed, edge i starts at corner i's match"
        );
    }
    Ok(())
}

#[test]
fn a_mirror_image_lies_on_the_original_only_flipped() -> Result<(), Setup> {
    let a = Outline::new(ell())?;
    let b = Outline::new(moved(&mirrored(&ell()), 90.0, DVec2::new(10.0, 10.0), 1))?;
    let flipped = fit(&a, &b, Facing::Flipped).ok_or("same corner count")?;
    assert!(flipped.gap_mm() <= 1e-9, "gap {}", flipped.gap_mm());
    let printed = fit(&a, &b, Facing::AsPrinted).ok_or("same corner count")?;
    assert!(
        printed.gap_mm() > 10.0,
        "an L lies on its mirror image: {}",
        printed.gap_mm()
    );
    Ok(())
}

#[test]
fn flipped_corners_and_edges_run_backwards() -> Result<(), Setup> {
    let a = Outline::new(ell())?;
    let b = Outline::new(mirrored(&ell()))?;
    let f = fit(&a, &b, Facing::Flipped).ok_or("same corner count")?;
    let (pa, pb) = (a.points(), b.points());
    let n = pa.len();
    for i in 0..n {
        // The mirror keeps y, so matched corners share it.
        assert!((pa[i].y - pb[f.corner(i)].y).abs() <= 1e-9, "corner {i}");
        // Edge i of `a` joins corners i and i + 1; its match joins their matches.
        let e = f.edge(i);
        let ends = [f.corner(i), f.corner((i + 1) % n)];
        assert!(
            ends.contains(&e) && ends.contains(&((e + 1) % n)),
            "edge {i} -> {e}"
        );
    }
    Ok(())
}

#[test]
fn a_corner_moved_3_mm_leaves_a_gap_of_at_most_3_mm() -> Result<(), Setup> {
    let a = Outline::new(ell())?;
    let mut near = mirrored(&ell());
    near[2].x -= 3.0;
    let b = Outline::new(near)?;
    let f = fit(&a, &b, Facing::Flipped).ok_or("same corner count")?;
    assert!(f.gap_mm() > 1.0 && f.gap_mm() <= 3.0, "gap {}", f.gap_mm());
    Ok(())
}

#[test]
fn outlines_with_different_corner_counts_do_not_fit() -> Result<(), Setup> {
    let a = Outline::new(ell())?;
    let square = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(1.0, 0.0),
        DVec2::new(1.0, 1.0),
        DVec2::new(0.0, 1.0),
    ])?;
    assert!(fit(&a, &square, Facing::AsPrinted).is_none());
    assert!(fit(&a, &square, Facing::Flipped).is_none());
    Ok(())
}

fn square() -> Result<Outline, Setup> {
    Ok(Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(10.0, 0.0),
        DVec2::new(10.0, 10.0),
        DVec2::new(0.0, 10.0),
    ])?)
}

#[test]
fn of_equally_good_pairings_the_lowest_offset_is_the_fit() -> Result<(), Setup> {
    let s = square()?;
    let f = fit(&s, &s, Facing::AsPrinted).ok_or("same corner count")?;
    assert_eq!(f.corner(0), 0);
    Ok(())
}

#[test]
fn a_symmetric_shape_lies_on_itself_every_way_it_can() -> Result<(), Setup> {
    let s = square()?;
    let all = fits(&s, &s, Facing::AsPrinted, 1e-9);
    let starts: Vec<usize> = all.iter().map(|f| f.corner(0)).collect();
    assert_eq!(starts, [0, 1, 2, 3]);
    let l = Outline::new(ell())?;
    assert_eq!(fits(&l, &l, Facing::AsPrinted, 1e-9).len(), 1);
    assert!(fits(&l, &s, Facing::AsPrinted, 1e9).is_empty());
    Ok(())
}
