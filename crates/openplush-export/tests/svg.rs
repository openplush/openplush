//! `svg::write`: one sheet as SVG at true scale, 1 user unit = 1 mm, printed the right way up.

use openplush_export::ExportError;
use openplush_export::svg::write;
use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Sheet};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

type Setup = Box<dyn std::error::Error>;

/// An L shape, so a print that is mirrored or upside down cannot match it.
fn ell(scale: f64) -> Result<Outline, Setup> {
    let pts = [
        (0.0, 0.0),
        (3.0, 0.0),
        (3.0, 1.0),
        (1.0, 1.0),
        (1.0, 2.0),
        (0.0, 2.0),
    ];
    Ok(Outline::new(
        pts.iter()
            .map(|&(x, y)| DVec2::new(x * scale, y * scale))
            .collect(),
    )?)
}

/// A piece whose sewing line is an L at 10 mm per unit, inside a cut line at 20 mm per unit.
fn piece(id: u32, cuts: u32, rotation: f64) -> Result<PatternPiece, Setup> {
    let sewing = Outline::new(
        ell(10.0)?
            .points()
            .iter()
            .map(|p| *p + DVec2::new(5.0, 5.0))
            .collect(),
    )?;
    Ok(PatternPiece::new(
        PieceId::new(id),
        sewing,
        ell(20.0)?,
        CutCount::new(cuts, 0)?,
        vec![rotation],
    )?)
}

fn sheet(pieces: &[PatternPiece], mirrored: bool) -> Result<Sheet, Setup> {
    let opts = LayoutOptions::new(Mm::try_new(200.0)?, Mm::try_new(5.0)?, mirrored)?;
    pack(pieces, &opts)?
        .into_iter()
        .next()
        .ok_or_else(|| LayoutError::NoPieces.into())
}

fn text(bytes: Vec<u8>) -> Result<String, Setup> {
    Ok(String::from_utf8(bytes)?)
}

/// Every path's points, in document order, read back from the `d` attributes.
fn paths(svg: &str) -> Result<Vec<Vec<DVec2>>, Setup> {
    let mut out = Vec::new();
    for chunk in svg.split(" d=\"").skip(1) {
        let d = chunk.split('"').next().ok_or("unterminated d")?;
        let nums: Vec<f64> = d
            .split(' ')
            .filter(|t| !matches!(*t, "M" | "L" | "Z"))
            .map(str::parse)
            .collect::<Result<_, _>>()?;
        out.push(nums.chunks(2).map(|c| DVec2::new(c[0], c[1])).collect());
    }
    Ok(out)
}

#[test]
fn the_document_is_the_sheet_in_millimetres() -> Result<(), Setup> {
    let pieces = [piece(0, 1, 0.0)?];
    let s = sheet(&pieces, false)?;
    let svg = text(write(&s, &pieces, &[])?)?;
    assert!(svg.starts_with(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\""
    ));
    assert!(
        svg.contains(" width=\"200.000mm\" height=\"40.000mm\" viewBox=\"0 0 200.000 40.000\"")
    );
    assert!(svg.ends_with("</svg>\n"));
    Ok(())
}

#[test]
fn each_copy_draws_a_solid_cut_line_then_a_dashed_sewing_line() -> Result<(), Setup> {
    let pieces = [piece(0, 3, 0.0)?];
    let s = sheet(&pieces, false)?;
    let svg = text(write(&s, &pieces, &[])?)?;
    assert_eq!(svg.matches("<path class=\"cut\"").count(), 3);
    assert_eq!(svg.matches("<path class=\"sew\"").count(), 3);
    let cut = svg.find("<path class=\"cut\"").ok_or("no cut")?;
    let sew = svg.find("<path class=\"sew\"").ok_or("no sew")?;
    assert!(cut < sew);
    assert!(svg.contains(".cut{fill:none;stroke:#000;stroke-width:0.5}"));
    assert!(svg.contains(".sew{fill:none;stroke:#000;stroke-width:0.3;stroke-dasharray:3 2}"));
    Ok(())
}

#[test]
fn a_cut_line_is_written_exactly() -> Result<(), Setup> {
    // The sheet is 40 mm long and SVG's y runs down, so sheet y becomes 40 - y.
    let pieces = [piece(0, 1, 0.0)?];
    let s = sheet(&pieces, false)?;
    let svg = text(write(&s, &pieces, &[])?)?;
    assert!(svg.contains(
        " d=\"M 0.000 40.000 L 60.000 40.000 L 60.000 20.000 L 20.000 20.000 L 20.000 0.000 L 0.000 0.000 Z\""
    ));
    Ok(())
}

#[test]
fn every_path_is_the_placed_outline_flipped_to_print_the_right_way_up() -> Result<(), Setup> {
    for (rotation, mirrored) in [(0.0, false), (90.0, false), (180.0, true), (270.0, true)] {
        let pieces = [piece(0, 2, rotation)?, piece(1, 1, rotation)?];
        let s = sheet(&pieces, mirrored)?;
        let drawn = paths(&text(write(&s, &pieces, &[])?)?)?;
        let mut expected = Vec::new();
        for pl in s.placements() {
            let p = pieces
                .iter()
                .find(|p| p.id() == pl.piece())
                .ok_or("piece")?;
            for outline in [p.cut_line(), p.sewing_line()] {
                let flipped = outline.points().iter().map(|&q| {
                    let on_sheet = pl.apply(q);
                    DVec2::new(on_sheet.x, s.length_mm() - on_sheet.y)
                });
                expected.push(flipped.collect::<Vec<_>>());
            }
        }
        assert_eq!(drawn.len(), expected.len());
        for (d, e) in drawn.iter().zip(&expected) {
            assert_eq!(d.len(), e.len());
            for (a, b) in d.iter().zip(e) {
                let close = (a.x - b.x).abs() <= 0.0005 && (a.y - b.y).abs() <= 0.0005;
                assert!(close, "{rotation} {mirrored}: {a:?} vs {b:?}");
            }
        }
    }
    Ok(())
}

#[test]
fn the_same_input_writes_the_same_bytes() -> Result<(), Setup> {
    let pieces = [piece(0, 2, 90.0)?, piece(1, 1, 0.0)?];
    let s = sheet(&pieces, true)?;
    assert_eq!(write(&s, &pieces, &[])?, write(&s, &pieces, &[])?);
    Ok(())
}

#[test]
fn a_placement_without_its_piece_is_refused() -> Result<(), Setup> {
    let placed = [piece(0, 1, 0.0)?, piece(1, 1, 0.0)?];
    let s = sheet(&placed, false)?;
    assert_eq!(
        write(&s, &placed[..1], &[]),
        Err(ExportError::UnknownPiece {
            piece: PieceId::new(1)
        })
    );
    Ok(())
}

#[test]
fn two_pieces_with_one_id_are_refused() -> Result<(), Setup> {
    let pieces = [piece(0, 1, 0.0)?];
    let s = sheet(&pieces, false)?;
    let twice = [piece(0, 1, 0.0)?, piece(0, 1, 0.0)?];
    assert_eq!(
        write(&s, &twice, &[]),
        Err(ExportError::DuplicatePiece {
            piece: PieceId::new(0)
        })
    );
    Ok(())
}

#[test]
fn the_errors_say_what_is_wrong() {
    assert_eq!(
        ExportError::UnknownPiece {
            piece: PieceId::new(4)
        }
        .to_string(),
        "the sheet places piece 4, which was not given"
    );
    assert_eq!(
        ExportError::DuplicatePiece {
            piece: PieceId::new(4)
        }
        .to_string(),
        "piece 4 was given more than once"
    );
}
