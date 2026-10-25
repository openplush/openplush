//! Piece labels: the block's lines, a seam's text, the label's checks, and the point inside an
//! outline the block is centred on.

use openplush_geom::DVec2;
use openplush_pattern::label::{block, seam_text};
use openplush_pattern::{CutCount, Label, Outline, PatternError, PieceId, SeamLabel};
use openplush_seams::SeamChainId;

type Setup = Box<dyn std::error::Error>;

#[test]
fn the_block_names_the_piece_its_cuts_its_fabric_and_its_partners() -> Result<(), Setup> {
    let lines = block(
        PieceId::new(3),
        CutCount::new(1, 1)?,
        "cotton",
        &[PieceId::new(1), PieceId::new(4)],
    );
    assert_eq!(
        lines,
        [
            "Piece 3",
            "Cut 2 (1 + 1 flipped)",
            "cotton",
            "Sews to pieces 1, 4"
        ]
    );
    Ok(())
}

#[test]
fn the_cut_line_reads_naturally_for_each_mix_of_counts() -> Result<(), Setup> {
    let cut = |a, f| -> Result<String, Setup> {
        Ok(block(PieceId::new(0), CutCount::new(a, f)?, "felt", &[])[1].clone())
    };
    assert_eq!(cut(1, 0)?, "Cut 1");
    assert_eq!(cut(4, 0)?, "Cut 4");
    assert_eq!(cut(0, 1)?, "Cut 1 flipped");
    assert_eq!(cut(0, 2)?, "Cut 2 flipped");
    assert_eq!(cut(2, 1)?, "Cut 3 (2 + 1 flipped)");
    Ok(())
}

#[test]
fn one_partner_is_singular_and_none_leaves_the_line_out() -> Result<(), Setup> {
    let one = block(
        PieceId::new(0),
        CutCount::new(1, 0)?,
        "felt",
        &[PieceId::new(2)],
    );
    assert_eq!(one.last().map(String::as_str), Some("Sews to piece 2"));
    let none = block(PieceId::new(0), CutCount::new(1, 0)?, "felt", &[]);
    assert_eq!(none, ["Piece 0", "Cut 1", "felt"]);
    Ok(())
}

#[test]
fn a_seam_is_named_as_the_warning_summary_names_it() {
    assert_eq!(seam_text(SeamChainId::new(12)), "Seam 12");
}

#[test]
fn a_label_keeps_what_it_was_given() -> Result<(), Setup> {
    let seam = SeamLabel::new(DVec2::new(1.0, 2.0), "Seam 4".to_owned())?;
    let label = Label::new(
        DVec2::new(5.0, 6.0),
        vec!["Piece 0".to_owned()],
        vec![seam.clone()],
    )?;
    assert_eq!(label.at(), DVec2::new(5.0, 6.0));
    assert_eq!(label.lines(), ["Piece 0"]);
    assert_eq!(label.seams(), [seam]);
    assert_eq!(label.seams()[0].at(), DVec2::new(1.0, 2.0));
    assert_eq!(label.seams()[0].text(), "Seam 4");
    Ok(())
}

#[test]
fn a_label_at_a_point_that_is_not_finite_is_refused() {
    let nan = DVec2::new(f64::NAN, 0.0);
    assert_eq!(
        Label::new(nan, vec!["Piece 0".to_owned()], vec![]),
        Err(PatternError::BadLabelPoint)
    );
    assert_eq!(
        SeamLabel::new(nan, "Seam 1".to_owned()),
        Err(PatternError::BadLabelPoint)
    );
}

fn inside(outline: &Outline, p: DVec2) -> bool {
    let pts = outline.points();
    let n = pts.len();
    (0..n)
        .filter(|&i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x)
        })
        .count()
        % 2
        == 1
}

#[test]
fn a_square_s_label_point_is_its_centre() -> Result<(), Setup> {
    let square = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(64.0, 0.0),
        DVec2::new(64.0, 64.0),
        DVec2::new(0.0, 64.0),
    ])?;
    let p = square.label_point();
    assert!((p - DVec2::new(32.0, 32.0)).length() <= 2.0 + 1e-9, "{p:?}");
    Ok(())
}

#[test]
fn a_concave_piece_s_label_point_is_inside_it_and_clear_of_its_edges() -> Result<(), Setup> {
    // A C shape whose bounding-box centre, (50, 50), is in its mouth, outside the piece.
    let c = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(100.0, 0.0),
        DVec2::new(100.0, 20.0),
        DVec2::new(30.0, 20.0),
        DVec2::new(30.0, 80.0),
        DVec2::new(100.0, 80.0),
        DVec2::new(100.0, 100.0),
        DVec2::new(0.0, 100.0),
    ])?;
    let p = c.label_point();
    assert!(inside(&c, p), "{p:?}");
    // The spine is 30 wide and the arms 20, so the spine's middle is the clearest place.
    assert!(
        p.x > 10.0 && p.x < 20.0 && p.y > 10.0 && p.y < 90.0,
        "{p:?}"
    );
    Ok(())
}
