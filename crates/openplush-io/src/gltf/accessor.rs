//! Accessor reads, each checked against its declared type and its buffer view's bounds.
//! The `gltf` crate's own reader trusts the declared component type, so it would misread
//! an unexpected one, or panic on it, where this module refuses it.

use std::borrow::Cow;

use ::gltf::Accessor;
use ::gltf::accessor::{DataType, Dimensions};

/// What an accessor is read as.
#[derive(Clone, Copy)]
pub(super) enum Use {
    /// `POSITION`: three floats.
    Position,
    /// `TEXCOORD_n`: two floats, or two normalized unsigned bytes or shorts.
    TexCoord,
}

/// The accessor's components as f64, `width` per element, normalized integers scaled to
/// `0..=1`.
pub(super) fn floats(
    acc: &Accessor<'_>,
    buffers: &[Cow<'_, [u8]>],
    used: Use,
) -> Result<Vec<f64>, String> {
    let (dims, allowed) = match used {
        Use::Position => (Dimensions::Vec3, &[DataType::F32][..]),
        Use::TexCoord => (
            Dimensions::Vec2,
            &[DataType::F32, DataType::U8, DataType::U16][..],
        ),
    };
    let ty = acc.data_type();
    let floats_ok = ty == DataType::F32 && !acc.normalized();
    let ints_ok = ty != DataType::F32 && acc.normalized();
    if acc.dimensions() != dims || !allowed.contains(&ty) || !(floats_ok || ints_ok) {
        return Err(format!(
            "accessor {} has a type its attribute does not allow",
            acc.index()
        ));
    }
    let scale = match ty {
        DataType::U8 => f64::from(u8::MAX),
        DataType::U16 => f64::from(u16::MAX),
        _ => 1.0,
    };
    Ok(components(acc, buffers)?
        .into_iter()
        .map(|c| c / scale)
        .collect())
}

/// The accessor's unsigned integer indices.
pub(super) fn indices(acc: &Accessor<'_>, buffers: &[Cow<'_, [u8]>]) -> Result<Vec<u32>, String> {
    let unsigned = matches!(
        acc.data_type(),
        DataType::U8 | DataType::U16 | DataType::U32
    );
    if acc.dimensions() != Dimensions::Scalar || !unsigned || acc.normalized() {
        return Err(format!(
            "accessor {} is not unsigned integer indices",
            acc.index()
        ));
    }
    // Lossless: every component is an unsigned integer of at most 32 bits.
    Ok(components(acc, buffers)?
        .into_iter()
        .map(|c| c as u32)
        .collect())
}

/// Every component of every element, as f64, in element order.
fn components(acc: &Accessor<'_>, buffers: &[Cow<'_, [u8]>]) -> Result<Vec<f64>, String> {
    let index = acc.index();
    if acc.sparse().is_some() {
        return Err(format!(
            "accessor {index} is sparse, which the reader does not read"
        ));
    }
    let width = acc.dimensions().multiplicity();
    let count = acc.count();
    // With no view and no sparse data an accessor is all zeros, which no geometry needs.
    let view = acc
        .view()
        .ok_or_else(|| format!("accessor {index} has no buffer view"))?;
    let size = acc.data_type().size();
    let element = size * width;
    let stride = view.stride().unwrap_or(element);
    let past_view = || format!("accessor {index} runs past the end of its buffer view");
    let bytes = buffers
        .get(view.buffer().index())
        .and_then(|b| b.get(view.offset()..view.offset().checked_add(view.length())?))
        .ok_or_else(|| {
            format!(
                "buffer view {} runs past the end of its buffer",
                view.index()
            )
        })?;
    if count > 0 {
        let last = (count - 1)
            .checked_mul(stride)
            .and_then(|s| s.checked_add(acc.offset()))
            .and_then(|s| s.checked_add(element))
            .ok_or_else(past_view)?;
        if last > bytes.len() {
            return Err(past_view());
        }
    }
    let ty = acc.data_type();
    let mut out = Vec::with_capacity(count * width);
    for i in 0..count {
        let start = acc.offset() + i * stride;
        // INVARIANT: the last element ends within `bytes` (checked above), so every one does.
        let at = &bytes[start..start + element];
        out.extend(at.chunks_exact(size).map(|c| decode(ty, c)));
    }
    Ok(out)
}

fn decode(ty: DataType, c: &[u8]) -> f64 {
    let mut b = [0u8; 4];
    b[..c.len()].copy_from_slice(c);
    match ty {
        DataType::F32 => f64::from(f32::from_le_bytes(b)),
        DataType::U8 => f64::from(b[0]),
        DataType::I8 => f64::from(i8::from_le_bytes([b[0]])),
        DataType::U16 => f64::from(u16::from_le_bytes([b[0], b[1]])),
        DataType::I16 => f64::from(i16::from_le_bytes([b[0], b[1]])),
        DataType::U32 => f64::from(u32::from_le_bytes(b)),
    }
}
