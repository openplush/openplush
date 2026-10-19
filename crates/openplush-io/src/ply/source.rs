//! The body's values, one at a time, from ASCII text or binary bytes.

use super::Failure;
use super::header::{Encoding, Ty};

pub(super) enum Source<'a> {
    Ascii {
        tokens: Box<dyn Iterator<Item = (usize, &'a str)> + 'a>,
    },
    Binary {
        bytes: &'a [u8],
        at: usize,
        big: bool,
    },
}

impl<'a> Source<'a> {
    pub(super) fn new(
        encoding: Encoding,
        bytes: &'a [u8],
        body: usize,
        body_line: usize,
    ) -> Result<Self, Failure> {
        let rest = &bytes[body..];
        if encoding == Encoding::Ascii {
            let text = std::str::from_utf8(rest).map_err(|e| {
                Failure(format!(
                    "byte {}: the body is not text",
                    body + e.valid_up_to()
                ))
            })?;
            let tokens = text.lines().enumerate().flat_map(move |(i, line)| {
                line.split_whitespace().map(move |t| (body_line + i, t))
            });
            Ok(Self::Ascii {
                tokens: Box::new(tokens),
            })
        } else {
            Ok(Self::Binary {
                bytes,
                at: body,
                big: encoding == Encoding::Big,
            })
        }
    }

    /// The next value of type `ty`; every PLY type up to 32 bits is exact in `f64`.
    pub(super) fn next(&mut self, ty: Ty) -> Result<f64, Failure> {
        match self {
            Self::Ascii { tokens } => {
                let (line, token) = tokens
                    .next()
                    .ok_or_else(|| Failure("the body ends before its last value".into()))?;
                let value: f64 = token
                    .parse()
                    .map_err(|_| Failure(format!("line {line}: `{token}` is not a number")))?;
                if ty.is_integer() {
                    let (low, high) = ty.range();
                    if value.fract() != 0.0 || value < low || value > high {
                        return Err(Failure(format!(
                            "line {line}: `{token}` is not an integer of the declared type"
                        )));
                    }
                }
                // A `float` is 32 bits however it is written, as in a binary file.
                Ok(if ty == Ty::F32 {
                    f64::from(value as f32)
                } else {
                    value
                })
            }
            Self::Binary { bytes, at, big } => {
                let end = *at + ty.size();
                let raw = bytes.get(*at..end).ok_or_else(|| {
                    Failure(format!("byte {}: the body ends before its last value", *at))
                })?;
                let mut buf = [0u8; 8];
                buf[..raw.len()].copy_from_slice(raw);
                if *big {
                    buf[..raw.len()].reverse();
                }
                *at = end;
                Ok(decode(ty, buf))
            }
        }
    }
}

/// `buf` holds the value's bytes, least significant first.
fn decode(ty: Ty, b: [u8; 8]) -> f64 {
    match ty {
        Ty::I8 => f64::from(i8::from_le_bytes([b[0]])),
        Ty::U8 => f64::from(b[0]),
        Ty::I16 => f64::from(i16::from_le_bytes([b[0], b[1]])),
        Ty::U16 => f64::from(u16::from_le_bytes([b[0], b[1]])),
        Ty::I32 => f64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        Ty::U32 => f64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        Ty::F32 => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        Ty::F64 => f64::from_le_bytes(b),
    }
}
