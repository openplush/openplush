//! Reading the vertex and face elements from the body, and reading past the rest.

use openplush_geom::{DVec2, DVec3};

use super::Failure;
use super::header::{Element, Kind, Property};
use super::source::Source;

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

fn skip_list(
    source: &mut Source,
    count: super::header::Ty,
    item: super::header::Ty,
) -> Result<(), Failure> {
    for _ in 0..source.next(count)? as u64 {
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
                    for _ in 0..source.next(count)? as u64 {
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
