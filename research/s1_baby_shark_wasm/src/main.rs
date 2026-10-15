//! Native run: prints each operation's fingerprint and wall time.

use std::time::Instant;

use s1_baby_shark_wasm::{
    SUBDIVISIONS, decimate, decimate_to_floor, icosphere, remesh, voxel, voxel_feature_preserving,
};

fn main() {
    let (v, f) = icosphere(SUBDIVISIONS);
    println!("input icosphere: {} vertices, {} faces", v.len(), f.len());
    let t = Instant::now();
    println!(
        "remesh(0.05)            {:?}  {:?}",
        remesh(0.05),
        t.elapsed()
    );
    for e in [1e-1, 1e-2] {
        let t = Instant::now();
        println!(
            "decimate({e:e})           {:?}  {:?}",
            decimate(e),
            t.elapsed()
        );
    }
    let t = Instant::now();
    println!(
        "decimate_to_floor(320)  {:?}  {:?}",
        decimate_to_floor(320),
        t.elapsed()
    );
    let t = Instant::now();
    println!(
        "voxel(0.05)             {:?}  {:?}",
        voxel(0.05),
        t.elapsed()
    );
    let t = Instant::now();
    println!(
        "voxel_fp(0.05)          {:?}  {:?}",
        voxel_feature_preserving(0.05),
        t.elapsed()
    );
}
