//! Every reader refuses a damaged file with an error and never panics: valid files of each
//! format, mutated from a fixed seed, are read and whatever comes back is accepted.

use openplush_io::{fbx, gltf, obj, ply, stl};
use openplush_testkit::fbx::{Scene, material, mesh, model};

type Setup = Box<dyn std::error::Error>;

/// xorshift64: deterministic, so a failing case reproduces from its number.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// `base` with one to five bit flips, byte writes, truncations, 0xFF runs, edge-case u32
/// writes or duplicated chunks.
fn mutate(base: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut b = base.to_vec();
    for _ in 0..=rng.below(4) {
        if b.is_empty() {
            break;
        }
        let at = rng.below(b.len());
        let end = (at + 4).min(b.len());
        match rng.below(6) {
            0 => b[at] ^= 1 << rng.below(8),
            1 => b[at] = rng.next() as u8,
            2 => b.truncate(at),
            3 => b[at..end].fill(0xff),
            4 => {
                let v = [0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff][rng.below(5)];
                b[at..end].copy_from_slice(&v.to_le_bytes()[..end - at]);
            }
            _ => {
                let len = rng.below(64).min(b.len() - at);
                let chunk = b[at..at + len].to_vec();
                b.splice(at..at, chunk);
            }
        }
    }
    b
}

/// Reads `cases` mutations of `base`; a reader that panics fails the test.
fn survive(seed: u64, base: &[u8], cases: u32, read: impl Fn(&[u8])) {
    let mut rng = Rng(seed);
    for _ in 0..cases {
        read(&mutate(base, &mut rng));
    }
}

fn fbx_cube() -> Result<Vec<u8>, Setup> {
    let corners = (0..8u32)
        .flat_map(|i| [i & 1, i >> 1 & 1, i >> 2 & 1].map(f64::from))
        .collect();
    let quads = [
        [0, 2, 3, 1],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 4, 6, 2],
        [1, 3, 7, 5],
    ];
    let polygons = quads
        .iter()
        .flat_map(|&[a, b, c, d]| [a, b, c, -d - 1])
        .collect();
    let mut scene = Scene::default();
    scene
        .object(model(1, "Cube", Vec::new()))
        .object(mesh(2, "Cube", corners, polygons))
        .object(material(3, "Felt"))
        .connect(1, 0)
        .connect(2, 1)
        .connect(3, 1);
    Ok(scene.bytes()?)
}

/// Two triangles as binary STL.
fn binary_stl() -> Vec<u8> {
    let mut b = vec![0u8; 80];
    b.extend_from_slice(&2u32.to_le_bytes());
    for tri in [
        [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
        [[0., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    ] {
        b.extend_from_slice(&[0u8; 12]);
        b.extend(tri.iter().flatten().flat_map(|c: &f32| c.to_le_bytes()));
        b.extend_from_slice(&[0u8; 2]);
    }
    b
}

const PLY: &str = "ply\nformat ascii 1.0\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nelement face 2\nproperty list uchar int vertex_indices\nend_header\n0 0 0\n1 0 0\n1 1 0\n0 1 0\n3 0 1 2\n4 0 1 2 3\n";

const OBJ: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/meshes/capsule.obj"
));

const GLB: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/meshes/asymmetric.glb"
));

#[test]
fn a_mutated_fbx_never_panics() -> Result<(), Setup> {
    survive(0x9e37_79b9_7f4a_7c15, &fbx_cube()?, 4_000, |b| {
        let _ = fbx::read(b);
    });
    Ok(())
}

#[test]
fn a_mutated_glb_never_panics() {
    survive(0x2545_f491_4f6c_dd1d, GLB, 3_000, |b| {
        let _ = gltf::read("model.glb", b, &|_| None);
    });
}

#[test]
fn a_mutated_obj_never_panics() {
    survive(0xd1b5_4a32_d192_ed03, OBJ, 2_000, |b| {
        let _ = obj::read(b);
    });
}

#[test]
fn a_mutated_stl_never_panics() {
    survive(0x8cb9_2ba7_2f3d_8dd7, &binary_stl(), 4_000, |b| {
        let _ = stl::read(b);
    });
}

#[test]
fn a_mutated_ply_never_panics() {
    survive(0xa076_1d64_78bd_642f, PLY.as_bytes(), 4_000, |b| {
        let _ = ply::read(b);
    });
}
