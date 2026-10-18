//! Reading the vertex and face elements from the body, and reading past the rest.

use openplush_geom::{DVec2, DVec3};

use super::Failure;
use super::header::{Element, Kind, Property, Ty};
use super::source::Source;

/// Ear clipping a polygon takes about the cube of its corners, so a file cannot make it run for minutes.
const MAX_FACE_CORNERS: u64 = 1000;

pub(super) struct Vertices {
    pub(super) positions: Vec<DVec3>,
    pub(super) uvs: Option<Vec<DVec2>>,
}

/// The slots of the properties a vertex row is read from.
struct VertexSlots {
    xyz: [usize; 3],
    uv: Option<[usize; 2]>,
}

const UV_NAMES: [[&str; 2]; 4] = [
    ["s", "t"],
    ["u", "v"],
    ["texture_s", "texture_t"],
    ["texture_u", "texture_v"],
];

fn slot(properties: &[Property], name: &str) -> Option<usize> {
    properties
        .iter()
        .position(|p| p.name == name && matches!(p.kind, Kind::Scalar(_)))
}

impl VertexSlots {
    /// `x`, `y` and `z` exist: the header check passed.
    fn of(properties: &[Property]) -> Option<Self> {
        let xyz = [
            slot(properties, "x")?,
            slot(properties, "y")?,
            slot(properties, "z")?,
        ];
        let uv = UV_NAMES
            .iter()
            .find_map(|[u, v]| Some([slot(properties, u)?, slot(properties, v)?]));
        Some(Self { xyz, uv })
    }
}

/// One row's values in property order; a list property reads past and leaves 0.
fn row(properties: &[Property], source: &mut Source) -> Result<Vec<f64>, Failure> {
    properties
        .iter()
        .map(|p| match p.kind {
            Kind::Scalar(ty) => source.next(ty),
            Kind::List { count, item } => {
                skip_list(source, count, item)?;
                Ok(0.0)
            }
        })
        .collect()
}

/// A list's length: a count type is an integer, but a signed one can still be negative.
fn list_len(source: &mut Source, count: Ty) -> Result<u64, Failure> {
    let len = source.next(count)?;
    if len < 0.0 {
        return Err(Failure(format!("a list has the negative length {len}")));
    }
    // Lossless: an integer count type is at most 32 bits and now non-negative.
    Ok(len as u64)
}

fn skip_list(source: &mut Source, count: Ty, item: Ty) -> Result<(), Failure> {
    for _ in 0..list_len(source, count)? {
        source.next(item)?;
    }
    Ok(())
}

pub(super) fn read_vertices(element: &Element, source: &mut Source) -> Result<Vertices, Failure> {
    let slots = VertexSlots::of(&element.properties)
        .ok_or_else(|| Failure("the `vertex` element needs scalar `x`, `y` and `z`".into()))?;
    let mut positions = Vec::new();
    let mut uvs = slots.uv.map(|_| Vec::new());
    for _ in 0..element.count {
        let values = row(&element.properties, source)?;
        // INVARIANT: `values` has one entry per property, and the slots index the properties.
        let [x, y, z] = slots.xyz.map(|i| values[i]);
        positions.push(DVec3::new(x, y, z));
        if let (Some(list), Some([u, v])) = (uvs.as_mut(), slots.uv) {
            list.push(DVec2::new(values[u], values[v]));
        }
    }
    Ok(Vertices { positions, uvs })
}

/// Every face's corner list, each checked to have three or more corners below `vertex_count`.
pub(super) fn read_faces(
    element: &Element,
    source: &mut Source,
    vertex_count: usize,
) -> Result<Vec<Vec<u32>>, Failure> {
    let list = element
        .properties
        .iter()
        .position(|p| {
            matches!(p.kind, Kind::List { .. })
                && ["vertex_indices", "vertex_index"].contains(&p.name.as_str())
        })
        .ok_or_else(|| Failure("the `face` element needs a `vertex_indices` list".into()))?;
    let mut faces = Vec::new();
    for face in 0..element.count {
        let mut corners = Vec::new();
        for (i, p) in element.properties.iter().enumerate() {
            match p.kind {
                Kind::Scalar(ty) => {
                    source.next(ty)?;
                }
                Kind::List { count, item } if i == list => {
                    let len = list_len(source, count)?;
                    if len > MAX_FACE_CORNERS {
                        return Err(Failure(format!(
                            "face {face} has {len} corners, over the {MAX_FACE_CORNERS} the reader triangulates"
                        )));
                    }
                    for _ in 0..len {
                        corners.push(source.next(item)?);
                    }
                }
                Kind::List { count, item } => skip_list(source, count, item)?,
            }
        }
        if corners.len() < 3 {
            return Err(Failure(format!(
                "face {face} has {} corners, not 3 or more",
                corners.len()
            )));
        }
        let corners = corners
            .into_iter()
            .map(|c| {
                // Lossless: a PLY index type is at most 32 bits, and negatives fail the range test.
                let index = c as u32;
                (c >= 0.0 && (index as usize) < vertex_count).then_some(index)
            })
            .collect::<Option<Vec<u32>>>()
            .ok_or_else(|| Failure(format!("face {face} refers past the last vertex")))?;
        faces.push(corners);
    }
    Ok(faces)
}

pub(super) fn skip(element: &Element, source: &mut Source) -> Result<(), Failure> {
    if element.properties.is_empty() {
        return Ok(());
    }
    for _ in 0..element.count {
        row(&element.properties, source)?;
    }
    Ok(())
}
