//! Same assertions natively (`cargo test`) and on wasm32 (`cargo test --target
//! wasm32-unknown-unknown`, through wasm-bindgen-test-runner under Node).
//! The expected values were produced by the native release binary; a wasm run
//! that passes therefore produced byte-identical buffers.

use baby_shark_wasm::{
    Fingerprint, decimate, decimate_to_floor, remesh, voxel, voxel_feature_preserving,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const REMESH: Fingerprint = Fingerprint {
    vertices: 4548,
    faces: 9092,
    hash: 933930466446281728,
};
const DECIMATE: Fingerprint = Fingerprint {
    vertices: 104,
    faces: 204,
    hash: 7995362684396460200,
};
const FLOOR: Fingerprint = Fingerprint {
    vertices: 3,
    faces: 2,
    hash: 9774961354207999086,
};
const VOXEL: Fingerprint = Fingerprint {
    vertices: 44880,
    faces: 14960,
    hash: 7896778119327624079,
};
const VOXEL_FP_ONE_THREAD: Fingerprint = Fingerprint {
    vertices: 44784,
    faces: 14928,
    hash: 3411817374792925112,
};

#[test]
fn isotropic_remesh_runs_and_matches_native() {
    assert_eq!(remesh(0.05), REMESH);
    assert_eq!(
        remesh(0.05),
        REMESH,
        "second run in the same process differs"
    );
}

#[test]
fn decimation_runs_and_matches_native() {
    assert_eq!(decimate(1e-1), DECIMATE);
}

/// Documents a library defect: the face floor of 320 is not honoured.
#[test]
fn decimation_floor_is_ignored() {
    assert_eq!(decimate_to_floor(320), FLOOR);
}

/// The voxel path is the only one that calls into rayon.
#[test]
fn voxel_remesh_runs_and_matches_native() {
    assert_eq!(voxel(0.05), Ok(VOXEL));
}

/// Dual contouring appends faces in thread-completion order, so only a
/// one-thread run (`RAYON_NUM_THREADS=1` natively; always on wasm32) is stable.
#[test]
fn voxel_feature_preserving_matches_single_thread_native() {
    let fp = voxel_feature_preserving(0.05);
    if cfg!(target_arch = "wasm32") || std::env::var("RAYON_NUM_THREADS").as_deref() == Ok("1") {
        assert_eq!(fp, Ok(VOXEL_FP_ONE_THREAD));
    } else {
        let fp = fp.map(|f| (f.vertices, f.faces));
        assert_eq!(
            fp,
            Ok((VOXEL_FP_ONE_THREAD.vertices, VOXEL_FP_ONE_THREAD.faces))
        );
    }
}
