//! glTF and GLB: the default scene drawn with its node transforms, triangles only, buffers
//! from the file itself or the files beside it, and what was not read listed.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use openplush_io::gltf::{self, Ignored, Imported};
use openplush_io::{Format, IoError};
use openplush_mesh::{FaceId, MaterialIndex, SourceCorner, TriangleSoup, VertexId};

type Setup = Box<dyn std::error::Error>;

/// The binary data of one buffer, with the views and accessors that describe it.
#[derive(Default)]
struct Bin {
    bytes: Vec<u8>,
    views: Vec<String>,
    accessors: Vec<String>,
}

impl Bin {
    /// Appends `data` as a view, 4-byte aligned, and an accessor over it; returns the
    /// accessor's index.
    fn accessor(
        &mut self,
        data: &[u8],
        (component, kind): (u32, &str),
        count: usize,
        extra: &str,
    ) -> usize {
        while !self.bytes.len().is_multiple_of(4) {
            self.bytes.push(0);
        }
        let view = self.views.len();
        self.views.push(format!(
            r#"{{"buffer":0,"byteOffset":{},"byteLength":{}}}"#,
            self.bytes.len(),
            data.len()
        ));
        self.bytes.extend_from_slice(data);
        self.accessors.push(format!(
            r#"{{"bufferView":{view},"componentType":{component},"type":"{kind}","count":{count}{extra}}}"#
        ));
        self.accessors.len() - 1
    }

    fn positions(&mut self, points: &[[f32; 3]]) -> usize {
        let data: Vec<u8> = points
            .iter()
            .flatten()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let (lo, hi) = bounds(points);
        let extra = format!(r#","min":{lo:?},"max":{hi:?}"#);
        self.accessor(&data, (5126, "VEC3"), points.len(), &extra)
    }

    fn indices(&mut self, indices: &[u16]) -> usize {
        let data: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        self.accessor(&data, (5123, "SCALAR"), indices.len(), "")
    }

    fn uvs(&mut self, uvs: &[[f32; 2]]) -> usize {
        let data: Vec<u8> = uvs.iter().flatten().flat_map(|c| c.to_le_bytes()).collect();
        self.accessor(&data, (5126, "VEC2"), uvs.len(), "")
    }

    fn data_uri(&self) -> String {
        format!(
            "data:application/octet-stream;base64,{}",
            STANDARD.encode(&self.bytes)
        )
    }

    /// The document, its one buffer at `uri` (`None`: GLB's chunk) of `length` bytes.
    fn document(&self, rest: &str, uri: Option<&str>, length: usize) -> String {
        let uri = uri.map(|u| format!(r#""uri":"{u}","#)).unwrap_or_default();
        format!(
            r#"{{"asset":{{"version":"2.0"}},"buffers":[{{{uri}"byteLength":{length}}}],"bufferViews":[{}],"accessors":[{}],{rest}}}"#,
            self.views.join(","),
            self.accessors.join(",")
        )
    }

    fn gltf(&self, rest: &str) -> String {
        self.document(rest, Some(&self.data_uri()), self.bytes.len())
    }
}

fn bounds(points: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in points {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

const TRIANGLE: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];

/// One indexed triangle as mesh 0, drawn by the nodes and scenes in `rest`.
fn triangle(rest: &str) -> String {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let i = bin.indices(&[0, 1, 2]);
    let meshes = format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}},"indices":{i}}}]}}]"#
    );
    bin.gltf(&format!("{meshes},{rest}"))
}

fn none(_: &str) -> Option<&'static [u8]> {
    None
}

fn read(text: &str) -> Result<Imported, IoError> {
    gltf::read("model.gltf", text.as_bytes(), &none)
}

fn parse_detail(r: Result<Imported, IoError>) -> Result<String, Setup> {
    match r {
        Err(IoError::Parse {
            format: Format::Gltf,
            detail,
        }) => Ok(detail),
        other => Err(format!("expected a glTF Parse, got {other:?}").into()),
    }
}

fn corner(primitive: u32, vertex: u32) -> SourceCorner {
    SourceCorner { primitive, vertex }
}

fn triangles(soup: &TriangleSoup) -> Result<Vec<[VertexId; 3]>, Setup> {
    (0..u32::try_from(soup.triangle_count())?)
        .map(|f| {
            soup.triangle(FaceId::new(f))
                .ok_or_else(|| "a triangle is missing".into())
        })
        .collect()
}

fn position(soup: &TriangleSoup, v: u32) -> Result<[f64; 3], Setup> {
    let p = soup
        .position(VertexId::new(v))
        .ok_or("a vertex is missing")?;
    Ok([p.x, p.y, p.z])
}

#[test]
fn the_default_scene_is_drawn_with_its_node_transform() -> Result<(), Setup> {
    let doc = triangle(
        r#""nodes":[{"mesh":0,"translation":[10,0,0]}],"scenes":[{"nodes":[0]}],"scene":0"#,
    );
    let imported = read(&doc)?;
    let soup = &imported.soup;
    assert_eq!(soup.vertex_count(), 3);
    assert_eq!(position(soup, 1)?, [11.0, 0.0, 0.0]);
    assert_eq!(soup.corner_source(FaceId::new(0), 2), Some(corner(0, 2)));
    assert!(imported.ignored.is_empty());
    Ok(())
}

#[test]
fn without_a_default_scene_scene_zero_is_drawn() -> Result<(), Setup> {
    let doc = triangle(r#""nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}]"#);
    assert_eq!(read(&doc)?.soup.triangle_count(), 1);
    Ok(())
}

#[test]
fn a_file_with_no_scene_is_refused() -> Result<(), Setup> {
    let detail = parse_detail(read(&triangle(r#""nodes":[{"mesh":0}]"#)))?;
    assert!(detail.contains("no scene"), "{detail}");
    Ok(())
}

#[test]
fn children_compose_their_parents_transforms() -> Result<(), Setup> {
    // Parent scales by 2, child translates by (0, 0, 5) in the parent's frame.
    let doc = triangle(
        r#""nodes":[{"scale":[2,2,2],"children":[1]},{"mesh":0,"translation":[0,0,5]}],"scenes":[{"nodes":[0]}],"scene":0"#,
    );
    let soup = read(&doc)?.soup;
    assert_eq!(position(&soup, 1)?, [2.0, 0.0, 10.0]);
    Ok(())
}

#[test]
fn a_matrix_transform_and_a_rotation_are_applied() -> Result<(), Setup> {
    // A quarter turn about z: (1, 0, 0) goes to (0, 1, 0).
    let half = std::f64::consts::FRAC_1_SQRT_2;
    let rotated = triangle(&format!(
        r#""nodes":[{{"mesh":0,"rotation":[0,0,{half},{half}]}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    let p = position(&read(&rotated)?.soup, 1)?;
    assert!(
        p[0].abs() < 1e-6 && (p[1] - 1.0).abs() < 1e-6 && p[2].abs() < 1e-6,
        "{p:?}"
    );
    let matrix = triangle(
        r#""nodes":[{"mesh":0,"matrix":[1,0,0,0, 0,1,0,0, 0,0,1,0, 3,4,5,1]}],"scenes":[{"nodes":[0]}],"scene":0"#,
    );
    assert_eq!(position(&read(&matrix)?.soup, 0)?, [3.0, 4.0, 5.0]);
    Ok(())
}

#[test]
fn a_mesh_drawn_by_two_nodes_is_two_primitives() -> Result<(), Setup> {
    let doc = triangle(
        r#""nodes":[{"mesh":0},{"mesh":0,"translation":[0,0,1]}],"scenes":[{"nodes":[0,1]}],"scene":0"#,
    );
    let soup = read(&doc)?.soup;
    assert_eq!(soup.vertex_count(), 6);
    assert_eq!(soup.triangle_count(), 2);
    assert_eq!(soup.corner_source(FaceId::new(1), 0), Some(corner(1, 0)));
    assert_eq!(position(&soup, 3)?, [0.0, 0.0, 1.0]);
    Ok(())
}

#[test]
fn a_mirroring_transform_reverses_the_winding() -> Result<(), Setup> {
    let plain = triangle(r#""nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0"#);
    let mirrored =
        triangle(r#""nodes":[{"mesh":0,"scale":[-1,1,1]}],"scenes":[{"nodes":[0]}],"scene":0"#);
    let [a, b, c] = triangles(&read(&plain)?.soup)?[0];
    assert_eq!(triangles(&read(&mirrored)?.soup)?, vec![[a, c, b]]);
    Ok(())
}

fn one_primitive(mode: u32, points: &[[f32; 3]], indexed: bool) -> String {
    let mut bin = Bin::default();
    let p = bin.positions(points);
    let indices = if indexed {
        let all: Vec<u16> = (0..u16::try_from(points.len()).unwrap_or(0)).collect();
        format!(r#","indices":{}"#, bin.indices(&all))
    } else {
        String::new()
    };
    bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}},"mode":{mode}{indices}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ))
}

const QUAD: [[f32; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [1.0, 1.0, 0.0],
];

fn ids(soup: &TriangleSoup) -> Result<Vec<[u32; 3]>, Setup> {
    Ok(triangles(soup)?
        .into_iter()
        .map(|t| t.map(VertexId::index))
        .collect())
}

#[test]
fn a_strip_alternates_its_winding_and_a_fan_turns_about_its_first_vertex() -> Result<(), Setup> {
    assert_eq!(
        ids(&read(&one_primitive(5, &QUAD, true))?.soup)?,
        vec![[0, 1, 2], [1, 3, 2]]
    );
    assert_eq!(
        ids(&read(&one_primitive(6, &QUAD, true))?.soup)?,
        vec![[0, 1, 2], [0, 2, 3]]
    );
    Ok(())
}

#[test]
fn a_primitive_without_indices_reads_its_vertices_in_order() -> Result<(), Setup> {
    assert_eq!(
        ids(&read(&one_primitive(4, &TRIANGLE, false))?.soup)?,
        vec![[0, 1, 2]]
    );
    Ok(())
}

#[test]
fn a_triangle_list_with_a_partial_triangle_is_refused() -> Result<(), Setup> {
    let detail = parse_detail(read(&one_primitive(4, &QUAD, true)))?;
    assert!(detail.contains("multiple of 3"), "{detail}");
    Ok(())
}

#[test]
fn points_and_lines_are_skipped_and_listed() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let doc = bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}},"mode":0}},{{"attributes":{{"POSITION":{p}}},"mode":4}},{{"attributes":{{"POSITION":{p}}},"mode":1}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    let imported = read(&doc)?;
    assert_eq!(imported.soup.triangle_count(), 1);
    assert_eq!(
        imported.soup.corner_source(FaceId::new(0), 0),
        Some(corner(0, 0))
    );
    assert_eq!(
        imported.ignored,
        vec![
            Ignored::Primitive {
                node: 0,
                mesh: 0,
                primitive: 0
            },
            Ignored::Primitive {
                node: 0,
                mesh: 0,
                primitive: 2
            },
        ]
    );
    Ok(())
}

#[test]
fn an_index_past_the_last_vertex_is_refused() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let i = bin.indices(&[0, 1, 3]);
    let doc = bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}},"indices":{i}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    let detail = parse_detail(read(&doc))?;
    assert!(detail.contains("index 3"), "{detail}");
    Ok(())
}

#[test]
fn corner_uvs_come_from_the_first_texture_set_and_further_sets_are_counted() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let t0 = bin.uvs(&[[0.0, 0.0], [0.5, 0.0], [0.0, 0.25]]);
    let t1 = bin.uvs(&[[1.0, 1.0]; 3]);
    let doc = bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p},"TEXCOORD_0":{t0},"TEXCOORD_1":{t1}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    let soup = read(&doc)?.soup;
    let uv = soup.corner_uv(FaceId::new(0), 1).ok_or("no uv")?;
    assert_eq!([uv.x, uv.y], [0.5, 0.0]);
    assert_eq!(soup.extra_uv_sets(), 1);
    Ok(())
}

#[test]
fn normalized_byte_uvs_are_scaled_to_one() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let t = bin.accessor(
        &[0, 0, 255, 0, 0, 51],
        (5121, "VEC2"),
        3,
        r#","normalized":true"#,
    );
    let doc = bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p},"TEXCOORD_0":{t}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    let soup = read(&doc)?.soup;
    let uv = soup.corner_uv(FaceId::new(0), 2).ok_or("no uv")?;
    assert_eq!([uv.x, uv.y], [0.0, 0.2]);
    Ok(())
}

#[test]
fn each_primitive_keeps_its_material() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let doc = bin.gltf(&format!(
        r#""materials":[{{"name":"fur"}},{{}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}},"material":1}},{{"attributes":{{"POSITION":{p}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    let soup = read(&doc)?.soup;
    let materials = soup.materials();
    assert_eq!(materials.names(), ["fur", "material 1"]);
    assert_eq!(materials.material(0), Some(MaterialIndex::new(1)));
    assert_eq!(materials.material(1), None);
    Ok(())
}

#[test]
fn skins_morph_targets_and_animations_are_listed() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let d = bin.positions(&[[0.0; 3]; 3]);
    let times = bin.accessor(
        &0f32.to_le_bytes(),
        (5126, "SCALAR"),
        1,
        r#","min":[0],"max":[0]"#,
    );
    let moves = bin.accessor(&[0u8; 12], (5126, "VEC3"), 1, "");
    let joints = bin.accessor(&[0u8; 64], (5126, "MAT4"), 1, "");
    let doc = bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}},"targets":[{{"POSITION":{d}}}]}}]}}],"nodes":[{{"mesh":0,"skin":0}},{{}}],"skins":[{{"joints":[1],"inverseBindMatrices":{joints}}}],"animations":[{{"channels":[{{"sampler":0,"target":{{"node":1,"path":"translation"}}}}],"samplers":[{{"input":{times},"output":{moves}}}]}}],"scenes":[{{"nodes":[0,1]}}],"scene":0"#
    ));
    let imported = read(&doc)?;
    assert_eq!(imported.soup.triangle_count(), 1);
    assert_eq!(
        imported.ignored,
        vec![
            Ignored::Skin { node: 0 },
            Ignored::MorphTargets { node: 0, mesh: 0 },
            Ignored::Animation { animation: 0 },
        ]
    );
    Ok(())
}

#[test]
fn an_unimplemented_required_extension_is_refused_by_name() -> Result<(), Setup> {
    let doc = triangle(
        r#""extensionsRequired":["KHR_draco_mesh_compression"],"extensionsUsed":["KHR_draco_mesh_compression"],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0"#,
    );
    let detail = parse_detail(read(&doc))?;
    assert!(detail.contains("KHR_draco_mesh_compression"), "{detail}");
    Ok(())
}

#[test]
fn a_sparse_accessor_is_refused() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let base = bin.positions(&TRIANGLE);
    let idx = bin.views.len();
    bin.indices(&[1]);
    let val = bin.views.len();
    bin.positions(&[[5.0, 5.0, 5.0]]);
    let sparse = format!(
        r#","sparse":{{"count":1,"indices":{{"bufferView":{idx},"componentType":5123}},"values":{{"bufferView":{val}}}}}"#
    );
    let accessor = bin.accessors[base].trim_end_matches('}').to_owned() + &sparse + "}";
    bin.accessors[base] = accessor;
    let doc = bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{base}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    let detail = parse_detail(read(&doc))?;
    assert!(detail.contains("sparse"), "{detail}");
    Ok(())
}

#[test]
fn a_non_finite_position_names_its_source_vertex() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.accessor(
        &[0.0f32, 0.0, 0.0, f32::NAN, 0.0, 0.0, 0.0, 1.0, 0.0]
            .iter()
            .flat_map(|c| c.to_le_bytes())
            .collect::<Vec<u8>>(),
        (5126, "VEC3"),
        3,
        r#","min":[0,0,0],"max":[1,1,0]"#,
    );
    let doc = bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ));
    assert_eq!(
        read(&doc).err(),
        Some(IoError::NonFiniteInput {
            vertices: vec![corner(0, 1)]
        })
    );
    Ok(())
}

#[test]
fn a_buffer_shorter_than_it_says_is_refused() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let rest = format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    );
    let doc = bin.document(&rest, Some(&bin.data_uri()), bin.bytes.len() + 4);
    let detail = parse_detail(read(&doc))?;
    assert!(detail.contains("buffer 0"), "{detail}");
    Ok(())
}

/// A triangle whose one buffer lives at `uri`, beside the file.
fn beside(uri: &str) -> (String, Vec<u8>) {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let rest = format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    );
    (bin.document(&rest, Some(uri), bin.bytes.len()), bin.bytes)
}

#[test]
fn a_buffer_beside_the_file_is_looked_up_relative_to_it() -> Result<(), Setup> {
    for (primary, uri, wanted) in [
        ("model.gltf", "model.bin", "model.bin"),
        ("scans/model.gltf", "model.bin", "scans/model.bin"),
        ("scans/model.gltf", "parts/../model.bin", "scans/model.bin"),
        ("scans/model.gltf", "./my%20model.bin", "scans/my model.bin"),
        ("a/b/model.gltf", "../model.bin", "a/model.bin"),
    ] {
        let (doc, bytes) = beside(uri);
        let asked = std::cell::RefCell::new(Vec::new());
        let lookup = |name: &str| {
            asked.borrow_mut().push(name.to_owned());
            (name == wanted).then_some(bytes.as_slice())
        };
        let imported = gltf::read(primary, doc.as_bytes(), &lookup)?;
        assert_eq!(imported.soup.triangle_count(), 1, "{uri}");
        assert_eq!(*asked.borrow(), vec![wanted.to_owned()], "{uri}");
    }
    Ok(())
}

#[test]
fn a_buffer_not_supplied_is_a_missing_resource() {
    let (doc, _) = beside("model.bin");
    assert_eq!(
        read(&doc).err(),
        Some(IoError::MissingResource {
            uri: "model.bin".into()
        })
    );
}

#[test]
fn a_reference_outside_the_supplied_files_is_refused() -> Result<(), Setup> {
    for uri in [
        "../model.bin",
        "/model.bin",
        "https://example.com/model.bin",
        "file:///model.bin",
        "a/../../model.bin",
        "bad%zzname.bin",
    ] {
        let (doc, bytes) = beside(uri);
        let lookup = |_: &str| Some(bytes.as_slice());
        let detail = parse_detail(gltf::read("model.gltf", doc.as_bytes(), &lookup))?;
        assert!(detail.contains(uri), "{uri}: {detail}");
    }
    Ok(())
}

fn glb(json: &str, bin: &[u8]) -> Vec<u8> {
    let mut json = json.as_bytes().to_vec();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut bin = bin.to_vec();
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let length = 12 + 8 + json.len() + 8 + bin.len();
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    for n in [length, json.len()] {
        out.extend_from_slice(&u32::try_from(n).unwrap_or(0).to_le_bytes());
        if n == json.len() {
            out.extend_from_slice(b"JSON");
            out.extend_from_slice(&json);
        }
    }
    out.extend_from_slice(&u32::try_from(bin.len()).unwrap_or(0).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}

#[test]
fn a_glb_reads_its_buffer_from_its_binary_chunk() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let rest = format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    );
    let json = bin.document(&rest, None, bin.bytes.len());
    let imported = gltf::read("model.glb", &glb(&json, &bin.bytes), &none)?;
    assert_eq!(imported.soup.triangle_count(), 1);
    Ok(())
}

#[test]
fn a_truncated_glb_is_refused_as_glb() {
    let mut bytes = glb(&triangle(r#""scenes":[]"#), &[]);
    bytes.truncate(20);
    assert!(
        matches!(
            gltf::read("model.glb", &bytes, &none),
            Err(IoError::Parse {
                format: Format::Glb,
                ..
            })
        ),
        "{:?}",
        gltf::read("model.glb", &bytes, &none)
    );
}

#[test]
fn a_node_reached_twice_is_refused() -> Result<(), Setup> {
    let doc = triangle(
        r#""nodes":[{"children":[1]},{"mesh":0,"children":[0]}],"scenes":[{"nodes":[0]}],"scene":0"#,
    );
    let detail = parse_detail(read(&doc))?;
    assert!(detail.contains("node"), "{detail}");
    Ok(())
}

#[test]
fn missing_resource_names_the_reference() {
    assert_eq!(
        IoError::MissingResource {
            uri: "model.bin".into()
        }
        .to_string(),
        "the file refers to \"model.bin\", which was not supplied beside it"
    );
}

/// `rest` placed after one mesh with these primitives, drawn by one node at the root.
fn drawn(bin: &Bin, primitives: &str, node: &str) -> String {
    bin.gltf(&format!(
        r#""meshes":[{{"primitives":[{primitives}]}}],"nodes":[{{"mesh":0{node}}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    ))
}

#[test]
fn the_second_primitive_indexes_its_own_vertices() -> Result<(), Setup> {
    let doc = triangle(
        r#""nodes":[{"mesh":0},{"mesh":0,"translation":[0,0,1]}],"scenes":[{"nodes":[0,1]}],"scene":0"#,
    );
    assert_eq!(ids(&read(&doc)?.soup)?, vec![[0, 1, 2], [3, 4, 5]]);
    Ok(())
}

#[test]
fn every_axis_is_scaled_and_kept() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&[[0.0, 0.0, 2.0], [1.0, 0.0, 2.0], [0.0, 1.0, 2.0]]);
    let doc = drawn(
        &bin,
        &format!(r#"{{"attributes":{{"POSITION":{p}}}}}"#),
        r#","scale":[1,2,3]"#,
    );
    let soup = read(&doc)?.soup;
    assert_eq!(position(&soup, 1)?, [1.0, 0.0, 6.0]);
    assert_eq!(position(&soup, 2)?, [0.0, 2.0, 6.0]);
    Ok(())
}

fn turned(axis: [f64; 3]) -> Result<TriangleSoup, Setup> {
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let q = [axis[0] * s, axis[1] * s, axis[2] * s, s];
    let doc = triangle(&format!(
        r#""nodes":[{{"mesh":0,"rotation":[{},{},{},{}]}}],"scenes":[{{"nodes":[0]}}],"scene":0"#,
        q[0], q[1], q[2], q[3]
    ));
    Ok(read(&doc)?.soup)
}

fn near(p: [f64; 3], q: [f64; 3]) -> bool {
    p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-6)
}

#[test]
fn quarter_turns_about_each_axis_move_points_the_right_way() -> Result<(), Setup> {
    let about_x = turned([1.0, 0.0, 0.0])?;
    assert!(
        near(position(&about_x, 2)?, [0.0, 0.0, 1.0]),
        "{:?}",
        position(&about_x, 2)?
    );
    let about_y = turned([0.0, 1.0, 0.0])?;
    assert!(
        near(position(&about_y, 1)?, [0.0, 0.0, -1.0]),
        "{:?}",
        position(&about_y, 1)?
    );
    Ok(())
}

#[test]
fn a_rotation_keeps_the_winding_and_a_flattening_scale_does_not_mirror() -> Result<(), Setup> {
    let plain = ids(&read(&triangle(
        r#""nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0"#,
    ))?
    .soup)?;
    assert_eq!(ids(&turned([0.0, 0.0, 1.0])?)?, plain);
    let flat =
        triangle(r#""nodes":[{"mesh":0,"scale":[1,1,0]}],"scenes":[{"nodes":[0]}],"scene":0"#);
    assert_eq!(ids(&read(&flat)?.soup)?, plain);
    Ok(())
}

#[test]
fn a_texture_set_beyond_the_first_is_counted_even_without_the_first() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let t1 = bin.uvs(&[[0.0, 0.0]; 3]);
    let doc = drawn(
        &bin,
        &format!(r#"{{"attributes":{{"POSITION":{p},"TEXCOORD_1":{t1}}}}}"#),
        "",
    );
    let soup = read(&doc)?.soup;
    assert_eq!(soup.extra_uv_sets(), 1);
    assert_eq!(soup.corner_uv(FaceId::new(0), 0), None);
    Ok(())
}

#[test]
fn uvs_of_another_length_than_the_positions_are_refused() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let t = bin.uvs(&[[0.0, 0.0]; 2]);
    let doc = drawn(
        &bin,
        &format!(r#"{{"attributes":{{"POSITION":{p},"TEXCOORD_0":{t}}}}}"#),
        "",
    );
    let detail = parse_detail(read(&doc))?;
    assert!(detail.contains("differ in length"), "{detail}");
    assert!(detail.contains("mesh 0 primitive 0"), "{detail}");
    Ok(())
}

#[test]
fn a_non_finite_coordinate_on_any_axis_is_named() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let data: Vec<u8> = [
        0.0f32,
        0.0,
        0.0,
        1.0,
        f32::NAN,
        0.0,
        0.0,
        1.0,
        f32::INFINITY,
    ]
    .iter()
    .flat_map(|c| c.to_le_bytes())
    .collect();
    let p = bin.accessor(&data, (5126, "VEC3"), 3, r#","min":[0,0,0],"max":[1,1,0]"#);
    let doc = drawn(&bin, &format!(r#"{{"attributes":{{"POSITION":{p}}}}}"#), "");
    assert_eq!(
        read(&doc).err(),
        Some(IoError::NonFiniteInput {
            vertices: vec![corner(0, 1), corner(0, 2)]
        })
    );
    Ok(())
}

#[test]
fn uvs_on_one_primitive_leave_the_others_corners_without() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let t = bin.uvs(&[[0.25, 0.5]; 3]);
    let doc = drawn(
        &bin,
        &format!(
            r#"{{"attributes":{{"POSITION":{p},"TEXCOORD_0":{t}}}}},{{"attributes":{{"POSITION":{p}}}}}"#
        ),
        "",
    );
    let soup = read(&doc)?.soup;
    let uv = soup
        .corner_uv(FaceId::new(0), 0)
        .ok_or("the first primitive lost its UVs")?;
    assert_eq!([uv.x, uv.y], [0.25, 0.5]);
    assert_eq!(soup.corner_uv(FaceId::new(1), 0), None);
    Ok(())
}

/// One primitive whose `attribute` is an accessor of `bytes` typed `(component, kind)`
/// with `extra` JSON; the positions are the triangle's unless `attribute` is `POSITION`.
fn typed(attribute: &str, bytes: &[u8], ty: (u32, &str), extra: &str) -> String {
    let mut bin = Bin::default();
    let a = bin.accessor(bytes, ty, 3, extra);
    let attributes = if attribute == "POSITION" {
        format!(r#""POSITION":{a}"#)
    } else {
        let p = bin.positions(&TRIANGLE);
        format!(r#""POSITION":{p},"{attribute}":{a}"#)
    };
    drawn(&bin, &format!(r#"{{"attributes":{{{attributes}}}}}"#), "")
}

#[test]
fn an_attribute_of_a_type_it_does_not_allow_is_refused() -> Result<(), Setup> {
    // Three-element bounds pass the crate's own validation, so the type check is the reader's.
    let bounds = r#","min":[0,0,0],"max":[1,1,1]"#;
    for doc in [
        typed("POSITION", &[0; 48], (5126, "VEC4"), bounds),
        typed(
            "POSITION",
            &[0; 9],
            (5121, "VEC3"),
            r#","normalized":true,"min":[0,0,0],"max":[1,1,1]"#,
        ),
        typed("TEXCOORD_0", &[0; 36], (5126, "VEC3"), ""),
        typed(
            "TEXCOORD_0",
            &[0; 6],
            (5120, "VEC2"),
            r#","normalized":true"#,
        ),
        typed("TEXCOORD_0", &[0; 6], (5121, "VEC2"), ""),
        typed(
            "TEXCOORD_0",
            &[0; 24],
            (5126, "VEC2"),
            r#","normalized":true"#,
        ),
    ] {
        let detail = parse_detail(read(&doc))?;
        assert!(detail.contains("does not allow"), "{detail}");
    }
    Ok(())
}

#[test]
fn normalized_short_uvs_are_scaled_by_their_own_range() -> Result<(), Setup> {
    let data: Vec<u8> = [0u16, 0, 0, 0, 0, 13107]
        .iter()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let doc = typed("TEXCOORD_0", &data, (5123, "VEC2"), r#","normalized":true"#);
    let soup = read(&doc)?.soup;
    let uv = soup.corner_uv(FaceId::new(0), 2).ok_or("no uv")?;
    assert_eq!([uv.x, uv.y], [0.0, 0.2]);
    Ok(())
}

#[test]
fn indices_that_are_not_unsigned_integers_are_refused() -> Result<(), Setup> {
    for (data, ty, extra) in [
        (vec![0u8; 12], (5126, "SCALAR"), ""),
        (vec![0u8; 3], (5120, "SCALAR"), ""),
        (vec![0u8; 12], (5123, "VEC2"), ""),
        (vec![0u8; 6], (5123, "SCALAR"), r#","normalized":true"#),
    ] {
        let mut bin = Bin::default();
        let p = bin.positions(&TRIANGLE);
        let i = bin.accessor(&data, ty, 3, extra);
        let doc = drawn(
            &bin,
            &format!(r#"{{"attributes":{{"POSITION":{p}}},"indices":{i}}}"#),
            "",
        );
        let detail = parse_detail(read(&doc))?;
        assert!(
            detail.contains("not unsigned integer indices"),
            "{ty:?}: {detail}"
        );
    }
    Ok(())
}

/// A document with `views` and `accessors` given as JSON over `bytes`, drawing accessor 0 as
/// POSITION and, if given, accessor 1 as TEXCOORD_0.
fn raw(bytes: &[u8], views: &str, accessors: &str, uvs: bool) -> String {
    let uv = if uvs { r#","TEXCOORD_0":1"# } else { "" };
    format!(
        r#"{{"asset":{{"version":"2.0"}},"buffers":[{{"uri":"data:application/octet-stream;base64,{}","byteLength":{}}}],"bufferViews":[{views}],"accessors":[{accessors}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0{uv}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0}}"#,
        STANDARD.encode(bytes),
        bytes.len()
    )
}

const POSITION_ACCESSOR: &str =
    r#""componentType":5126,"type":"VEC3","count":3,"min":[0,0,0],"max":[1,1,1]"#;

#[test]
fn an_interleaved_view_is_read_by_its_stride() -> Result<(), Setup> {
    let mut bytes = Vec::new();
    for (p, t) in TRIANGLE.iter().zip([[0.1f32, 0.2], [0.3, 0.4], [0.5, 0.6]]) {
        bytes.extend(p.iter().chain(&t).flat_map(|c| c.to_le_bytes()));
    }
    let doc = raw(
        &bytes,
        r#"{"buffer":0,"byteLength":60,"byteStride":20}"#,
        &format!(
            r#"{{"bufferView":0,{POSITION_ACCESSOR}}},{{"bufferView":0,"byteOffset":12,"componentType":5126,"type":"VEC2","count":3}}"#
        ),
        true,
    );
    let soup = read(&doc)?.soup;
    assert_eq!(position(&soup, 2)?, [0.0, 1.0, 0.0]);
    let uv = soup.corner_uv(FaceId::new(0), 2).ok_or("no uv")?;
    assert!(
        (uv.x - 0.5).abs() < 1e-7 && (uv.y - 0.6).abs() < 1e-7,
        "{uv:?}"
    );
    Ok(())
}

#[test]
fn a_stride_shorter_than_its_element_is_refused_not_read_overlapping() -> Result<(), Setup> {
    let bytes: Vec<u8> = TRIANGLE
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let doc = raw(
        &bytes,
        r#"{"buffer":0,"byteLength":36,"byteStride":8}"#,
        &format!(r#"{{"bufferView":0,{POSITION_ACCESSOR}}}"#),
        false,
    );
    let detail = parse_detail(read(&doc))?;
    assert!(detail.contains("stride"), "{detail}");
    Ok(())
}

#[test]
fn an_accessor_offset_into_its_view_is_honoured() -> Result<(), Setup> {
    let points = [
        [9.0f32, 9.0, 9.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let bytes: Vec<u8> = points
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let doc = raw(
        &bytes,
        r#"{"buffer":0,"byteLength":48}"#,
        &format!(r#"{{"bufferView":0,"byteOffset":12,{POSITION_ACCESSOR}}}"#),
        false,
    );
    let soup = read(&doc)?.soup;
    assert_eq!(position(&soup, 0)?, [0.0, 0.0, 0.0]);
    assert_eq!(position(&soup, 1)?, [1.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn an_accessor_or_view_past_its_bounds_is_refused_not_read() -> Result<(), Setup> {
    let bytes: Vec<u8> = TRIANGLE
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect();
    let past_buffer = raw(
        &bytes,
        r#"{"buffer":0,"byteLength":48}"#,
        &format!(r#"{{"bufferView":0,{POSITION_ACCESSOR}}}"#),
        false,
    );
    parse_detail(read(&past_buffer))?;
    let four = r#""componentType":5126,"type":"VEC3","count":4,"min":[0,0,0],"max":[1,1,1]"#;
    let past_view = raw(
        &bytes,
        r#"{"buffer":0,"byteLength":36}"#,
        &format!(r#"{{"bufferView":0,{four}}}"#),
        false,
    );
    let detail = parse_detail(read(&past_view))?;
    assert!(
        detail.contains("past the end of its buffer view"),
        "{detail}"
    );
    let short_view = raw(
        &bytes,
        r#"{"buffer":0,"byteLength":32}"#,
        &format!(r#"{{"bufferView":0,{POSITION_ACCESSOR}}}"#),
        false,
    );
    let detail = parse_detail(read(&short_view))?;
    assert!(
        detail.contains("past the end of its buffer view"),
        "{detail}"
    );
    Ok(())
}

#[test]
fn a_data_uri_that_is_not_base64_is_refused() -> Result<(), Setup> {
    for uri in [
        "data:application/octet-stream,AAAAAAAAAAAAAAAA",
        "data:application/octet-stream;base64,!!!!",
    ] {
        let (doc, _) = beside(uri);
        let detail = parse_detail(read(&doc))?;
        assert!(detail.contains("not base64 data"), "{uri}: {detail}");
    }
    Ok(())
}

#[test]
fn empty_path_segments_are_skipped_and_a_missing_reference_is_named_as_written() -> Result<(), Setup>
{
    let (doc, bytes) = beside("x//y.bin");
    let lookup = |name: &str| (name == "a/x/y.bin").then_some(bytes.as_slice());
    assert_eq!(
        gltf::read("a/b.gltf", doc.as_bytes(), &lookup)?
            .soup
            .triangle_count(),
        1
    );
    let (doc, _) = beside("my%20model.bin");
    assert_eq!(
        read(&doc).err(),
        Some(IoError::MissingResource {
            uri: "my%20model.bin".into()
        })
    );
    Ok(())
}

#[test]
fn a_gltf_buffer_with_no_uri_needs_a_binary_chunk() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let rest = format!(
        r#""meshes":[{{"primitives":[{{"attributes":{{"POSITION":{p}}}}}]}}],"nodes":[{{"mesh":0}}],"scenes":[{{"nodes":[0]}}],"scene":0"#
    );
    let detail = parse_detail(read(&bin.document(&rest, None, bin.bytes.len())))?;
    assert!(detail.contains("binary chunk"), "{detail}");
    Ok(())
}

#[test]
fn the_file_s_scene_is_drawn_when_it_is_not_scene_zero() -> Result<(), Setup> {
    let doc = triangle(
        r#""nodes":[{"mesh":0,"translation":[10,0,0]},{"mesh":0,"translation":[20,0,0]}],"scenes":[{"nodes":[0]},{"nodes":[1]}],"scene":1"#,
    );
    let soup = read(&doc)?.soup;
    assert_eq!(soup.vertex_count(), 3);
    assert_eq!(position(&soup, 0)?, [20.0, 0.0, 0.0]);
    Ok(())
}

#[test]
fn children_are_drawn_in_array_order() -> Result<(), Setup> {
    let doc = triangle(
        r#""nodes":[{"children":[1,2]},{"mesh":0,"translation":[0,0,1]},{"mesh":0,"translation":[0,0,2]}],"scenes":[{"nodes":[0]}],"scene":0"#,
    );
    let soup = read(&doc)?.soup;
    assert_eq!(position(&soup, 0)?, [0.0, 0.0, 1.0]);
    assert_eq!(position(&soup, 3)?, [0.0, 0.0, 2.0]);
    Ok(())
}

#[test]
fn a_mesh_with_morph_targets_on_one_primitive_is_listed_once() -> Result<(), Setup> {
    let mut bin = Bin::default();
    let p = bin.positions(&TRIANGLE);
    let d = bin.positions(&[[0.0; 3]; 3]);
    let doc = drawn(
        &bin,
        &format!(
            r#"{{"attributes":{{"POSITION":{p}}},"targets":[{{"POSITION":{d}}}]}},{{"attributes":{{"POSITION":{p}}}}}"#
        ),
        "",
    );
    assert_eq!(
        read(&doc)?.ignored,
        vec![Ignored::MorphTargets { node: 0, mesh: 0 }]
    );
    Ok(())
}
