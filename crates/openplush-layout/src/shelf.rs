use openplush_geom::DVec2;
use openplush_pattern::{PatternPiece, PieceId};

use crate::{LayoutError, LayoutOptions, Placement, Sheet};

/// Lays every copy of every piece on shelves across the roll, tallest first.
///
/// Each piece uses its first allowed rotation. Pieces tie on height by `PieceId`, then keep input order.
pub fn pack(pieces: &[PatternPiece], opts: &LayoutOptions) -> Result<Vec<Sheet>, LayoutError> {
    if pieces.is_empty() {
        return Err(LayoutError::NoPieces);
    }
    let roll = opts.roll_width.get();
    let gap = opts.gap.get();
    let mut items = pieces
        .iter()
        .map(|p| Item::new(p, opts.mirrored))
        .collect::<Result<Vec<_>, _>>()?;
    items.sort_by(|a, b| b.height.total_cmp(&a.height).then(a.piece.cmp(&b.piece)));

    let mut placements = Vec::new();
    let (mut x, mut y, mut shelf_height) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut shelf_empty = true;
    for item in &items {
        if item.width > roll {
            return Err(LayoutError::PieceTooWide {
                piece: item.piece,
                width_mm: item.width,
                roll_mm: roll,
            });
        }
        for copy in 0..item.cuts {
            if !shelf_empty && x + gap + item.width > roll {
                y += shelf_height + gap;
                (x, shelf_height, shelf_empty) = (0.0, 0.0, true);
            }
            let left = if shelf_empty { 0.0 } else { x + gap };
            placements.push(item.placement(copy, DVec2::new(left, y)));
            x = left + item.width;
            shelf_height = shelf_height.max(item.height);
            shelf_empty = false;
        }
    }
    Ok(vec![Sheet {
        width_mm: roll,
        length_mm: y + shelf_height,
        placements,
    }])
}

/// A piece as it will sit: its transform and the size and corner of its transformed bounding box.
struct Item {
    piece: PieceId,
    cuts: u32,
    rotation_deg: f64,
    mirrored: bool,
    min: DVec2,
    width: f64,
    height: f64,
}

impl Item {
    fn new(piece: &PatternPiece, mirrored: bool) -> Result<Self, LayoutError> {
        // INVARIANT: `PatternPiece::new` refuses an empty rotation set.
        let rotation_deg = piece.rotations_deg()[0];
        let turns = quarter_turns(rotation_deg).ok_or(LayoutError::UnsupportedRotation {
            piece: piece.id(),
            rotation_deg,
        })?;
        let (mut lo, mut hi) = (
            DVec2::new(f64::MAX, f64::MAX),
            DVec2::new(f64::MIN, f64::MIN),
        );
        for q in piece.cut_line().points() {
            let t = transform(*q, turns, mirrored);
            (lo, hi) = (
                DVec2::new(lo.x.min(t.x), lo.y.min(t.y)),
                DVec2::new(hi.x.max(t.x), hi.y.max(t.y)),
            );
        }
        Ok(Self {
            piece: piece.id(),
            cuts: piece.cut_quantity(),
            rotation_deg,
            mirrored,
            min: lo,
            width: hi.x - lo.x,
            height: hi.y - lo.y,
        })
    }

    /// The placement that puts the transformed bounding box's lower-left corner at `corner`.
    fn placement(&self, copy: u32, corner: DVec2) -> Placement {
        Placement {
            piece: self.piece,
            copy,
            rotation_deg: self.rotation_deg,
            offset: DVec2::new(corner.x - self.min.x, corner.y - self.min.y),
            mirrored: self.mirrored,
        }
    }
}

/// Quarter turns (0 to 3) for a rotation that is an exact multiple of 90 degrees.
fn quarter_turns(deg: f64) -> Option<u8> {
    let d = deg.rem_euclid(360.0);
    [0.0, 90.0, 180.0, 270.0]
        .iter()
        .position(|&q| q == d)
        .and_then(|i| u8::try_from(i).ok())
}

/// Mirrors about the y-axis if asked, then turns counter-clockwise: exact, with no trigonometry.
fn transform(p: DVec2, turns: u8, mirrored: bool) -> DVec2 {
    let (x, y) = if mirrored { (-p.x, p.y) } else { (p.x, p.y) };
    match turns {
        1 => DVec2::new(-y, x),
        2 => DVec2::new(-x, -y),
        3 => DVec2::new(y, -x),
        _ => DVec2::new(x, y),
    }
}
