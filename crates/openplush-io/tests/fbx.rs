//! Binary FBX: the models under the root drawn with the full transform stack, the file's
//! axes turned to Y-up, one primitive per (model, material), UV layers, what was not read
//! listed, and ASCII FBX refused. Every file is built here with `fbxcel`'s writer.

use openplush_geom::{DVec2, DVec3};
use openplush_io::{Format, Ignored, Imported, IoError, fbx};
use openplush_mesh::{FaceId, MaterialIndex, SourceCorner, VertexId};
use openplush_testkit::fbx::{
    Node, Scene, Value, binary, int, layer, material, mesh, model, object, vector,
};

type Setup = Box<dyn std::error::Error>;

/// Three control points, one on each axis, and the one triangle through them.
const AXES: [f64; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
const TRIANGLE: [i32; 3] = [0, 1, -3];

/// The unit cube's corners, `i` at (i & 1, i >> 1 & 1, i >> 2 & 1) rearranged so the
/// bottom face runs 0, 1, 2, 3 counter-clockwise from above.
const CUBE: [[f64; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
    [1.0, 1.0, 1.0],
    [0.0, 1.0, 1.0],
];

/// Six outward quads: bottom, top, front, back, left, right.
const CUBE_QUADS: [[i32; 4]; 6] = [
    [0, 3, 2, 1],
    [4, 5, 6, 7],
    [0, 1, 5, 4],
    [2, 3, 7, 6],
    [0, 4, 7, 3],
    [1, 2, 6, 5],
];

fn cube_mesh(id: i64) -> Node {
    let polygons = CUBE_QUADS
        .iter()
        .flat_map(|&[a, b, c, d]| [a, b, c, -d - 1])
        .collect();
    mesh(
        id,
        "Cube",
        CUBE.iter().flatten().copied().collect(),
        polygons,
    )
}

fn triangle_mesh(id: i64) -> Node {
    mesh(id, "Tri", AXES.to_vec(), TRIANGLE.to_vec())
}

/// Model 1 under the root, with `props`, drawing the triangle (geometry 2).
fn one_triangle(props: Vec<Node>) -> Scene {
    let mut scene = Scene::default();
    scene
        .object(model(1, "Tri", props))
        .object(triangle_mesh(2))
        .connect(1, 0)
        .connect(2, 1);
    scene
}

fn read(scene: &Scene) -> Result<Imported, Setup> {
    Ok(fbx::read(&scene.bytes()?)?)
}

fn positions(imported: &Imported) -> Vec<DVec3> {
    imported.soup.positions().to_vec()
}

#[track_caller]
fn assert_near(got: &[DVec3], want: &[[f64; 3]]) {
    assert_eq!(got.len(), want.len(), "{got:?}");
    for (g, w) in got.iter().zip(want) {
        let w = DVec3::new(w[0], w[1], w[2]);
        assert!((*g - w).length() < 1e-9, "{got:?} is not {want:?}");
    }
}

fn triangles(imported: &Imported) -> Vec<[VertexId; 3]> {
    (0..imported.soup.triangle_count())
        .filter_map(|f| imported.soup.triangle(FaceId::new(f as u32)))
        .collect()
}

fn v(i: u32) -> VertexId {
    VertexId::new(i)
}

/// The detail of an FBX `Parse` refusal; any other outcome is an error.
fn parse_detail(result: Result<Imported, IoError>) -> Result<String, Setup> {
    match result {
        Err(IoError::Parse {
            format: Format::Fbx,
            detail,
        }) => Ok(detail),
        other => Err(format!("expected an FBX parse refusal, got {other:?}").into()),
    }
}

// Rotations by a quarter turn, right-handed, for writing expected positions by hand.
fn rx([x, y, z]: [f64; 3]) -> [f64; 3] {
    [x, -z, y]
}
fn ry([x, y, z]: [f64; 3]) -> [f64; 3] {
    [z, y, -x]
}
fn rz([x, y, z]: [f64; 3]) -> [f64; 3] {
    [-y, x, z]
}
fn rz_back([x, y, z]: [f64; 3]) -> [f64; 3] {
    [y, -x, z]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f64; 3], s: [f64; 3]) -> [f64; 3] {
    [a[0] * s[0], a[1] * s[1], a[2] * s[2]]
}

#[test]
fn a_cube_under_a_rotated_pivoted_parent_lands_where_the_stack_puts_it() -> Result<(), Setup> {
    let mut scene = Scene::default();
    scene
        .object(model(
            1,
            "Parent",
            vec![
                vector("Lcl Translation", [10.0, 0.0, 0.0]),
                vector("Lcl Rotation", [0.0, 0.0, 90.0]),
                vector("RotationPivot", [1.0, 0.0, 0.0]),
            ],
        ))
        .object(model(
            2,
            "Cube",
            vec![
                vector("Lcl Translation", [0.0, 0.0, 2.0]),
                vector("Lcl Scaling", [2.0, 2.0, 2.0]),
                vector("ScalingPivot", [0.5, 0.5, 0.5]),
            ],
        ))
        .object(cube_mesh(3))
        .connect(1, 0)
        .connect(2, 1)
        .connect(3, 2);
    let imported = read(&scene)?;
    // The cube scales by 2 about (0.5, 0.5, 0.5) and moves up 2: q = 2p + (-0.5, -0.5, 1.5).
    // The parent turns q a quarter about z through the pivot (1, 0, 0), then moves it 10
    // along x: (1 - q.y, q.x - 1, q.z) + (10, 0, 0).
    let want: Vec<[f64; 3]> = CUBE
        .iter()
        .map(|p| [11.5 - 2.0 * p[1], 2.0 * p[0] - 1.5, 2.0 * p[2] + 1.5])
        .collect();
    assert_near(&positions(&imported), &want);
    assert_eq!(imported.soup.triangle_count(), 12);
    assert!(imported.ignored.is_empty());
    Ok(())
}

#[test]
fn every_part_of_the_transform_stack_applies_in_the_sdk_order() -> Result<(), Setup> {
    let (t, roff, rp) = ([1.0, 2.0, 3.0], [0.5, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let (soff, sp, s) = ([0.0, 0.0, 0.25], [1.0, 0.0, 0.0], [2.0, 3.0, 4.0]);
    let imported = read(&one_triangle(vec![
        vector("Lcl Translation", t),
        vector("RotationOffset", roff),
        vector("RotationPivot", rp),
        vector("PreRotation", [90.0, 0.0, 0.0]),
        vector("Lcl Rotation", [0.0, 90.0, 0.0]),
        vector("PostRotation", [0.0, 0.0, 90.0]),
        vector("ScalingOffset", soff),
        vector("ScalingPivot", sp),
        vector("Lcl Scaling", s),
    ]))?;
    // T Roff Rp Rpre R Rpost^-1 Rp^-1 Soff Sp S Sp^-1, rightmost first, one step at a time.
    let stack = |p: [f64; 3]| {
        let p = add(add(scale(sub(p, sp), s), sp), soff);
        let p = rx(ry(rz_back(sub(p, rp))));
        add(add(add(p, rp), roff), t)
    };
    let want: Vec<[f64; 3]> = AXES.chunks(3).map(|c| stack([c[0], c[1], c[2]])).collect();
    assert_near(&positions(&imported), &want);
    Ok(())
}

#[test]
fn the_rotation_order_property_picks_the_euler_order() -> Result<(), Setup> {
    let turned = vector("Lcl Rotation", [90.0, 90.0, 0.0]);
    // XYZ, the default: x first, then y.
    let xyz = read(&one_triangle(vec![turned.clone()]))?;
    let points = AXES.chunks(3).map(|c| [c[0], c[1], c[2]]);
    let want: Vec<[f64; 3]> = points.clone().map(|p| ry(rx(p))).collect();
    assert_near(&positions(&xyz), &want);
    // YXZ (3): y first, then x.
    let yxz = read(&one_triangle(vec![turned, int("RotationOrder", 3)]))?;
    let want: Vec<[f64; 3]> = points.map(|p| rx(ry(p))).collect();
    assert_near(&positions(&yxz), &want);
    Ok(())
}

#[test]
fn post_rotation_is_undone_and_pre_rotation_applied() -> Result<(), Setup> {
    let quarter = [0.0, 0.0, 90.0];
    let points: Vec<[f64; 3]> = AXES.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
    let post = read(&one_triangle(vec![vector("PostRotation", quarter)]))?;
    let want: Vec<[f64; 3]> = points.iter().map(|&p| rz_back(p)).collect();
    assert_near(&positions(&post), &want);
    let pre = read(&one_triangle(vec![vector("PreRotation", quarter)]))?;
    let want: Vec<[f64; 3]> = points.iter().map(|&p| rz(p)).collect();
    assert_near(&positions(&pre), &want);
    let both = read(&one_triangle(vec![
        vector("PreRotation", quarter),
        vector("PostRotation", quarter),
    ]))?;
    assert_near(&positions(&both), &points);
    Ok(())
}

#[test]
fn the_geometric_transform_moves_only_its_own_models_geometry() -> Result<(), Setup> {
    let mut scene = Scene::default();
    scene
        .object(model(
            1,
            "Parent",
            vec![
                vector("GeometricTranslation", [100.0, 0.0, 0.0]),
                vector("GeometricRotation", [0.0, 0.0, 90.0]),
                vector("GeometricScaling", [2.0, 2.0, 2.0]),
            ],
        ))
        .object(triangle_mesh(2))
        .object(model(3, "Child", Vec::new()))
        .object(triangle_mesh(4))
        .connect(1, 0)
        .connect(2, 1)
        .connect(3, 1)
        .connect(4, 3);
    let imported = read(&scene)?;
    let points: Vec<[f64; 3]> = AXES.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
    let mut want: Vec<[f64; 3]> = points
        .iter()
        .map(|&p| add(rz(scale(p, [2.0; 3])), [100.0, 0.0, 0.0]))
        .collect();
    want.extend(&points);
    assert_near(&positions(&imported), &want);
    let primitives: Vec<u32> = (0..2)
        .filter_map(|f| imported.soup.corner_source(FaceId::new(f), 0))
        .map(|c| c.primitive)
        .collect();
    assert_eq!(primitives, [0, 1]);
    Ok(())
}

#[test]
fn a_z_up_file_comes_out_y_up() -> Result<(), Setup> {
    let mut scene = one_triangle(Vec::new());
    // Blender's export axes: Z up, -Y front, X right.
    scene.axes([2, 1], [1, -1], [0, 1]);
    let imported = read(&scene)?;
    // (x, y, z) in the file is (x, z, -y) in Y-up axes: what was up is +y.
    assert_near(
        &positions(&imported),
        &[[1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]],
    );
    assert_eq!(triangles(&imported), [[v(0), v(1), v(2)]]);
    Ok(())
}

#[test]
fn absent_axis_settings_are_y_up() -> Result<(), Setup> {
    let mut explicit = one_triangle(Vec::new());
    explicit.axes([1, 1], [2, 1], [0, 1]);
    assert_eq!(read(&explicit)?, read(&one_triangle(Vec::new()))?);
    assert_near(
        &positions(&read(&explicit)?),
        &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    );
    Ok(())
}

#[test]
fn a_mirroring_transform_reverses_the_winding() -> Result<(), Setup> {
    let mirrored = read(&one_triangle(vec![vector("Lcl Scaling", [-1.0, 1.0, 1.0])]))?;
    assert_near(
        &positions(&mirrored),
        &[[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    );
    assert_eq!(triangles(&mirrored), [[v(0), v(2), v(1)]]);
    let sources: Vec<u32> = (0..3)
        .filter_map(|c| mirrored.soup.corner_source(FaceId::new(0), c))
        .map(|c| c.vertex)
        .collect();
    assert_eq!(sources, [0, 2, 1]);
    Ok(())
}

#[test]
fn a_left_handed_axis_system_reverses_the_winding_and_a_second_mirror_cancels_it()
-> Result<(), Setup> {
    let mut left = one_triangle(Vec::new());
    left.axes([1, 1], [2, 1], [0, -1]);
    let left = read(&left)?;
    assert_near(
        &positions(&left),
        &[[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    );
    assert_eq!(triangles(&left), [[v(0), v(2), v(1)]]);
    let mut twice = one_triangle(vec![vector("Lcl Scaling", [-1.0, 1.0, 1.0])]);
    twice.axes([1, 1], [2, 1], [0, -1]);
    assert_eq!(triangles(&read(&twice)?), [[v(0), v(1), v(2)]]);
    Ok(())
}

/// The cube with two materials: `slots` per quad, materials connected `connected` order.
fn two_material_cube(slots: Vec<i32>) -> Scene {
    let materials = layer(
        "LayerElementMaterial",
        "ByPolygon",
        "IndexToDirect",
        vec![Node::new("Materials", vec![slots.into()])],
    );
    let mut scene = Scene::default();
    scene
        .object(model(1, "Cube", Vec::new()))
        .object(cube_mesh(2).with([materials]))
        .object(material(21, "Blue"))
        .object(material(20, "Red"))
        .connect(1, 0)
        .connect(2, 1)
        .connect(20, 1)
        .connect(21, 1);
    scene
}

#[test]
fn two_materials_on_one_mesh_are_two_primitives_each_with_its_own() -> Result<(), Setup> {
    let imported = read(&two_material_cube(vec![1, 0, 1, 0, 1, 0]))?;
    let soup = &imported.soup;
    // Names in object order; slot 0 is the first connected material, Red.
    assert_eq!(soup.materials().names(), ["Blue", "Red"]);
    assert_eq!(
        soup.materials().by_primitive(),
        [Some(MaterialIndex::new(1)), Some(MaterialIndex::new(0))]
    );
    assert_eq!(soup.triangle_count(), 12);
    let corners: Vec<SourceCorner> = (0..12)
        .flat_map(|f| (0..3).filter_map(move |c| soup.corner_source(FaceId::new(f), c)))
        .collect();
    // Slot 0 holds the top, back and right quads, in file order; then slot 1's.
    let primitive_of = |f: usize| corners[3 * f].primitive;
    assert_eq!((0..12).map(primitive_of).collect::<Vec<_>>(), {
        let mut p = vec![0; 6];
        p.extend([1; 6]);
        p
    });
    let first_quad: Vec<u32> = corners[..6].iter().map(|c| c.vertex).collect();
    assert!(
        first_quad.iter().all(|i| [4, 5, 6, 7].contains(i)),
        "{first_quad:?}"
    );
    // Each primitive carries the whole control-point array.
    assert_eq!(soup.vertex_count(), 16);
    Ok(())
}

#[test]
fn an_all_same_material_layer_gives_its_one_slot() -> Result<(), Setup> {
    let mut scene = two_material_cube(Vec::new());
    scene.objects[1] = cube_mesh(2).with([layer(
        "LayerElementMaterial",
        "AllSame",
        "IndexToDirect",
        vec![Node::new("Materials", vec![vec![1_i32].into()])],
    )]);
    let imported = read(&scene)?;
    assert_eq!(
        imported.soup.materials().by_primitive(),
        [Some(MaterialIndex::new(0))]
    );
    assert_eq!(imported.soup.triangle_count(), 12);
    Ok(())
}

#[test]
fn a_model_with_no_material_layer_is_one_primitive_with_its_first_material_or_none()
-> Result<(), Setup> {
    let bare = read(&one_triangle(Vec::new()))?;
    assert!(bare.soup.materials().names().is_empty());
    assert_eq!(bare.soup.materials().by_primitive(), [None]);
    let mut dressed = one_triangle(Vec::new());
    dressed.object(material(9, "")).connect(9, 1);
    let dressed = read(&dressed)?;
    assert_eq!(dressed.soup.materials().names(), ["material 0"]);
    assert_eq!(
        dressed.soup.materials().by_primitive(),
        [Some(MaterialIndex::new(0))]
    );
    Ok(())
}

#[test]
fn a_material_slot_with_no_connected_material_is_parse() -> Result<(), Setup> {
    let detail = parse_detail(fbx::read(
        &two_material_cube(vec![0, 1, 2, 0, 1, 0]).bytes()?,
    ))?;
    assert!(detail.contains("material"), "{detail}");
    Ok(())
}

fn uv_triangle(polygons: Vec<i32>, uv_layers: Vec<Node>) -> Scene {
    let mut scene = Scene::default();
    scene
        .object(model(1, "Tri", Vec::new()))
        .object(mesh(2, "Tri", AXES.to_vec(), polygons).with(uv_layers))
        .connect(1, 0)
        .connect(2, 1);
    scene
}

fn corner_uvs(imported: &Imported) -> Vec<Option<DVec2>> {
    (0..3)
        .map(|c| imported.soup.corner_uv(FaceId::new(0), c))
        .collect()
}

fn uv(u: f64, v: f64) -> Option<DVec2> {
    Some(DVec2::new(u, v))
}

#[test]
fn uvs_by_polygon_vertex_index_to_direct_follow_the_uv_index() -> Result<(), Setup> {
    let layer = layer(
        "LayerElementUV",
        "ByPolygonVertex",
        "IndexToDirect",
        vec![
            Node::new("UV", vec![vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0].into()]),
            Node::new("UVIndex", vec![vec![2_i32, 1, 0].into()]),
        ],
    );
    let imported = read(&uv_triangle(TRIANGLE.to_vec(), vec![layer]))?;
    assert_eq!(
        corner_uvs(&imported),
        [uv(0.0, 1.0), uv(1.0, 0.0), uv(0.0, 0.0)]
    );
    assert_eq!(imported.soup.extra_uv_sets(), 0);
    Ok(())
}

#[test]
fn uvs_by_control_point_direct_follow_the_control_point() -> Result<(), Setup> {
    // FBX writes "ByVertice" for by-control-point mapping.
    for mapping in ["ByVertice", "ByControlPoint"] {
        let layer = layer(
            "LayerElementUV",
            mapping,
            "Direct",
            vec![Node::new(
                "UV",
                vec![vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0].into()],
            )],
        );
        let imported = read(&uv_triangle(vec![1, 2, -1], vec![layer]))?;
        assert_eq!(
            corner_uvs(&imported),
            [uv(1.0, 0.0), uv(0.0, 1.0), uv(0.0, 0.0)],
            "{mapping}"
        );
    }
    Ok(())
}

#[test]
fn the_first_uv_layer_is_read_and_the_others_counted() -> Result<(), Setup> {
    let direct = |u: f64| {
        layer(
            "LayerElementUV",
            "ByPolygonVertex",
            "Direct",
            vec![Node::new("UV", vec![vec![u, 0.0, u, 0.0, u, 0.0].into()])],
        )
    };
    let imported = read(&uv_triangle(
        TRIANGLE.to_vec(),
        vec![direct(0.25), direct(0.5), direct(0.75)],
    ))?;
    assert_eq!(corner_uvs(&imported), [uv(0.25, 0.0); 3]);
    assert_eq!(imported.soup.extra_uv_sets(), 2);
    Ok(())
}

#[test]
fn a_uv_index_past_the_uv_array_is_parse() -> Result<(), Setup> {
    let indexed = |uv: Vec<f64>, index: Vec<i32>| {
        layer(
            "LayerElementUV",
            "ByPolygonVertex",
            "IndexToDirect",
            vec![
                Node::new("UV", vec![uv.into()]),
                Node::new("UVIndex", vec![index.into()]),
            ],
        )
    };
    for (uv, index) in [
        (vec![0.0, 0.0], vec![0, 0, 1]),
        (vec![0.0, 0.0], vec![0, 0, -1]),
        (vec![0.0, 0.0, 1.0, 0.0], vec![0, 1]),
    ] {
        let scene = uv_triangle(TRIANGLE.to_vec(), vec![indexed(uv, index.clone())]);
        let detail = parse_detail(fbx::read(&scene.bytes()?))?;
        assert!(detail.contains("UV"), "{index:?}: {detail}");
    }
    let direct = layer(
        "LayerElementUV",
        "ByControlPoint",
        "Direct",
        vec![Node::new("UV", vec![vec![0.0, 0.0, 1.0, 0.0].into()])],
    );
    parse_detail(fbx::read(
        &uv_triangle(TRIANGLE.to_vec(), vec![direct]).bytes()?,
    ))?;
    Ok(())
}

#[test]
fn a_uv_layer_that_cannot_be_read_is_parse() -> Result<(), Setup> {
    let uv = || Node::new("UV", vec![vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0].into()]);
    let layers = [
        layer("LayerElementUV", "ByEdge", "Direct", vec![uv()]),
        layer("LayerElementUV", "ByPolygonVertex", "Indexed", vec![uv()]),
        // IndexToDirect with no UVIndex.
        layer(
            "LayerElementUV",
            "ByPolygonVertex",
            "IndexToDirect",
            vec![uv()],
        ),
        // No UV array.
        layer("LayerElementUV", "ByPolygonVertex", "Direct", Vec::new()),
        // An odd number of values.
        layer(
            "LayerElementUV",
            "ByPolygonVertex",
            "Direct",
            vec![Node::new("UV", vec![vec![0.0, 0.0, 1.0, 0.0, 0.0].into()])],
        ),
    ];
    for layer in layers {
        let scene = uv_triangle(TRIANGLE.to_vec(), vec![layer.clone()]);
        let detail = parse_detail(fbx::read(&scene.bytes()?))?;
        assert!(detail.contains("UV"), "{layer:?}: {detail}");
    }
    Ok(())
}

#[test]
fn a_non_finite_texture_coordinate_is_parse() -> Result<(), Setup> {
    let layer = layer(
        "LayerElementUV",
        "ByPolygonVertex",
        "Direct",
        vec![Node::new(
            "UV",
            vec![vec![0.0, 0.0, f64::INFINITY, 0.0, 0.0, 1.0].into()],
        )],
    );
    let detail = parse_detail(fbx::read(
        &uv_triangle(TRIANGLE.to_vec(), vec![layer]).bytes()?,
    ))?;
    assert!(detail.contains("finite"), "{detail}");
    Ok(())
}

#[test]
fn deformers_and_animation_stacks_are_listed_as_ignored() -> Result<(), Setup> {
    let mut scene = one_triangle(Vec::new());
    scene
        .object(object("Deformer", 5, "Armature", "Deformer", "Skin"))
        .object(object("Deformer", 6, "Smile", "Deformer", "BlendShape"))
        .object(object("AnimationStack", 7, "Take 001", "AnimStack", ""))
        .connect(5, 2)
        .connect(6, 2);
    let imported = read(&scene)?;
    assert_eq!(
        imported.ignored,
        [
            Ignored::Deformer {
                model: "Tri".into(),
                deformer: "Armature".into()
            },
            Ignored::Deformer {
                model: "Tri".into(),
                deformer: "Smile".into()
            },
            Ignored::AnimationStack {
                stack: "Take 001".into()
            },
        ]
    );
    // The rest pose: the control points as written.
    assert_eq!(
        imported.soup,
        read(&one_triangle(Vec::new()))?.soup,
        "a deformer changed the soup"
    );
    Ok(())
}

#[test]
fn geometry_that_is_not_a_mesh_is_listed_and_not_drawn() -> Result<(), Setup> {
    let mut scene = one_triangle(Vec::new());
    scene
        .object(object("Geometry", 8, "Guide", "Geometry", "Line"))
        .connect(8, 1);
    let imported = read(&scene)?;
    assert_eq!(imported.soup.triangle_count(), 1);
    assert_eq!(
        imported.ignored,
        [Ignored::Geometry {
            model: "Tri".into(),
            geometry: "Guide".into()
        }]
    );
    Ok(())
}

#[test]
fn an_ascii_fbx_file_is_parse_naming_it_as_ascii() -> Result<(), Setup> {
    let ascii = b"; FBX 7.4.0 project file\n; ----------------------------------------------------\n\nFBXHeaderExtension:  {\n\tFBXHeaderVersion: 1003\n}\n";
    let detail = parse_detail(fbx::read(ascii))?;
    assert!(detail.contains("ASCII"), "{detail}");
    // A writer that leaves out the comment line starts with the first node.
    let bare = parse_detail(fbx::read(b"\n  FBXHeaderExtension:  {\n}\n"))?;
    assert!(bare.contains("ASCII"), "{bare}");
    let garbage = parse_detail(fbx::read(b"\x00\x01not a model at all"))?;
    assert!(!garbage.contains("ASCII"), "{garbage}");
    Ok(())
}

#[test]
fn a_binary_fbx_that_is_not_version_7_or_is_cut_short_is_parse() -> Result<(), Setup> {
    let mut six = b"Kaydara FBX Binary  \x00\x1a\x00".to_vec();
    six.extend(6100_u32.to_le_bytes());
    six.extend([0; 32]);
    let detail = parse_detail(fbx::read(&six))?;
    assert!(detail.contains("6100"), "{detail}");
    let whole = one_triangle(Vec::new()).bytes()?;
    parse_detail(fbx::read(&whole[..whole.len() / 2]))?;
    let header = parse_detail(fbx::read(&whole[..25]))?;
    assert!(header.contains("header"), "{header}");
    Ok(())
}

#[test]
fn a_node_whose_attributes_end_past_the_file_is_parse() -> Result<(), Setup> {
    let mut bytes = one_triangle(Vec::new()).bytes()?;
    // The first node's header follows the 27-byte file header: its end offset, attribute
    // count and attribute length, each a little-endian u32 in FBX 7.4.
    bytes[35..39].copy_from_slice(&0xffff_fff0_u32.to_le_bytes());
    let detail = parse_detail(fbx::read(&bytes))?;
    assert!(detail.contains("past the end of the file"), "{detail}");
    Ok(())
}

#[test]
fn a_polygon_index_past_the_control_points_is_parse() -> Result<(), Setup> {
    let mut scene = one_triangle(Vec::new());
    scene.objects[1] = mesh(2, "Tri", AXES.to_vec(), vec![0, 1, -6]);
    let detail = parse_detail(fbx::read(&scene.bytes()?))?;
    assert!(detail.contains("Tri"), "{detail}");
    Ok(())
}

#[test]
fn an_unclosed_or_too_small_polygon_is_parse() -> Result<(), Setup> {
    for polygons in [vec![0, 1, 2], vec![0, -2], vec![0, 1, -3, 2]] {
        let mut scene = one_triangle(Vec::new());
        scene.objects[1] = mesh(2, "Tri", AXES.to_vec(), polygons.clone());
        parse_detail(fbx::read(&scene.bytes()?))?;
    }
    Ok(())
}

#[test]
fn a_quad_is_split_into_two_triangles_keeping_its_control_points() -> Result<(), Setup> {
    let square = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
    let mut scene = one_triangle(Vec::new());
    scene.objects[1] = mesh(2, "Square", square, vec![0, 1, 2, -4]);
    let imported = read(&scene)?;
    assert_eq!(imported.soup.triangle_count(), 2);
    let mut used: Vec<VertexId> = triangles(&imported).concat();
    used.sort();
    used.dedup();
    assert_eq!(used, [v(0), v(1), v(2), v(3)]);
    Ok(())
}

#[test]
fn a_non_finite_control_point_is_non_finite_input() -> Result<(), Setup> {
    let mut scene = one_triangle(Vec::new());
    let points = vec![1.0, 0.0, 0.0, f64::NAN, 1.0, 0.0, 0.0, 0.0, 1.0];
    scene.objects[1] = mesh(2, "Tri", points, TRIANGLE.to_vec());
    assert_eq!(
        fbx::read(&scene.bytes()?),
        Err(IoError::NonFiniteInput {
            vertices: vec![SourceCorner {
                primitive: 0,
                vertex: 1
            }]
        })
    );
    Ok(())
}

#[test]
fn f32_control_points_are_read() -> Result<(), Setup> {
    let mut scene = one_triangle(Vec::new());
    let points: Vec<f32> = AXES.iter().map(|&c| c as f32).collect();
    scene.objects[1] = triangle_mesh(2);
    scene.objects[1].children[0] = Node::new("Vertices", vec![Value::ArrF32(points)]);
    assert_eq!(read(&scene)?.soup, read(&one_triangle(Vec::new()))?.soup);
    Ok(())
}

#[test]
fn only_models_under_the_root_are_drawn_and_one_reached_twice_is_parse() -> Result<(), Setup> {
    let mut stray = one_triangle(Vec::new());
    stray
        .object(model(3, "Stray", Vec::new()))
        .object(triangle_mesh(4))
        .connect(4, 3);
    assert_eq!(read(&stray)?.soup.triangle_count(), 1);
    let mut twice = Scene::default();
    twice
        .object(model(1, "A", Vec::new()))
        .object(model(3, "B", Vec::new()))
        .object(triangle_mesh(2))
        .connect(1, 0)
        .connect(3, 0)
        .connect(2, 3)
        .connect(3, 1);
    parse_detail(fbx::read(&twice.bytes()?))?;
    Ok(())
}

#[test]
fn a_mesh_drawn_by_two_models_is_two_primitives() -> Result<(), Setup> {
    let mut scene = Scene::default();
    scene
        .object(model(1, "A", Vec::new()))
        .object(model(
            3,
            "B",
            vec![vector("Lcl Translation", [0.0, 5.0, 0.0])],
        ))
        .object(triangle_mesh(2))
        .connect(1, 0)
        .connect(3, 0)
        .connect(2, 1)
        .connect(2, 3);
    let imported = read(&scene)?;
    assert_eq!(imported.soup.materials().by_primitive(), [None, None]);
    let mut want: Vec<[f64; 3]> = AXES.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
    want.extend(
        AXES.chunks(3)
            .map(|c| [c[0], c[1] + 5.0, c[2]])
            .collect::<Vec<_>>(),
    );
    assert_near(&positions(&imported), &want);
    Ok(())
}

#[test]
fn an_unknown_rotation_order_or_axis_is_parse() -> Result<(), Setup> {
    // 6 is spheric XYZ, which has no single Euler order to read it in.
    for order in [6, 7, -1] {
        let detail = parse_detail(fbx::read(
            &one_triangle(vec![int("RotationOrder", order)]).bytes()?,
        ))?;
        assert!(detail.contains("RotationOrder"), "{order}: {detail}");
    }
    let mut bad_sign = one_triangle(Vec::new());
    bad_sign.axes([1, 2], [2, 1], [0, 1]);
    parse_detail(fbx::read(&bad_sign.bytes()?))?;
    let mut twice_up = one_triangle(Vec::new());
    twice_up.axes([1, 1], [1, 1], [0, 1]);
    parse_detail(fbx::read(&twice_up.bytes()?))?;
    let mut no_axis = one_triangle(Vec::new());
    no_axis.axes([3, 1], [2, 1], [0, 1]);
    parse_detail(fbx::read(&no_axis.bytes()?))?;
    Ok(())
}

#[test]
fn the_same_bytes_read_the_same() -> Result<(), Setup> {
    let bytes = two_material_cube(vec![1, 0, 1, 0, 1, 0]).bytes()?;
    assert_eq!(fbx::read(&bytes)?, fbx::read(&bytes)?);
    assert_eq!(bytes, two_material_cube(vec![1, 0, 1, 0, 1, 0]).bytes()?);
    Ok(())
}

#[test]
fn a_file_built_by_hand_without_the_scene_helper_reads() -> Result<(), Setup> {
    let bytes = binary(&[
        Node::new("Objects", Vec::new()),
        Node::new("Connections", Vec::new()),
    ])?;
    let imported = fbx::read(&bytes)?;
    assert_eq!(imported.soup.triangle_count(), 0);
    assert!(imported.ignored.is_empty());
    Ok(())
}

#[test]
fn a_reversed_triangle_keeps_each_corners_uv() -> Result<(), Setup> {
    let layer = layer(
        "LayerElementUV",
        "ByPolygonVertex",
        "Direct",
        vec![Node::new(
            "UV",
            vec![vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0].into()],
        )],
    );
    let mut scene = uv_triangle(TRIANGLE.to_vec(), vec![layer]);
    scene.objects[0] = model(1, "Tri", vec![vector("Lcl Scaling", [1.0, 1.0, -1.0])]);
    let imported = read(&scene)?;
    assert_eq!(triangles(&imported), [[v(0), v(2), v(1)]]);
    assert_eq!(
        corner_uvs(&imported),
        [uv(0.0, 0.0), uv(0.0, 1.0), uv(1.0, 0.0)]
    );
    Ok(())
}

#[test]
fn a_name_written_with_its_class_in_front_loses_the_class() -> Result<(), Setup> {
    let mut scene = one_triangle(Vec::new());
    scene
        .object(Node::new(
            "Material",
            vec![20_i64.into(), "Material::Felt".into(), "".into()],
        ))
        .connect(20, 1);
    assert_eq!(read(&scene)?.soup.materials().names(), ["Felt"]);
    Ok(())
}

#[test]
fn a_material_layer_that_cannot_be_read_is_parse() -> Result<(), Setup> {
    let materials = |mapping: &str, slots: Vec<i32>| {
        layer(
            "LayerElementMaterial",
            mapping,
            "IndexToDirect",
            vec![Node::new("Materials", vec![slots.into()])],
        )
    };
    let layers = [
        // Five slots for six quads.
        materials("ByPolygon", vec![0, 1, 0, 1, 0]),
        materials("ByPolygon", vec![0, 1, -1, 1, 0, 1]),
        materials("AllSame", Vec::new()),
        materials("ByPolygonVertex", vec![0; 24]),
        layer(
            "LayerElementMaterial",
            "AllSame",
            "IndexToDirect",
            Vec::new(),
        ),
    ];
    for layer in layers {
        let mut scene = two_material_cube(Vec::new());
        scene.objects[1] = cube_mesh(2).with([layer.clone()]);
        let detail = parse_detail(fbx::read(&scene.bytes()?))?;
        assert!(detail.contains("material"), "{layer:?}: {detail}");
    }
    Ok(())
}

#[test]
fn a_geometry_without_readable_control_points_or_polygons_is_parse() -> Result<(), Setup> {
    let geometry =
        |children: Vec<Node>| object("Geometry", 2, "Tri", "Geometry", "Mesh").with(children);
    let vertices = |v: Value| Node::new("Vertices", vec![v]);
    let polygons = || Node::new("PolygonVertexIndex", vec![TRIANGLE.to_vec().into()]);
    let cases = [
        geometry(vec![polygons()]),
        geometry(vec![vertices(AXES.to_vec().into())]),
        geometry(vec![vertices(AXES[..8].to_vec().into()), polygons()]),
        geometry(vec![vertices(Value::ArrI32(vec![0; 9])), polygons()]),
        geometry(vec![
            vertices(AXES.to_vec().into()),
            Node::new("PolygonVertexIndex", vec![AXES.to_vec().into()]),
        ]),
    ];
    for geometry in cases {
        let mut scene = one_triangle(Vec::new());
        scene.objects[1] = geometry.clone();
        let detail = parse_detail(fbx::read(&scene.bytes()?))?;
        assert!(detail.contains("Tri"), "{geometry:?}: {detail}");
    }
    Ok(())
}

#[test]
fn a_connection_or_object_that_cannot_be_read_is_parse() -> Result<(), Setup> {
    let mut dangling = one_triangle(Vec::new());
    dangling.connect(9, 1);
    let detail = parse_detail(fbx::read(&dangling.bytes()?))?;
    assert!(detail.contains('9'), "{detail}");
    let mut twice = one_triangle(Vec::new());
    twice.object(material(2, "Felt"));
    let detail = parse_detail(fbx::read(&twice.bytes()?))?;
    assert!(detail.contains('2'), "{detail}");
    let mut short = one_triangle(Vec::new());
    short
        .connections
        .push(Node::new("C", vec!["OO".into(), 2_i64.into()]));
    parse_detail(fbx::read(&short.bytes()?))?;
    let mut unnamed = one_triangle(Vec::new());
    unnamed.objects[0] = Node::new("Model", vec![1_i64.into()]);
    parse_detail(fbx::read(&unnamed.bytes()?))?;
    Ok(())
}

#[test]
fn a_property_that_cannot_be_read_is_parse() -> Result<(), Setup> {
    let short = Node::new(
        "P",
        vec![
            "Lcl Translation".into(),
            "Lcl Translation".into(),
            "".into(),
            "A".into(),
            1.0_f64.into(),
            2.0_f64.into(),
        ],
    );
    let words = Node::new(
        "P",
        vec![
            "RotationOrder".into(),
            "enum".into(),
            "".into(),
            "".into(),
            "XYZ".into(),
        ],
    );
    for property in [short, words] {
        let detail = parse_detail(fbx::read(&one_triangle(vec![property.clone()]).bytes()?))?;
        assert!(detail.contains("Tri"), "{property:?}: {detail}");
    }
    Ok(())
}

#[test]
fn a_property_written_in_another_number_type_reads_the_same() -> Result<(), Setup> {
    let as_f32 = Node::new(
        "P",
        vec![
            "Lcl Translation".into(),
            "Lcl Translation".into(),
            "".into(),
            "A".into(),
            1.0_f32.into(),
            2_i32.into(),
            3_i64.into(),
        ],
    );
    assert_eq!(
        read(&one_triangle(vec![as_f32]))?,
        read(&one_triangle(vec![vector(
            "Lcl Translation",
            [1.0, 2.0, 3.0]
        )]))?
    );
    Ok(())
}
