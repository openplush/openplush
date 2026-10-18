use std::str::SplitWhitespace;

use openplush_geom::{DVec2, DVec3};

use crate::triangulate::MAX_POLYGON_CORNERS;

/// One face corner: a position index, and a texture-coordinate index if the face gave one.
#[derive(Debug, Clone, Copy)]
pub(super) struct Corner {
    pub(super) vertex: usize,
    pub(super) uv: Option<usize>,
}

/// An `f` statement with its indices resolved but not yet range-checked.
#[derive(Debug)]
pub(super) struct Face {
    pub(super) line: usize,
    pub(super) primitive: u32,
    pub(super) corners: Vec<Corner>,
}

/// What a file's `v`, `vt` and `f` statements hold, in file order.
#[derive(Debug, Default)]
pub(super) struct Statements {
    pub(super) positions: Vec<DVec3>,
    /// The primitive current at each `v` line, so an unused vertex still has provenance.
    pub(super) position_primitives: Vec<u32>,
    pub(super) uvs: Vec<DVec2>,
    pub(super) faces: Vec<Face>,
    /// `usemtl` names as written, in first-use order, each once.
    pub(super) material_names: Vec<String>,
    /// Each primitive's index into `material_names`, for primitives that have faces.
    pub(super) primitive_materials: Vec<Option<usize>>,
}

/// Why a line was refused, with its 1-based line number.
#[derive(Debug)]
pub(super) struct LineError {
    pub(super) line: usize,
    pub(super) reason: String,
}

fn refuse<T>(line: usize, reason: impl Into<String>) -> Result<T, LineError> {
    Err(LineError {
        line,
        reason: reason.into(),
    })
}

/// A primitive is a maximal run of `f` lines with no `o`, `g` or `usemtl` between them,
/// so each one has a single material.
#[derive(Debug, Default)]
struct Primitives {
    current: u32,
    has_faces: bool,
    /// Set by `usemtl` and, as in the format, kept across `o` and `g`.
    material: Option<usize>,
}

impl Primitives {
    fn split(&mut self, line: usize) -> Result<(), LineError> {
        if self.has_faces {
            let Some(next) = self.current.checked_add(1) else {
                return refuse(line, "more primitives than a u32 can number");
            };
            self.current = next;
            self.has_faces = false;
        }
        Ok(())
    }
}

/// Reads every statement; indices are resolved, not yet checked against the final counts.
pub(super) fn statements(text: &str) -> Result<Statements, LineError> {
    let mut out = Statements::default();
    let mut primitives = Primitives::default();
    for (line, statement) in logical_lines(text) {
        let mut words = statement.split_whitespace();
        let Some(keyword) = words.next() else {
            continue;
        };
        match keyword {
            "v" => {
                out.positions.push(position(words, line)?);
                out.position_primitives.push(primitives.current);
            }
            "vt" => out.uvs.push(texcoord(words, line)?),
            "f" => {
                let corners = face(words, line, out.positions.len(), out.uvs.len())?;
                let primitive = primitives.current;
                out.faces.push(Face {
                    line,
                    primitive,
                    corners,
                });
                if !primitives.has_faces {
                    out.primitive_materials.push(primitives.material);
                }
                primitives.has_faces = true;
            }
            "o" | "g" => primitives.split(line)?,
            "usemtl" => {
                primitives.split(line)?;
                primitives.material = Some(material(&statement, line, &mut out.material_names)?);
            }
            // Free-form surfaces would vanish from the model if skipped.
            "cstype" | "curv" | "curv2" | "surf" => {
                return refuse(
                    line,
                    format!("free-form geometry ({keyword}) is not supported"),
                );
            }
            _ => {}
        }
    }
    Ok(out)
}

/// The index of the name `usemtl` gives, added if new. The name is the rest of the line
/// as written, trailing blanks included; only the blanks after the keyword are dropped.
fn material(statement: &str, line: usize, names: &mut Vec<String>) -> Result<usize, LineError> {
    let name = statement
        .trim_start()
        .strip_prefix("usemtl")
        .map_or("", str::trim_start);
    if name.is_empty() {
        return refuse(line, "usemtl needs a material name");
    }
    if let Some(i) = names.iter().position(|n| n == name) {
        return Ok(i);
    }
    names.push(name.to_owned());
    Ok(names.len() - 1)
}

/// Statements with comments removed and `\`-continued lines joined, each with the
/// number of the line it starts on.
fn logical_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut pending: Option<(usize, String)> = None;
    for (i, raw) in text.lines().enumerate() {
        let content = raw.split_once('#').map_or(raw, |(before, _)| before);
        // Trailing blanks stay on a finished line, because `usemtl` names keep them.
        let (body, continues) = match content.trim_end().strip_suffix('\\') {
            Some(body) => (body, true),
            None => (content, false),
        };
        let (start, mut joined) = pending.take().unwrap_or((i + 1, String::new()));
        joined.push(' ');
        joined.push_str(body);
        if continues {
            pending = Some((start, joined));
        } else {
            out.push((start, joined));
        }
    }
    out.extend(pending);
    out
}

fn numbers(words: SplitWhitespace<'_>, line: usize) -> Result<Vec<f64>, LineError> {
    words
        .map(|w| match w.parse::<f64>() {
            Ok(x) => Ok(x),
            Err(_) => refuse(line, format!("{w:?} is not a number")),
        })
        .collect()
}

/// `v x y z [w | r g b]`: the first three numbers. Non-finite values pass here, so
/// the caller can name every one.
fn position(words: SplitWhitespace<'_>, line: usize) -> Result<DVec3, LineError> {
    match numbers(words, line)?.as_slice() {
        [x, y, z, ..] => Ok(DVec3::new(*x, *y, *z)),
        _ => refuse(line, "a vertex needs three coordinates"),
    }
}

/// `vt u [v [w]]`, with `v` defaulting to 0 as the format specifies.
fn texcoord(words: SplitWhitespace<'_>, line: usize) -> Result<DVec2, LineError> {
    let n = numbers(words, line)?;
    let uv = match n.as_slice() {
        [u] => DVec2::new(*u, 0.0),
        [u, v] | [u, v, _] => DVec2::new(*u, *v),
        _ => return refuse(line, "a texture coordinate needs one to three numbers"),
    };
    if !uv.is_finite() {
        return refuse(line, "a texture coordinate is not finite");
    }
    Ok(uv)
}

/// `f v[/vt[/vn]] ...`: three or more corners. Normals are not read.
fn face(
    words: SplitWhitespace<'_>,
    line: usize,
    positions: usize,
    uvs: usize,
) -> Result<Vec<Corner>, LineError> {
    let corners = words
        .map(|word| corner(word, line, positions, uvs))
        .collect::<Result<Vec<_>, _>>()?;
    if corners.len() < 3 {
        return refuse(line, "a face needs at least three vertices");
    }
    if corners.len() > MAX_POLYGON_CORNERS {
        return refuse(
            line,
            format!(
                "a face has {} corners, over the {MAX_POLYGON_CORNERS} the reader triangulates",
                corners.len()
            ),
        );
    }
    Ok(corners)
}

fn corner(word: &str, line: usize, positions: usize, uvs: usize) -> Result<Corner, LineError> {
    let parts: Vec<&str> = word.split('/').collect();
    let (v, vt, vn) = match parts.as_slice() {
        [v] => (*v, "", ""),
        [v, vt] => (*v, *vt, ""),
        [v, vt, vn] => (*v, *vt, *vn),
        _ => return refuse(line, format!("{word:?} is not a face corner")),
    };
    let vertex = index(v, positions, line)?;
    let uv = if vt.is_empty() {
        None
    } else {
        Some(index(vt, uvs, line)?)
    };
    if !vn.is_empty() && vn.parse::<i64>().is_err() {
        return refuse(line, format!("{vn:?} is not a normal index"));
    }
    Ok(Corner { vertex, uv })
}

/// A 1-based index, or a negative one counting back from the `defined` items read so
/// far. A positive index may point past them; the caller checks the final count.
fn index(word: &str, defined: usize, line: usize) -> Result<usize, LineError> {
    let Ok(i) = word.parse::<i64>() else {
        return refuse(line, format!("{word:?} is not an index"));
    };
    let resolved = match i {
        0 => None,
        1.. => usize::try_from(i - 1).ok(),
        _ => usize::try_from(i.unsigned_abs())
            .ok()
            .and_then(|back| defined.checked_sub(back)),
    };
    match resolved {
        Some(r) => Ok(r),
        None => refuse(line, format!("index {i} refers to nothing")),
    }
}
