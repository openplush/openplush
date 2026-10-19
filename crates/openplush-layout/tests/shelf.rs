//! `shelf::pack`: simple shelf packing across a roll, with rotation sets and mirroring honoured.

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Placement, Sheet};
use openplush_pattern::{CutCount, Outline, PatternError, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

/// A rectangle with its lower-left corner at (x0, y0).
fn rect(x0: f64, y0: f64, w: f64, h: f64) -> Result<Outline, PatternError> {
    Outline::new(vec![
        DVec2::new(x0, y0),
        DVec2::new(x0 + w, y0),
        DVec2::new(x0 + w, y0 + h),
        DVec2::new(x0, y0 + h),
    ])
}

fn piece(
    id: u32,
    o: (f64, f64, f64, f64),
    cuts: u32,
    rot: &[f64],
) -> Result<PatternPiece, PatternError> {
    let cut = rect(o.0, o.1, o.2, o.3)?;
    PatternPiece::new(
        PieceId::new(id),
        cut.clone(),
        cut,
        CutCount::new(cuts, 0)?,
        rot.to_vec(),
    )
}

fn opts(roll: f64, gap: f64, mirrored: bool) -> Result<LayoutOptions, Setup> {
    Ok(LayoutOptions::new(
        Mm::try_new(roll)?,
        Mm::try_new(gap)?,
        mirrored,
    )?)
}

fn only(sheets: Vec<Sheet>) -> Result<Sheet, Setup> {
    let mut it = sheets.into_iter();
    match (it.next(), it.next()) {
        (Some(s), None) => Ok(s),
        _ => Err(LayoutError::NoPieces.into()),
    }
}

/// Where a placement puts the corners of `outline`, recomputed here from the documented transform.
fn placed(outline: &Outline, p: &Placement) -> Vec<DVec2> {
    outline
        .points()
        .iter()
        .map(|q| {
            let (x, y) = if p.mirrored() {
                (-q.x, q.y)
            } else {
                (q.x, q.y)
            };
            let (x, y) = match (p.rotation_deg() as i64).rem_euclid(360) {
                90 => (-y, x),
                180 => (-x, -y),
                270 => (y, -x),
                _ => (x, y),
            };
            DVec2::new(x + p.offset().x, y + p.offset().y)
        })
        .collect()
}

fn bounds(pts: &[DVec2]) -> (f64, f64, f64, f64) {
    pts.iter()
        .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |b, p| {
            (b.0.min(p.x), b.1.min(p.y), b.2.max(p.x), b.3.max(p.y))
        })
}

#[test]
fn one_piece_sits_in_the_corner_and_the_sheet_is_as_long_as_it_is() -> Result<(), Setup> {
    let s = only(pack(
        &[piece(1, (0.0, 0.0, 10.0, 20.0), 1, &[0.0])?],
        &opts(100.0, 5.0, false)?,
    )?)?;
    assert_eq!((s.width_mm(), s.length_mm()), (100.0, 20.0));
    let [p] = s.placements() else {
        return Err(LayoutError::NoPieces.into());
    };
    assert_eq!(
        (p.piece(), p.copy(), p.rotation_deg(), p.mirrored()),
        (PieceId::new(1), 0, 0.0, false)
    );
    assert_eq!(p.offset(), DVec2::new(0.0, 0.0));
    Ok(())
}

#[test]
fn a_piece_not_at_the_origin_is_moved_so_its_corner_meets_the_shelf() -> Result<(), Setup> {
    let pc = piece(1, (30.0, 40.0, 10.0, 10.0), 1, &[0.0])?;
    let s = only(pack(std::slice::from_ref(&pc), &opts(100.0, 0.0, false)?)?)?;
    let b = bounds(&placed(pc.cut_line(), &s.placements()[0]));
    assert_eq!(b, (0.0, 0.0, 10.0, 10.0));
    Ok(())
}

#[test]
fn every_copy_is_placed_in_copy_order_with_the_gap_between() -> Result<(), Setup> {
    let pc = piece(1, (0.0, 0.0, 10.0, 10.0), 3, &[0.0])?;
    let s = only(pack(&[pc], &opts(100.0, 5.0, false)?)?)?;
    let xs: Vec<_> = s
        .placements()
        .iter()
        .map(|p| (p.copy(), p.offset().x))
        .collect();
    assert_eq!(xs, [(0, 0.0), (1, 15.0), (2, 30.0)]);
    Ok(())
}

#[test]
fn a_full_shelf_wraps_to_a_new_one_above_the_tallest_piece_plus_the_gap() -> Result<(), Setup> {
    let pc = piece(1, (0.0, 0.0, 10.0, 10.0), 3, &[0.0])?;
    let s = only(pack(&[pc], &opts(25.0, 5.0, false)?)?)?;
    let at: Vec<_> = s
        .placements()
        .iter()
        .map(|p| (p.offset().x, p.offset().y))
        .collect();
    assert_eq!(at, [(0.0, 0.0), (15.0, 0.0), (0.0, 15.0)]);
    assert_eq!(s.length_mm(), 25.0);
    Ok(())
}

#[test]
fn a_piece_that_fits_exactly_stays_and_one_a_hair_over_wraps() -> Result<(), Setup> {
    let pc = || piece(1, (0.0, 0.0, 10.0, 10.0), 2, &[0.0]);
    let fits = only(pack(&[pc()?], &opts(25.0, 5.0, false)?)?)?;
    assert_eq!(fits.length_mm(), 10.0);
    let wraps = only(pack(&[pc()?], &opts(24.99, 5.0, false)?)?)?;
    assert_eq!(wraps.length_mm(), 25.0);
    Ok(())
}

#[test]
fn taller_pieces_go_first_whatever_the_input_order() -> Result<(), Setup> {
    let (a, b) = (
        piece(1, (0.0, 0.0, 10.0, 10.0), 1, &[0.0])?,
        piece(2, (0.0, 0.0, 10.0, 30.0), 1, &[0.0])?,
    );
    let o = opts(100.0, 0.0, false)?;
    let fwd = pack(&[a.clone(), b.clone()], &o)?;
    let rev = pack(&[b, a], &o)?;
    assert_eq!(fwd, rev);
    let first = fwd[0].placements()[0].piece();
    assert_eq!(first, PieceId::new(2));
    Ok(())
}

#[test]
fn equal_heights_fall_back_to_the_piece_id() -> Result<(), Setup> {
    let (a, b) = (
        piece(5, (0.0, 0.0, 10.0, 10.0), 1, &[0.0])?,
        piece(3, (0.0, 0.0, 10.0, 10.0), 1, &[0.0])?,
    );
    let s = only(pack(&[a, b], &opts(100.0, 0.0, false)?)?)?;
    let ids: Vec<_> = s.placements().iter().map(|p| p.piece()).collect();
    assert_eq!(ids, [PieceId::new(3), PieceId::new(5)]);
    Ok(())
}

#[test]
fn the_first_allowed_rotation_is_used_and_recorded() -> Result<(), Setup> {
    for deg in [90.0, 180.0, 270.0] {
        let pc = piece(1, (2.0, 3.0, 20.0, 5.0), 1, &[deg, 0.0])?;
        let s = only(pack(std::slice::from_ref(&pc), &opts(100.0, 0.0, false)?)?)?;
        let p = &s.placements()[0];
        assert_eq!(p.rotation_deg(), deg);
        let (x0, y0, x1, y1) = bounds(&placed(pc.cut_line(), p));
        assert_eq!((x0, y0), (0.0, 0.0), "{deg}");
        let swapped = deg == 90.0 || deg == 270.0;
        assert_eq!(
            (x1, y1),
            if swapped { (5.0, 20.0) } else { (20.0, 5.0) },
            "{deg}"
        );
    }
    Ok(())
}

#[test]
fn a_quarter_turn_lets_a_long_piece_fit_a_narrow_roll() -> Result<(), Setup> {
    let pc = piece(1, (0.0, 0.0, 20.0, 5.0), 1, &[90.0])?;
    assert!(pack(&[pc], &opts(10.0, 0.0, false)?).is_ok());
    Ok(())
}

#[test]
fn mirroring_is_recorded_and_the_piece_still_starts_at_the_corner() -> Result<(), Setup> {
    let pc = piece(1, (4.0, 4.0, 10.0, 6.0), 2, &[0.0])?;
    let s = only(pack(std::slice::from_ref(&pc), &opts(100.0, 0.0, true)?)?)?;
    assert!(s.placements().iter().all(Placement::mirrored));
    let b = bounds(&placed(pc.cut_line(), &s.placements()[0]));
    assert_eq!(b, (0.0, 0.0, 10.0, 6.0));
    Ok(())
}

#[test]
fn a_piece_wider_than_the_roll_is_refused_with_its_width() -> Result<(), Setup> {
    let pc = piece(4, (0.0, 0.0, 30.0, 5.0), 1, &[0.0])?;
    assert_eq!(
        pack(&[pc], &opts(20.0, 0.0, false)?),
        Err(LayoutError::PieceTooWide {
            piece: PieceId::new(4),
            width_mm: 30.0,
            roll_mm: 20.0
        })
    );
    Ok(())
}

#[test]
fn a_rotation_that_is_not_a_quarter_turn_is_refused_not_approximated() -> Result<(), Setup> {
    let pc = piece(2, (0.0, 0.0, 10.0, 10.0), 1, &[45.0, 225.0])?;
    assert_eq!(
        pack(&[pc], &opts(100.0, 0.0, false)?),
        Err(LayoutError::UnsupportedRotation {
            piece: PieceId::new(2),
            rotation_deg: 45.0
        })
    );
    Ok(())
}

#[test]
fn no_pieces_and_a_zero_roll_are_refused() -> Result<(), Setup> {
    assert_eq!(
        pack(&[], &opts(100.0, 0.0, false)?),
        Err(LayoutError::NoPieces)
    );
    assert_eq!(
        LayoutOptions::new(Mm::try_new(0.0)?, Mm::try_new(0.0)?, false),
        Err(LayoutError::BadRollWidth)
    );
    Ok(())
}

#[test]
fn a_varied_set_never_overlaps_and_stays_inside_the_roll() -> Result<(), Setup> {
    let mut seed = 12345_u64;
    let mut next = |lo: f64, hi: f64| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        lo + (hi - lo) * ((seed >> 33) as f64 / (1_u64 << 31) as f64)
    };
    let mut pieces = Vec::new();
    for id in 0..12 {
        let (w, h) = (next(10.0, 90.0), next(10.0, 90.0));
        pieces.push(piece(
            id,
            (next(-5.0, 5.0), next(-5.0, 5.0), w, h),
            1 + id % 3,
            &[[0.0, 90.0, 180.0][id as usize % 3]],
        )?);
    }
    let roll = 200.0;
    let s = only(pack(&pieces, &opts(roll, 2.0, false)?)?)?;
    let boxes: Vec<_> = s
        .placements()
        .iter()
        .map(|p| {
            let src = pieces.iter().find(|q| q.id() == p.piece());
            src.map(|q| bounds(&placed(q.cut_line(), p)))
        })
        .collect::<Option<_>>()
        .ok_or(LayoutError::NoPieces)?;
    assert_eq!(
        boxes.len(),
        pieces
            .iter()
            .map(|p| p.cuts().total() as usize)
            .sum::<usize>()
    );
    for (i, a) in boxes.iter().enumerate() {
        assert!(
            a.0 >= 0.0 && a.2 <= roll && a.1 >= 0.0 && a.3 <= s.length_mm(),
            "{a:?}"
        );
        for b in &boxes[i + 1..] {
            let apart = a.2 <= b.0 || b.2 <= a.0 || a.3 <= b.1 || b.3 <= a.1;
            assert!(apart, "{a:?} overlaps {b:?}");
        }
    }
    Ok(())
}

#[test]
fn the_same_input_gives_the_same_sheet() -> Result<(), Setup> {
    let ps = [
        piece(1, (0.0, 0.0, 10.0, 10.0), 4, &[0.0])?,
        piece(2, (0.0, 0.0, 7.0, 13.0), 2, &[180.0])?,
    ];
    let o = opts(40.0, 3.0, true)?;
    assert_eq!(pack(&ps, &o)?, pack(&ps, &o)?);
    Ok(())
}

#[test]
fn the_errors_say_what_is_wrong() {
    let cases = [
        (
            LayoutError::BadRollWidth,
            "the roll width must be greater than zero",
        ),
        (LayoutError::NoPieces, "there are no pieces to lay out"),
        (
            LayoutError::PieceTooWide {
                piece: PieceId::new(4),
                width_mm: 30.0,
                roll_mm: 20.0,
            },
            "piece 4 is 30.000 mm wide but the roll is 20.000 mm",
        ),
        (
            LayoutError::UnsupportedRotation {
                piece: PieceId::new(2),
                rotation_deg: 45.0,
            },
            "piece 2 allows a rotation of 45 degrees; only multiples of 90 are supported",
        ),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
    }
}

#[test]
fn a_piece_exactly_as_wide_as_the_roll_fits_on_an_empty_shelf_without_the_gap() -> Result<(), Setup>
{
    let pc = piece(1, (0.0, 0.0, 10.0, 10.0), 1, &[0.0])?;
    let s = only(pack(&[pc], &opts(10.0, 5.0, false)?)?)?;
    assert_eq!(s.placements()[0].offset(), DVec2::new(0.0, 0.0));
    assert_eq!(s.length_mm(), 10.0);
    Ok(())
}

#[test]
fn a_new_shelf_starts_with_the_height_of_its_own_pieces() -> Result<(), Setup> {
    let (a, b) = (
        piece(1, (0.0, 0.0, 10.0, 20.0), 1, &[0.0])?,
        piece(2, (0.0, 0.0, 10.0, 10.0), 1, &[0.0])?,
    );
    let s = only(pack(&[a, b], &opts(10.0, 0.0, false)?)?)?;
    assert_eq!(s.placements()[1].offset(), DVec2::new(0.0, 20.0));
    assert_eq!(s.length_mm(), 30.0);
    Ok(())
}

#[test]
fn a_rotation_outside_zero_to_359_is_the_same_turn_it_names() -> Result<(), Setup> {
    for (deg, turn) in [(-90.0, 270.0), (360.0, 0.0), (450.0, 90.0)] {
        let given = piece(1, (2.0, 3.0, 20.0, 5.0), 1, &[deg])?;
        let named = piece(1, (2.0, 3.0, 20.0, 5.0), 1, &[turn])?;
        let o = opts(100.0, 0.0, false)?;
        let (g, n) = (only(pack(&[given], &o)?)?, only(pack(&[named], &o)?)?);
        assert_eq!(g.placements()[0].rotation_deg(), deg);
        assert_eq!(
            g.placements()[0].offset(),
            n.placements()[0].offset(),
            "{deg}"
        );
        assert_eq!(g.length_mm(), n.length_mm(), "{deg}");
    }
    Ok(())
}

#[test]
fn a_mirrored_piece_lies_at_negative_x_and_still_packs_by_its_real_width() -> Result<(), Setup> {
    let pc = piece(1, (4.0, 4.0, 10.0, 6.0), 2, &[0.0])?;
    let s = only(pack(std::slice::from_ref(&pc), &opts(100.0, 0.0, true)?)?)?;
    let second = bounds(&placed(pc.cut_line(), &s.placements()[1]));
    assert_eq!(second, (10.0, 0.0, 20.0, 6.0));
    Ok(())
}

#[test]
fn the_cut_line_decides_the_size_not_the_sewing_line() -> Result<(), Setup> {
    let pc = PatternPiece::new(
        PieceId::new(1),
        rect(0.0, 0.0, 10.0, 10.0)?,
        rect(0.0, 0.0, 14.0, 14.0)?,
        CutCount::new(2, 0)?,
        vec![0.0],
    )?;
    let s = only(pack(&[pc], &opts(100.0, 0.0, false)?)?)?;
    assert_eq!(s.placements()[1].offset(), DVec2::new(14.0, 0.0));
    assert_eq!(s.length_mm(), 14.0);
    Ok(())
}
