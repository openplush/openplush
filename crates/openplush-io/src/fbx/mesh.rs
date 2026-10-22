//! A `Mesh` geometry's control points, polygons, material slots and first UV layer.

use openplush_geom::DVec2;

use super::tree::{Node, floats, ints};
use crate::triangulate::MAX_POLYGON_CORNERS;

/// A polygon mesh as the file writes it, every index checked against its array.
pub(super) struct Mesh {
    pub(super) points: Vec<[f64; 3]>,
    /// Control-point indices, polygon after polygon.
    pub(super) corners: Vec<u32>,
    /// Where each polygon's corners start in `corners`, then where the last one ends.
    pub(super) starts: Vec<usize>,
    /// Each polygon's material slot; `None` when the mesh has no material layer.
    pub(super) slots: Option<Vec<u32>>,
    /// Each corner's UV, in `corners` order, from the first UV layer.
    pub(super) uvs: Option<Vec<DVec2>>,
    /// UV layers beyond the one read.
    pub(super) extra_uv_sets: u32,
}

impl Mesh {
    pub(super) fn polygon_count(&self) -> usize {
        self.starts.len() - 1
    }
}

pub(super) fn read(geometry: &Node) -> Result<Mesh, String> {
    let values = floats(geometry.child("Vertices").and_then(|v| v.attr(0)))
        .ok_or("it has no Vertices array of numbers")?;
    if !values.len().is_multiple_of(3) {
        return Err(format!(
            "its Vertices array has {} numbers, not three per control point",
            values.len()
        ));
    }
    let points: Vec<[f64; 3]> = values.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();
    if u32::try_from(points.len()).is_err() {
        return Err(format!(
            "{} control points exceed the u32 index space",
            points.len()
        ));
    }
    let indices = ints(geometry.child("PolygonVertexIndex").and_then(|v| v.attr(0)))
        .ok_or("it has no PolygonVertexIndex array of integers")?;
    let (corners, starts) = polygons(indices, points.len())?;
    let slots = slots(geometry, starts.len() - 1)?;
    let (uvs, extra_uv_sets) = uvs(geometry, &corners)?;
    Ok(Mesh {
        points,
        corners,
        starts,
        slots,
        uvs,
        extra_uv_sets,
    })
}

/// FBX ends each polygon with its last index written as `-i - 1`.
fn polygons(indices: &[i32], points: usize) -> Result<(Vec<u32>, Vec<usize>), String> {
    let mut corners = Vec::with_capacity(indices.len());
    let mut starts = vec![0];
    for (k, &i) in indices.iter().enumerate() {
        let v = if i < 0 { !i } else { i };
        // Lossless: `v` is not negative.
        let v = v as u32;
        if v as usize >= points {
            return Err(format!(
                "polygon vertex {k} is control point {v}, past the last of its {points}"
            ));
        }
        corners.push(v);
        if i < 0 {
            let start = starts.len() - 1;
            let n = corners.len() - starts[start];
            if n < 3 {
                return Err(format!("polygon {start} has {n} corners, under three"));
            }
            if n > MAX_POLYGON_CORNERS {
                return Err(format!(
                    "polygon {start} has {n} corners, over the {MAX_POLYGON_CORNERS} the reader triangulates"
                ));
            }
            starts.push(corners.len());
        }
    }
    if starts.last() != Some(&corners.len()) {
        return Err("its last polygon is not closed by a negative index".into());
    }
    Ok((corners, starts))
}

/// The first `LayerElementMaterial`: one slot for every polygon or one per polygon.
fn slots(geometry: &Node, polygons: usize) -> Result<Option<Vec<u32>>, String> {
    let Some(layer) = geometry.child("LayerElementMaterial") else {
        return Ok(None);
    };
    let values = ints(layer.child("Materials").and_then(|m| m.attr(0)))
        .ok_or("its material layer has no Materials array of integers")?;
    let slot = |k: usize| -> Result<u32, String> {
        let s = *values
            .get(k)
            .ok_or_else(|| format!("its material layer has no slot for polygon {k}"))?;
        u32::try_from(s).map_err(|_| format!("polygon {k} has material slot {s}"))
    };
    let slots = match layer.child_text("MappingInformationType") {
        Some("AllSame") => vec![slot(0)?; polygons],
        Some("ByPolygon") => (0..polygons).map(slot).collect::<Result<_, _>>()?,
        other => {
            return Err(format!(
                "its material layer's mapping \"{}\" is not AllSame or ByPolygon",
                other.unwrap_or_default()
            ));
        }
    };
    Ok(Some(slots))
}

/// The first `LayerElementUV`'s UV at each corner; the further UV layers are counted.
fn uvs(geometry: &Node, corners: &[u32]) -> Result<(Option<Vec<DVec2>>, u32), String> {
    let mut layers = geometry.children_named("LayerElementUV");
    let Some(layer) = layers.next() else {
        return Ok((None, 0));
    };
    let extra = u32::try_from(layers.count()).map_err(|_| "it has too many UV layers")?;
    let layer = UvLayer::read(layer)?;
    let uvs = corners
        .iter()
        .enumerate()
        .map(|(k, &v)| layer.at(k, v))
        .collect::<Result<_, _>>()?;
    Ok((Some(uvs), extra))
}

/// A UV layer's table and how a corner finds its entry.
struct UvLayer<'a> {
    table: Vec<DVec2>,
    by_control_point: bool,
    /// `UVIndex` for index-to-direct reference; `None` for direct.
    index: Option<&'a [i32]>,
}

impl<'a> UvLayer<'a> {
    fn read(layer: &'a Node) -> Result<Self, String> {
        let values = floats(layer.child("UV").and_then(|u| u.attr(0)))
            .ok_or("its UV layer has no UV array")?;
        if !values.len().is_multiple_of(2) {
            return Err(format!(
                "its UV array has {} numbers, not two per UV",
                values.len()
            ));
        }
        let by_control_point = match layer.child_text("MappingInformationType") {
            Some("ByPolygonVertex") => false,
            // The SDK writes by-control-point mapping as "ByVertice".
            Some("ByVertice" | "ByControlPoint") => true,
            other => {
                let other = other.unwrap_or_default();
                return Err(format!("its UV layer's mapping \"{other}\" is not read"));
            }
        };
        let index = match layer.child_text("ReferenceInformationType") {
            Some("Direct") => None,
            Some("IndexToDirect") => Some(
                ints(layer.child("UVIndex").and_then(|u| u.attr(0)))
                    .ok_or("its UV layer has no UVIndex array of integers")?,
            ),
            other => {
                let other = other.unwrap_or_default();
                return Err(format!("its UV layer's reference \"{other}\" is not read"));
            }
        };
        Ok(Self {
            table: values
                .chunks_exact(2)
                .map(|t| DVec2::new(t[0], t[1]))
                .collect(),
            by_control_point,
            index,
        })
    }

    /// The UV of corner `k`, at control point `v`.
    fn at(&self, k: usize, v: u32) -> Result<DVec2, String> {
        let key = if self.by_control_point { v as usize } else { k };
        let at = match self.index {
            None => key,
            Some(index) => {
                let i = *index
                    .get(key)
                    .ok_or_else(|| format!("its UVIndex has no entry {key}"))?;
                usize::try_from(i).map_err(|_| format!("its UVIndex entry {key} is {i}"))?
            }
        };
        let t = *self
            .table
            .get(at)
            .ok_or_else(|| format!("UV {at} is past the last of its {} UVs", self.table.len()))?;
        if !(t.x.is_finite() && t.y.is_finite()) {
            return Err(format!("corner {k} has a non-finite texture coordinate"));
        }
        Ok(t)
    }
}
