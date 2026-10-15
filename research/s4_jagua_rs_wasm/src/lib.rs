//! Spike S4: nest a few pattern-like pieces on a fabric strip with jagua-rs's
//! collision-detection engine and a greedy bottom-left placer, under the two
//! discrete rotation sets the data model passes to layout: {0°} (napped) and
//! {0°, 180°} (non-napped).
//! Every coordinate is millimetres; jagua-rs computes in f32.

use std::sync::Arc;

use jagua_rs::collision_detection::CDEConfig;
use jagua_rs::collision_detection::hazards::filter::NoFilter;
use jagua_rs::entities::{Item, Layout};
use jagua_rs::geometry::fail_fast::SPSurrogateConfig;
use jagua_rs::geometry::geo_enums::RotationRange;
use jagua_rs::geometry::geo_traits::Transformable;
use jagua_rs::geometry::{DTransformation, normalize_rotation};
use jagua_rs::io::ext_repr::{
    ExtContainer, ExtItem, ExtOrientation, ExtRotation, ExtSPolygon, ExtShape,
};
use jagua_rs::io::import::Importer;

/// Fabric width across the strip.
pub const FABRIC_WIDTH: f32 = 150.0;
/// Strip length offered to the placer; the used length is reported.
const STRIP_LENGTH: f32 = 1000.0;
/// Candidate grid pitch. 2 mm keeps the scan under a second natively.
const STEP: f32 = 2.0;

/// The fabric rotation sets under test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationSet {
    /// Napped fabric: {0°}.
    Fixed,
    /// Non-napped fabric: {0°, 180°}.
    HalfTurn,
}

impl RotationSet {
    fn degrees(self) -> Vec<f32> {
        match self {
            RotationSet::Fixed => vec![0.0],
            RotationSet::HalfTurn => vec![0.0, 180.0],
        }
    }
}

/// One placed piece: rotation in radians, translation of jagua's centred shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub id: u64,
    pub rotation: f32,
    pub translation: (f32, f32),
}

/// The finished nest: placements in placement order, used strip length, and a
/// FNV-1a hash over their bit patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct Nest {
    pub placements: Vec<Placement>,
    pub length: f32,
    pub hash: u64,
}

/// Asymmetric outlines (counter-clockwise, mm) so that a half turn changes the fit.
pub fn pieces() -> Vec<(u64, Vec<(f32, f32)>)> {
    let tri = vec![(0.0, 0.0), (70.0, 0.0), (0.0, 50.0)];
    let trap = vec![(0.0, 0.0), (80.0, 0.0), (60.0, 40.0), (10.0, 40.0)];
    let ell = vec![
        (0.0, 0.0),
        (50.0, 0.0),
        (50.0, 20.0),
        (20.0, 20.0),
        (20.0, 60.0),
        (0.0, 60.0),
    ];
    let rect = vec![(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)];
    vec![
        (0, rect),
        (1, tri.clone()),
        (2, tri),
        (3, trap.clone()),
        (4, trap),
        (5, ell),
    ]
}

fn importer() -> Importer {
    let cde = CDEConfig {
        quadtree_depth: 5,
        cd_threshold: 16,
        item_surrogate_config: SPSurrogateConfig::none(),
    };
    Importer::new(cde, None, None)
}

/// Imports one piece with `set`, optionally mirrored about a local axis (degrees).
pub fn import_piece(
    id: u64,
    outline: &[(f32, f32)],
    set: RotationSet,
    mirror_axis: Option<f32>,
) -> Result<Arc<Item>, String> {
    let ext = ExtItem {
        id,
        orientation: ExtOrientation {
            rotation: ExtRotation::Discrete {
                angles: set.degrees(),
            },
            reflection_axes: mirror_axis.into_iter().collect(),
        },
        shape: ExtShape::SimplePolygon(ExtSPolygon(outline.to_vec())),
        min_quality: None,
    };
    importer()
        .import_item(&ext, usize::try_from(id).map_err(|e| e.to_string())?)
        .map_err(|e| format!("import piece {id}: {e}"))
}

/// Greedy constructive nest: largest piece first; for each allowed rotation the
/// first collision-free grid position in (y, x) order; keep the lowest top edge.
pub fn nest(set: RotationSet) -> Result<Nest, String> {
    let container = importer()
        .import_container(&ExtContainer {
            id: 0,
            shape: ExtShape::Rectangle {
                x_min: 0.0,
                y_min: 0.0,
                width: FABRIC_WIDTH,
                height: STRIP_LENGTH,
            },
            zones: vec![],
        })
        .map_err(|e| format!("import strip: {e}"))?;
    let mut items = pieces()
        .iter()
        .map(|(id, outline)| import_piece(*id, outline, set, None))
        .collect::<Result<Vec<_>, _>>()?;
    items.sort_by(|a, b| b.area().total_cmp(&a.area()).then(a.idx().cmp(&b.idx())));
    let mut layout = Layout::new(container);
    let mut placements = Vec::new();
    let mut length = 0.0f32;
    for item in &items {
        let (top, t) = best_position(&layout, item)?
            .ok_or_else(|| format!("piece {} fits nowhere", item.external_id()))?;
        if !item.allowed_orientations().allows(&t) {
            return Err(format!(
                "piece {} placed off its rotation set",
                item.external_id()
            ));
        }
        layout.place_item(item, t);
        length = length.max(top);
        placements.push(Placement {
            id: item.external_id(),
            rotation: t.rotation(),
            translation: t.translation(),
        });
    }
    if !layout.is_collision_free() {
        return Err("layout has a collision".to_string());
    }
    let hash = fingerprint(&placements, length);
    Ok(Nest {
        placements,
        length,
        hash,
    })
}

fn rotations(item: &Item) -> Result<Vec<f32>, String> {
    match item.allowed_orientations().rotations() {
        RotationRange::None => Ok(vec![0.0]),
        RotationRange::Discrete(angles) => Ok(angles.clone()),
        RotationRange::Continuous => Err("continuous rotation is out of scope".to_string()),
    }
}

/// Returns the top edge and pose of the best candidate, or `None` if nothing fits.
fn best_position(layout: &Layout, item: &Item) -> Result<Option<(f32, DTransformation)>, String> {
    let mut best: Option<(f32, f32, DTransformation)> = None;
    for r in rotations(item)? {
        let probe = item
            .shape_cd()
            .transform_clone(&DTransformation::new(r, (0.0, 0.0)).compose());
        let bb = probe.bbox();
        let Some((x, y)) = first_free(
            layout,
            item,
            r,
            (bb.x_min, bb.y_min),
            (bb.width(), bb.height()),
        ) else {
            continue;
        };
        let top = y + bb.height();
        let pose = DTransformation::new(r, (x - bb.x_min, y - bb.y_min));
        let better =
            best.is_none_or(|(bt, bx, _)| top.total_cmp(&bt).then(x.total_cmp(&bx)).is_lt());
        if better {
            best = Some((top, x, pose));
        }
    }
    Ok(best.map(|(top, _, pose)| (top, pose)))
}

/// First grid corner (x, y) in row-major order where the rotated piece is free.
fn first_free(
    layout: &Layout,
    item: &Item,
    r: f32,
    origin: (f32, f32),
    size: (f32, f32),
) -> Option<(f32, f32)> {
    let grid = |extent: f32, span: f32| ((extent - span) / STEP).floor() as u32;
    for iy in 0..=grid(STRIP_LENGTH, size.1) {
        let y = iy as f32 * STEP;
        for ix in 0..=grid(FABRIC_WIDTH, size.0) {
            let x = ix as f32 * STEP;
            let pose = DTransformation::new(r, (x - origin.0, y - origin.1));
            let shape = item.shape_cd().transform_clone(&pose.compose());
            if !layout.cde().detect_poly_collision(&shape, &NoFilter) {
                return Some((x, y));
            }
        }
    }
    None
}

/// Whether a rotation (radians) is a half turn, within f32 rounding.
pub fn is_half_turn(rotation: f32) -> bool {
    (normalize_rotation(rotation) - std::f32::consts::PI).abs() < 1e-6
}

fn fingerprint(placements: &[Placement], length: f32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut write = |word: u64| {
        for byte in word.to_le_bytes() {
            h ^= u64::from(byte);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    for p in placements {
        write(p.id);
        write(u64::from(p.rotation.to_bits()));
        write(u64::from(p.translation.0.to_bits()));
        write(u64::from(p.translation.1.to_bits()));
    }
    write(u64::from(length.to_bits()));
    h
}
