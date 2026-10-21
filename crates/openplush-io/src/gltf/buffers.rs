//! Every buffer's bytes: GLB's binary chunk, a `data:` URI, or a file beside the primary,
//! looked up by name in the files the shell supplied and never outside them.

use std::borrow::Cow;

use ::gltf::Document;
use ::gltf::buffer::Source;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::{Format, IoError};

/// One entry per buffer, in buffer order, each at least as long as it declares.
pub(super) fn resolve<'b>(
    doc: &Document,
    blob: Option<&'b [u8]>,
    primary: &str,
    sidecar: &dyn Fn(&str) -> Option<&'b [u8]>,
    format: Format,
) -> Result<Vec<Cow<'b, [u8]>>, IoError> {
    let parse = |detail: String| IoError::Parse { format, detail };
    let mut out = Vec::new();
    for buffer in doc.buffers() {
        let i = buffer.index();
        let data = match buffer.source() {
            Source::Bin => Cow::Borrowed(blob.ok_or_else(|| {
                parse(format!(
                    "buffer {i} is the binary chunk, which the file lacks"
                ))
            })?),
            Source::Uri(uri) => from_uri(uri, primary, sidecar).map_err(|e| match e {
                Refused::Missing => IoError::MissingResource {
                    uri: uri.to_owned(),
                },
                Refused::Outside => parse(format!(
                    "buffer {i} refers to \"{uri}\", outside the files supplied"
                )),
                Refused::NotBase64 => parse(format!("buffer {i}'s data URI is not base64 data")),
            })?,
        };
        if data.len() < buffer.length() {
            return Err(parse(format!(
                "buffer {i} holds {} bytes, fewer than the {} it declares",
                data.len(),
                buffer.length()
            )));
        }
        out.push(data);
    }
    Ok(out)
}

enum Refused {
    Missing,
    Outside,
    NotBase64,
}

fn from_uri<'b>(
    uri: &str,
    primary: &str,
    sidecar: &dyn Fn(&str) -> Option<&'b [u8]>,
) -> Result<Cow<'b, [u8]>, Refused> {
    if let Some(data) = uri.strip_prefix("data:") {
        return decode(data).map(Cow::Owned).ok_or(Refused::NotBase64);
    }
    let name = beside(primary, uri).ok_or(Refused::Outside)?;
    sidecar(&name).map(Cow::Borrowed).ok_or(Refused::Missing)
}

/// The bytes of a `data:` URI's text after the scheme, if it is base64.
fn decode(data: &str) -> Option<Vec<u8>> {
    let (header, payload) = data.split_once(',')?;
    header.ends_with(";base64").then_some(())?;
    STANDARD.decode(payload).ok()
}

/// The supplied file `uri` names, relative to the primary's directory; `None` when it has a
/// scheme, is absolute, has a bad percent escape, or climbs above the primary's directory
/// tree.
fn beside(primary: &str, uri: &str) -> Option<String> {
    if has_scheme(uri) || uri.starts_with('/') {
        return None;
    }
    let path = percent_decoded(uri)?;
    let mut parts: Vec<&str> = match primary.rsplit_once('/') {
        Some((dir, _)) => dir.split('/').collect(),
        None => Vec::new(),
    };
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(parts.join("/"))
}

/// RFC 3986: a letter, then letters, digits, `+`, `-` or `.`, then `:`, before any `/`.
fn has_scheme(uri: &str) -> bool {
    let head = uri.split('/').next().unwrap_or(uri);
    head.split_once(':').is_some_and(|(scheme, _)| {
        let mut chars = scheme.chars();
        chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

fn percent_decoded(uri: &str) -> Option<String> {
    let bytes = uri.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
