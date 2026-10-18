//! ASCII STL: `solid`, `facet`, `outer loop`, three `vertex` lines, `endloop`,
//! `endfacet`, `endsolid`, keywords in any case, one statement per line.

use openplush_geom::DVec3;

use super::{Corners, parse_error};
use crate::IoError;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Outside,
    Solid,
    Facet,
    Loop,
    LoopClosed,
}

pub(super) fn read(text: &str) -> Result<Corners, IoError> {
    let mut corners = Corners {
        positions: Vec::new(),
        solid_of_triangle: Vec::new(),
        solids: 0,
    };
    let mut state = State::Outside;
    let mut in_loop = 0usize;
    let mut last_line = 0;
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        last_line = line_number;
        let mut tokens = line.split_whitespace();
        let Some(keyword) = tokens.next() else {
            continue;
        };
        let word = |w: &str| keyword.eq_ignore_ascii_case(w);
        let fail = |reason: String| parse_error(format!("line {line_number}: {reason}"));
        state = match state {
            State::Outside if word("solid") => {
                corners.solids += 1;
                State::Solid
            }
            State::Solid if word("facet") => State::Facet,
            State::Solid if word("endsolid") => State::Outside,
            State::Facet if word("outer") => {
                expect_loop(&mut tokens).map_err(&fail)?;
                in_loop = 0;
                State::Loop
            }
            State::Loop if word("vertex") => {
                if in_loop == 3 {
                    return Err(fail("a facet has more than three vertices".into()));
                }
                in_loop += 1;
                corners.positions.push(vertex(&mut tokens).map_err(&fail)?);
                State::Loop
            }
            State::Loop if word("endloop") => {
                if in_loop != 3 {
                    return Err(fail(format!("a facet has {in_loop} vertices, not 3")));
                }
                corners.solid_of_triangle.push(corners.solids - 1);
                State::LoopClosed
            }
            State::LoopClosed if word("endfacet") => State::Solid,
            _ => return Err(fail(format!("unexpected `{keyword}`"))),
        };
    }
    if state != State::Outside {
        return Err(parse_error(format!(
            "line {}: the file ends inside a solid",
            last_line + 1
        )));
    }
    Ok(corners)
}

fn expect_loop<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Result<(), String> {
    match tokens.next() {
        Some(t) if t.eq_ignore_ascii_case("loop") => Ok(()),
        _ => Err("`outer` must be followed by `loop`".into()),
    }
}

/// Three coordinates and nothing after them.
fn vertex<'a>(tokens: &mut impl Iterator<Item = &'a str>) -> Result<DVec3, String> {
    let mut next = || {
        let token = tokens.next().ok_or("a vertex needs three coordinates")?;
        token
            .parse::<f64>()
            .map_err(|_| format!("`{token}` is not a number"))
    };
    let at = DVec3::new(next()?, next()?, next()?);
    match tokens.next() {
        Some(extra) => Err(format!("`{extra}` follows the three coordinates")),
        None => Ok(at),
    }
}
