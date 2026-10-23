//! Spike 001 T001: run baby_shark's preprocessing operations on a closed icosphere and
//! fingerprint the output, so native and wasm32 runs can be compared bit for bit.

use std::collections::BTreeMap;

use baby_shark::decimation::{AlwaysDecimate, ConstantErrorDecimationCriteria, EdgeDecimator};
use baby_shark::exports::nalgebra::Vector3;
use baby_shark::mesh::corner_table::CornerTable;
use baby_shark::mesh::polygon_soup::data_structure::PolygonSoup;
use baby_shark::mesh::traits::TriangleMesh;
use baby_shark::remeshing::incremental::IncrementalRemesher;
use baby_shark::remeshing::voxel::{MeshingMethod, VoxelRemesher};

/// Subdivision level 3: 642 vertices, 1280 faces, mean edge ≈ 0.095 on the unit sphere.
pub const SUBDIVISIONS: u32 = 3;

/// Vertex count, face count, and an FNV-1a hash of the compacted buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    pub vertices: usize,
    pub faces: usize,
    pub hash: u64,
}

/// Unit icosphere: positions and counter-clockwise (outward) triangles.
pub fn icosphere(subdivisions: u32) -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
    let t = (1.0 + 5f64.sqrt()) / 2.0;
    let mut v: Vec<[f64; 3]> = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ]
    .iter()
    .map(|p| normalize(*p))
    .collect();
    let mut f: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..subdivisions {
        let mut mid: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        let mut next = Vec::with_capacity(f.len() * 4);
        for [a, b, c] in f {
            let ab = midpoint(&mut v, &mut mid, a, b);
            let bc = midpoint(&mut v, &mut mid, b, c);
            let ca = midpoint(&mut v, &mut mid, c, a);
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        f = next;
    }
    (v, f)
}

fn normalize([x, y, z]: [f64; 3]) -> [f64; 3] {
    let n = (x * x + y * y + z * z).sqrt();
    [x / n, y / n, z / n]
}

fn midpoint(
    v: &mut Vec<[f64; 3]>,
    mid: &mut BTreeMap<(usize, usize), usize>,
    a: usize,
    b: usize,
) -> usize {
    let key = (a.min(b), a.max(b));
    *mid.entry(key).or_insert_with(|| {
        let (p, q) = (v[a], v[b]);
        v.push(normalize([
            (p[0] + q[0]) / 2.0,
            (p[1] + q[1]) / 2.0,
            (p[2] + q[2]) / 2.0,
        ]));
        v.len() - 1
    })
}

fn corner_table(subdivisions: u32) -> CornerTable<f64> {
    let (v, f) = icosphere(subdivisions);
    let v: Vec<Vector3<f64>> = v.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect();
    let f: Vec<usize> = f.iter().flatten().copied().collect();
    CornerTable::from_vertex_and_face_slices(&v, &f)
}

/// Incremental isotropic remesh to `target_edge` (model units), library defaults otherwise.
pub fn remesh(target_edge: f64) -> Fingerprint {
    let mut mesh = corner_table(SUBDIVISIONS);
    IncrementalRemesher::new()
        .with_iterations_count(10)
        .remesh(&mut mesh, target_edge);
    fingerprint(&mesh)
}

/// Quadric edge-collapse decimation down to a maximum quadric error.
pub fn decimate(max_error: f64) -> Fingerprint {
    let mut mesh = corner_table(SUBDIVISIONS);
    let mut decimator =
        EdgeDecimator::new().decimation_criteria(ConstantErrorDecimationCriteria::new(max_error));
    decimator.decimate(&mut mesh);
    fingerprint(&mesh)
}

/// Unconditional decimation with a face floor; the floor is what's under test.
pub fn decimate_to_floor(min_faces: usize) -> Fingerprint {
    let mut mesh = corner_table(SUBDIVISIONS);
    EdgeDecimator::<f64, AlwaysDecimate>::new()
        .min_faces_count(Some(min_faces))
        .decimate(&mut mesh);
    fingerprint(&mesh)
}

/// Voxel remesh (f32 only in baby_shark) with the manifold marching-cubes mesher.
pub fn voxel(voxel_size: f32) -> Result<Fingerprint, String> {
    voxel_with(voxel_size, MeshingMethod::Manifold)
}

/// Voxel remesh with the feature-preserving dual-contouring mesher.
pub fn voxel_feature_preserving(voxel_size: f32) -> Result<Fingerprint, String> {
    voxel_with(voxel_size, MeshingMethod::FeaturePreserving)
}

fn voxel_with(voxel_size: f32, method: MeshingMethod) -> Result<Fingerprint, String> {
    let (v, f) = icosphere(SUBDIVISIONS);
    let v: Vec<Vector3<f32>> = v
        .iter()
        .map(|p| Vector3::new(p[0] as f32, p[1] as f32, p[2] as f32))
        .collect();
    let f: Vec<usize> = f.iter().flatten().copied().collect();
    let soup = PolygonSoup::from_vertex_and_face_slices(&v, &f);
    let out = VoxelRemesher::default()
        .with_voxel_size(voxel_size)
        .with_meshing_method(method)
        .remesh(&soup)
        .ok_or_else(|| "VoxelRemesher::remesh returned None".to_string())?;
    Ok(fingerprint(&out))
}

/// Hash vertex positions (as bit patterns) in iteration order, then faces as compacted indices.
fn fingerprint<M: TriangleMesh>(mesh: &M) -> Fingerprint
where
    M::Scalar: Bits,
    M::VertexId: Copy,
{
    let mut h = Fnv::new();
    let mut index = BTreeMap::new();
    for v in mesh.vertices() {
        let n = index.len();
        for c in mesh.position(v) {
            h.write(c.bits());
        }
        index.insert(v, n as u64);
    }
    let mut faces = 0;
    for tri in mesh.faces() {
        for v in tri {
            // INVARIANT: every face vertex is listed by vertices(); a miss is a library bug.
            #[allow(clippy::expect_used)]
            h.write(
                *index
                    .get(&v)
                    .expect("face references a vertex not in vertices()"),
            );
        }
        faces += 1;
    }
    Fingerprint {
        vertices: index.len(),
        faces,
        hash: h.0,
    }
}

pub trait Bits {
    fn bits(self) -> u64;
}
impl Bits for f64 {
    fn bits(self) -> u64 {
        self.to_bits()
    }
}
impl Bits for f32 {
    fn bits(self) -> u64 {
        u64::from(self.to_bits())
    }
}

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn write(&mut self, word: u64) {
        for byte in word.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}
