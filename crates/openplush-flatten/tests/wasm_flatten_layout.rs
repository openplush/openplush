//! The same chart flattens to the same layout, bit for bit, natively and on wasm32
//! (Constitution V): the boundary, the harmonic start and every SLIM step.
//!
//! The pinned hash was computed natively; the wasm32 run must reproduce it. It changes only when
//! the flattener does, and then a person decides whether the new layout is right.

use std::f64::consts::{FRAC_PI_2, TAU};

use openplush_flatten::{FlattenError, FlattenSession, Progress};
use openplush_geom::DVec3;
use openplush_mesh::{FaceId, VertexId};
use openplush_seams::ChartId;

type Faces = Vec<(FaceId, [VertexId; 3])>;

/// A hemisphere of radius 40 mm as a `rings` by `24` grid of quads from the rim to near the pole,
/// cut open along one meridian so it is a disk. Built with `libm`, so both targets get the same
/// input bits.
fn hemisphere(rings: u32) -> (Vec<DVec3>, Faces) {
    let cols = 24;
    let xyz = (0..=rings)
        .flat_map(|r| (0..=cols).map(move |c| (c, r)))
        .map(|(c, r)| {
            let polar = FRAC_PI_2 * (1.0 - 0.9 * f64::from(r) / f64::from(rings));
            let around = TAU * 0.95 * f64::from(c) / f64::from(cols);
            let s = 40.0 * libm::sin(polar);
            DVec3::new(
                s * libm::cos(around),
                s * libm::sin(around),
                40.0 * libm::cos(polar),
            )
        })
        .collect();
    let id = |c: u32, r: u32| r * (cols + 1) + c;
    let faces = (0..rings)
        .flat_map(|r| (0..cols).map(move |c| (c, r)))
        .flat_map(|(c, r)| {
            let (a, b, d, e) = (id(c, r), id(c + 1, r), id(c + 1, r + 1), id(c, r + 1));
            [[a, b, d], [a, d, e]]
        })
        .zip(0..)
        .map(|(t, f)| (FaceId::new(f), t.map(VertexId::new)))
        .collect();
    (xyz, faces)
}

/// 64-bit FNV-1a, fed one word at a time.
fn fnv(hash: u64, word: u64) -> u64 {
    word.to_le_bytes().iter().fold(hash, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Steps the session to `Done`, hashing every reported energy and then the layout's bits.
fn flatten_hash() -> Result<u64, FlattenError> {
    let (xyz, faces) = hemisphere(8);
    let mut session = FlattenSession::new(ChartId::new(0), &xyz, faces)?;
    let mut hash = 0xcbf2_9ce4_8422_2325;
    while let Progress::Working { iteration, energy } = session.step()? {
        hash = fnv(fnv(hash, u64::from(iteration)), energy.to_bits());
    }
    let layout = session.finish().unwrap_or_default();
    Ok(layout.iter().fold(hash, |h, (v, p)| {
        fnv(
            fnv(fnv(h, u64::from(v.index())), p.x.to_bits()),
            p.y.to_bits(),
        )
    }))
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn the_layout_is_the_same_on_every_target() {
    assert_eq!(flatten_hash(), Ok(0xbb78_bd1f_610e_e867));
}
