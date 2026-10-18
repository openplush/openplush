//! The PLY header: its encoding and the elements the body holds, in order.

use super::Failure;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Encoding {
    Ascii,
    Little,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ty {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl Ty {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "char" | "int8" => Self::I8,
            "uchar" | "uint8" => Self::U8,
            "short" | "int16" => Self::I16,
            "ushort" | "uint16" => Self::U16,
            "int" | "int32" => Self::I32,
            "uint" | "uint32" => Self::U32,
            "float" | "float32" => Self::F32,
            "double" | "float64" => Self::F64,
            _ => return None,
        })
    }

    pub(super) fn is_integer(self) -> bool {
        !matches!(self, Self::F32 | Self::F64)
    }

    pub(super) fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Scalar(Ty),
    List { count: Ty, item: Ty },
}

#[derive(Debug)]
pub(super) struct Property {
    pub(super) name: String,
    pub(super) kind: Kind,
}

#[derive(Debug)]
pub(super) struct Element {
    pub(super) name: String,
    pub(super) count: u64,
    pub(super) properties: Vec<Property>,
}

#[derive(Debug)]
pub(super) struct Header {
    pub(super) encoding: Encoding,
    pub(super) elements: Vec<Element>,
    /// Where the body starts in the file, and the 1-based line it starts on.
    pub(super) body: usize,
    pub(super) body_line: usize,
}

fn fail(line: usize, reason: impl Into<String>) -> Failure {
    Failure(format!("line {line}: {}", reason.into()))
}

pub(super) fn parse(bytes: &[u8]) -> Result<Header, Failure> {
    let mut encoding = None;
    let mut elements: Vec<Element> = Vec::new();
    let mut offset = 0;
    for (index, raw) in bytes.split(|&b| b == b'\n').enumerate() {
        let line = index + 1;
        offset += raw.len() + 1;
        let text = std::str::from_utf8(raw)
            .map_err(|_| fail(line, "the header is not text"))?
            .trim();
        let mut tokens = text.split_whitespace();
        let keyword = tokens.next().unwrap_or("");
        if index == 0 {
            if keyword != "ply" || tokens.next().is_some() {
                return Err(fail(line, "a PLY file starts with `ply`"));
            }
            continue;
        }
        match keyword {
            "" | "comment" | "obj_info" => {}
            "format" => {
                encoding = Some(match tokens.next() {
                    Some("ascii") => Encoding::Ascii,
                    Some("binary_little_endian") => Encoding::Little,
                    Some("binary_big_endian") => Encoding::Big,
                    _ => return Err(fail(line, "the format is not ascii or binary")),
                });
            }
            "element" => elements.push(element(line, tokens)?),
            "property" => {
                let current = elements
                    .last_mut()
                    .ok_or_else(|| fail(line, "a property comes before any element"))?;
                current.properties.push(property(line, tokens)?);
            }
            "end_header" => {
                let encoding = encoding.ok_or_else(|| fail(line, "the header has no format"))?;
                return Ok(Header {
                    encoding,
                    elements,
                    body: offset.min(bytes.len()),
                    body_line: line + 1,
                });
            }
            other => return Err(fail(line, format!("unexpected `{other}` in the header"))),
        }
    }
    Err(Failure("the header never reaches `end_header`".into()))
}

fn element<'a>(line: usize, mut tokens: impl Iterator<Item = &'a str>) -> Result<Element, Failure> {
    let name = tokens
        .next()
        .ok_or_else(|| fail(line, "an element needs a name"))?;
    let count = tokens
        .next()
        .and_then(|c| c.parse::<u64>().ok())
        .ok_or_else(|| fail(line, "an element needs a count"))?;
    Ok(Element {
        name: name.to_owned(),
        count,
        properties: Vec::new(),
    })
}

fn property<'a>(
    line: usize,
    mut tokens: impl Iterator<Item = &'a str>,
) -> Result<Property, Failure> {
    let ty = |name: Option<&str>| {
        name.and_then(Ty::parse)
            .ok_or_else(|| fail(line, "a property needs a known type"))
    };
    let first = tokens.next();
    let kind = if first == Some("list") {
        Kind::List {
            count: ty(tokens.next())?,
            item: ty(tokens.next())?,
        }
    } else {
        Kind::Scalar(ty(first)?)
    };
    let name = tokens
        .next()
        .ok_or_else(|| fail(line, "a property needs a name"))?;
    Ok(Property {
        name: name.to_owned(),
        kind,
    })
}
