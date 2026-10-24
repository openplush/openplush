use openplush_geom::DVec2;
use openplush_pattern::{PatternPiece, PieceId};

use crate::sheet::{quarter_turns, transform};
use crate::{LayoutError, LayoutOptions, Placement, Sheet};

/// The fabric plan: every copy of every piece on shelves across the roll, tallest first.
///
/// Copies cut as printed come first, then the flipped ones, mirrored. The whole plan is mirrored
/// when the print is (`LayoutOptions`'s `mirrored`), so a flipped copy is then the unmirrored one.
/// Each piece uses its first allowed rotation. Pieces tie on height by `PieceId`, then keep input order.
pub fn pack(pieces: &[PatternPiece], opts: &LayoutOptions) -> Result<Vec<Sheet>, LayoutError> {
    let mut items = Vec::new();
    for p in pieces {
        let cuts = p.cuts();
        if cuts.as_printed() > 0 {
            items.push(Item::new(p, opts.mirrored, cuts.as_printed(), 0)?);
        }
        if cuts.flipped() > 0 {
            items.push(Item::new(
                p,
                !opts.mirrored,
                cuts.flipped(),
                cuts.as_printed(),
            )?);
        }
    }
    Ok(vec![place(items, opts)?.0])
}

/// The print: one template per piece, mirrored when the print is, laid out as `pack` lays copies.
///
/// The sheet is as wide as the pieces reach, not the roll, so no page is tiled over empty roll.
pub fn pack_templates(
    pieces: &[PatternPiece],
    opts: &LayoutOptions,
) -> Result<Vec<Sheet>, LayoutError> {
    let items = pieces
        .iter()
        .map(|p| Item::new(p, opts.mirrored, 1, 0))
        .collect::<Result<Vec<_>, _>>()?;
    let (mut sheet, used) = place(items, opts)?;
    sheet.width_mm = used;
    Ok(vec![sheet])
}

/// The sheet across the whole roll, and how far right its pieces reach.
fn place(mut items: Vec<Item>, opts: &LayoutOptions) -> Result<(Sheet, f64), LayoutError> {
    if items.is_empty() {
        return Err(LayoutError::NoPieces);
    }
    let roll = opts.roll_width.get();
    let gap = opts.gap.get();
    items.sort_by(|a, b| b.height.total_cmp(&a.height).then(a.piece.cmp(&b.piece)));

    let mut placements = Vec::new();
    let (mut x, mut y, mut shelf_height) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut shelf_empty = true;
    let mut used = 0.0_f64;
    for item in &items {
        if item.width > roll {
            return Err(LayoutError::PieceTooWide {
                piece: item.piece,
                width_mm: item.width,
                roll_mm: roll,
            });
        }
        for copy in item.first_copy..item.first_copy + item.cuts {
            if !shelf_empty && x + gap + item.width > roll {
                y += shelf_height + gap;
                (x, shelf_height, shelf_empty) = (0.0, 0.0, true);
            }
            let left = if shelf_empty { 0.0 } else { x + gap };
            placements.push(item.placement(copy, DVec2::new(left, y)));
            x = left + item.width;
            used = used.max(x);
            shelf_height = shelf_height.max(item.height);
            shelf_empty = false;
        }
    }
    let sheet = Sheet {
        width_mm: roll,
        length_mm: y + shelf_height,
        placements,
    };
    Ok((sheet, used))
}

/// A piece as it will sit: its transform and the size and corner of its transformed bounding box.
struct Item {
    piece: PieceId,
    cuts: u32,
    first_copy: u32,
    rotation_deg: f64,
    turns: u8,
    mirrored: bool,
    min: DVec2,
    width: f64,
    height: f64,
}

impl Item {
    fn new(
        piece: &PatternPiece,
        mirrored: bool,
        cuts: u32,
        first_copy: u32,
    ) -> Result<Self, LayoutError> {
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
            cuts,
            first_copy,
            rotation_deg,
            turns,
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
            turns: self.turns,
            offset: DVec2::new(corner.x - self.min.x, corner.y - self.min.y),
            mirrored: self.mirrored,
        }
    }
}
