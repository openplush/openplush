//! A small PDF reader for the tests, independent of the writer: it finds each page's media box
//! and content stream through the page tree, and interprets the drawing operators the writer may
//! use, so what a page shows is measured rather than read back from the writer's own numbers.

use std::collections::BTreeMap;

use openplush_geom::DVec2;
use openplush_geom::units::Mm;
use openplush_layout::shelf::pack;
use openplush_layout::{LayoutError, LayoutOptions, Sheet};
use openplush_pattern::{CutCount, Outline, PatternPiece, PieceId};

pub(crate) type Setup = Box<dyn std::error::Error>;

const MM_PER_PT: f64 = 25.4 / 72.0;

/// What one page draws, in page millimetres from its lower-left corner.
#[derive(Debug, Default)]
pub(crate) struct Page {
    /// Width and height.
    pub(crate) size: DVec2,
    /// Each stroked subpath's points, in drawing order, with the line width and dash it used.
    pub(crate) strokes: Vec<Stroke>,
    /// Each clipping path's points.
    pub(crate) clips: Vec<Vec<DVec2>>,
    /// Each shown string, in drawing order.
    pub(crate) texts: Vec<Text>,
}

#[derive(Debug, Clone)]
pub(crate) struct Text {
    pub(crate) text: String,
    /// Where its baseline starts.
    pub(crate) at: DVec2,
    /// Font size in page millimetres.
    pub(crate) size: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct Stroke {
    pub(crate) points: Vec<DVec2>,
    /// In page millimetres.
    pub(crate) width: f64,
    /// Dash lengths in page millimetres; empty for a solid line.
    pub(crate) dash: Vec<f64>,
    /// Stroke gray level: 0 is black.
    pub(crate) gray: f64,
}

/// One `w` × `h` mm rectangle, cut and sewn on the same line, packed alone on a roll 20 mm wider.
pub(crate) fn rectangle(w: f64, h: f64) -> Result<(Sheet, Vec<PatternPiece>), Setup> {
    let outline = Outline::new(vec![
        DVec2::new(0.0, 0.0),
        DVec2::new(w, 0.0),
        DVec2::new(w, h),
        DVec2::new(0.0, h),
    ])?;
    let pieces = vec![PatternPiece::new(
        PieceId::new(0),
        outline.clone(),
        outline,
        CutCount::new(1, 0)?,
        vec![0.0],
    )?];
    let opts = LayoutOptions::new(Mm::try_new(w + 20.0)?, Mm::try_new(0.0)?, false)?;
    let sheet = pack(&pieces, &opts)?
        .into_iter()
        .next()
        .ok_or(LayoutError::NoPieces)?;
    Ok((sheet, pieces))
}

/// Every page after the cover, in page-tree order.
pub(crate) fn tiles(pdf: &[u8]) -> Result<Vec<Page>, Setup> {
    let mut all = pages(pdf)?;
    if all.is_empty() {
        return Err("no cover".into());
    }
    all.remove(0);
    Ok(all)
}

/// Every page in page-tree order.
pub(crate) fn pages(pdf: &[u8]) -> Result<Vec<Page>, Setup> {
    // The header's binary comment is not UTF-8; everything the reader needs is ASCII.
    let text = String::from_utf8_lossy(pdf);
    let objects = objects(&text)?;
    let catalog = objects
        .values()
        .find(|o| o.contains("/Type /Catalog"))
        .ok_or("no catalog")?;
    let tree = &objects[&reference(catalog, "/Pages")?];
    kids(tree)?
        .into_iter()
        .map(|id| {
            let page = &objects[&id];
            let media = numbers(bracketed(page, "/MediaBox")?)?;
            let content = &objects[&reference(page, "/Contents")?];
            let stream = content
                .split("stream\n")
                .nth(1)
                .and_then(|s| s.split("\nendstream").next())
                .ok_or("no stream")?;
            let mut drawn = interpret(stream)?;
            drawn.size = DVec2::new(media[2] - media[0], media[3] - media[1]) * MM_PER_PT;
            Ok(drawn)
        })
        .collect()
}

/// Object number to body, for every `N 0 obj … endobj`.
fn objects(text: &str) -> Result<BTreeMap<u32, String>, Setup> {
    let mut out = BTreeMap::new();
    for chunk in text.split("endobj").filter(|c| c.contains(" 0 obj")) {
        let (head, body) = chunk.split_once(" 0 obj").ok_or("bad object")?;
        let id = head.split_whitespace().last().ok_or("no id")?.parse()?;
        out.insert(id, body.to_owned());
    }
    Ok(out)
}

fn reference(body: &str, key: &str) -> Result<u32, Setup> {
    let rest = body.split(key).nth(1).ok_or("missing key")?;
    Ok(rest.split_whitespace().next().ok_or("no ref")?.parse()?)
}

fn bracketed<'a>(body: &'a str, key: &str) -> Result<&'a str, Setup> {
    let rest = body.split(key).nth(1).ok_or("missing key")?;
    let inner = rest.split_once('[').ok_or("no [")?.1;
    Ok(inner.split_once(']').ok_or("no ]")?.0)
}

fn numbers(s: &str) -> Result<Vec<f64>, Setup> {
    Ok(s.split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()?)
}

fn kids(tree: &str) -> Result<Vec<u32>, Setup> {
    let inner = bracketed(tree, "/Kids")?;
    let words: Vec<&str> = inner.split_whitespace().collect();
    words
        .chunks(3)
        .map(|r| Ok(r.first().ok_or("bad kid")?.parse()?))
        .collect()
}

/// The page's current transformation, PDF's row-vector convention: `[a b c d e f]`.
type Matrix = [f64; 6];

fn apply(m: &Matrix, x: f64, y: f64) -> DVec2 {
    DVec2::new(m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// `n` then the current matrix: the new CTM after `cm`.
fn concat(n: &Matrix, m: &Matrix) -> Matrix {
    [
        n[0] * m[0] + n[1] * m[2],
        n[0] * m[1] + n[1] * m[3],
        n[2] * m[0] + n[3] * m[2],
        n[2] * m[1] + n[3] * m[3],
        n[4] * m[0] + n[5] * m[2] + m[4],
        n[4] * m[1] + n[5] * m[3] + m[5],
    ]
}

#[derive(Clone)]
struct State {
    ctm: Matrix,
    width: f64,
    dash: Vec<f64>,
    gray: f64,
}

enum Token<'a> {
    Number(f64),
    Array(Vec<f64>),
    Text(String),
    Operator(&'a str),
}

/// Numbers, `[…]` arrays of numbers, `(…)` strings with no escapes, and everything else as an
/// operator.
fn tokens(stream: &str) -> Result<Vec<Token<'_>>, Setup> {
    let mut out = Vec::new();
    let mut array: Option<Vec<f64>> = None;
    let mut text: Option<String> = None;
    for token in stream.split_whitespace() {
        if let Some(t) = text.as_mut() {
            t.push(' ');
            t.push_str(token);
        } else if token.starts_with('(') {
            text = Some(token.to_owned());
        }
        if let Some(t) = text.as_ref().filter(|t| t.ends_with(')')) {
            out.push(Token::Text(t[1..t.len() - 1].to_owned()));
            text = None;
            continue;
        } else if text.is_some() {
            continue;
        }
        let opens = token.starts_with('[');
        let closes = token.ends_with(']');
        let word = token.trim_start_matches('[').trim_end_matches(']');
        if opens {
            array = Some(Vec::new());
        }
        match array.as_mut() {
            Some(a) => {
                if !word.is_empty() {
                    a.push(word.parse()?);
                }
                if closes {
                    out.push(Token::Array(array.take().unwrap_or_default()));
                }
            }
            None => out.push(match word.parse::<f64>() {
                Ok(v) => Token::Number(v),
                Err(_) => Token::Operator(word),
            }),
        }
    }
    Ok(out)
}

/// Runs the path, state and painting operators; text and anything else is skipped.
fn interpret(stream: &str) -> Result<Page, Setup> {
    let mut run = Run {
        page: Page::default(),
        state: State {
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            width: 1.0,
            dash: Vec::new(),
            gray: 0.0,
        },
        saved: Vec::new(),
        path: Vec::new(),
        clip: false,
        font: 0.0,
        text_at: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };
    let (mut operands, mut array, mut text) = (Vec::new(), Vec::new(), String::new());
    for token in tokens(stream)? {
        match token {
            Token::Number(v) => operands.push(v),
            Token::Array(a) => array = a,
            Token::Text(t) => text = t,
            Token::Operator("Tf") => {
                run.font = operands[0];
                operands.clear();
            }
            Token::Operator("Tm") => {
                let o = &operands;
                run.text_at = [o[0], o[1], o[2], o[3], o[4], o[5]];
                operands.clear();
            }
            Token::Operator("Tj") => {
                let m = concat(&run.text_at, &run.state.ctm);
                run.page.texts.push(Text {
                    text: std::mem::take(&mut text),
                    at: apply(&m, 0.0, 0.0) * MM_PER_PT,
                    size: run.font * scale(&m),
                });
            }
            Token::Operator(op) => {
                run.operator(op, &operands, &array)?;
                operands.clear();
            }
        }
    }
    Ok(run.page)
}

struct Run {
    page: Page,
    state: State,
    saved: Vec<State>,
    path: Vec<Vec<DVec2>>,
    clip: bool,
    font: f64,
    text_at: Matrix,
}

impl Run {
    fn at(&self, x: f64, y: f64) -> DVec2 {
        apply(&self.state.ctm, x, y) * MM_PER_PT
    }

    fn operator(&mut self, op: &str, o: &[f64], array: &[f64]) -> Result<(), Setup> {
        match op {
            "q" => self.saved.push(self.state.clone()),
            "Q" => self.state = self.saved.pop().ok_or("unbalanced Q")?,
            "cm" => self.state.ctm = concat(&[o[0], o[1], o[2], o[3], o[4], o[5]], &self.state.ctm),
            "w" => self.state.width = o[0] * scale(&self.state.ctm),
            "G" => self.state.gray = o[0],
            "d" => {
                let k = scale(&self.state.ctm);
                self.state.dash = array.iter().map(|v| v * k).collect();
            }
            "m" => self.path.push(vec![self.at(o[0], o[1])]),
            "l" => {
                let p = self.at(o[0], o[1]);
                self.path.last_mut().ok_or("l without m")?.push(p);
            }
            "re" => {
                let (x, y, w, h) = (o[0], o[1], o[2], o[3]);
                let c = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
                let corners = c.iter().map(|&(a, b)| self.at(a, b)).collect();
                self.path.push(corners);
            }
            "W" => self.clip = true,
            "S" | "s" => {
                for points in self.path.drain(..) {
                    self.page.strokes.push(Stroke {
                        points,
                        width: self.state.width,
                        dash: self.state.dash.clone(),
                        gray: self.state.gray,
                    });
                }
            }
            "n" => {
                if self.clip {
                    self.page.clips.append(&mut self.path);
                }
                self.path.clear();
                self.clip = false;
            }
            _ => {}
        }
        Ok(())
    }
}

/// How much the matrix scales a length, for an unrotated, unsheared, uniform matrix.
fn scale(m: &Matrix) -> f64 {
    m[0].abs() * MM_PER_PT
}
