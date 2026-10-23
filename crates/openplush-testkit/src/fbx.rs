//! Binary FBX 7.4 files built in code for reader tests, written with `fbxcel`'s writer.

use std::io::Cursor;

use fbxcel::low::FbxVersion;
use fbxcel::low::v7400::ArrayAttributeEncoding;
use fbxcel::writer::v7400::binary::{FbxFooter, Writer};

pub use fbxcel::low::v7400::AttributeValue as Value;
pub use fbxcel::writer::v7400::binary::Error;

/// One FBX node: its name, its attributes, and its children in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The node's name, such as `Model` or `Vertices`.
    pub name: String,
    /// Its attributes in order.
    pub attrs: Vec<Value>,
    /// Its child nodes in order.
    pub children: Vec<Node>,
}

impl Node {
    /// A node with no children.
    pub fn new(name: &str, attrs: Vec<Value>) -> Self {
        Self {
            name: name.into(),
            attrs,
            children: Vec::new(),
        }
    }

    /// This node with `children` appended.
    #[must_use]
    pub fn with(mut self, children: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(children);
        self
    }
}

/// A scene: `GlobalSettings` properties, objects, and object-to-object connections.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// `P` nodes under `GlobalSettings`; none writes no `GlobalSettings` at all.
    pub settings: Vec<Node>,
    /// The children of `Objects`, in order.
    pub objects: Vec<Node>,
    /// The `C` nodes of `Connections`, in order.
    pub connections: Vec<Node>,
}

impl Scene {
    /// Adds `object` under `Objects`.
    pub fn object(&mut self, object: Node) -> &mut Self {
        self.objects.push(object);
        self
    }

    /// Connects `child` to `parent` object to object; 0 is the document's root.
    pub fn connect(&mut self, child: i64, parent: i64) -> &mut Self {
        self.connections.push(Node::new(
            "C",
            vec!["OO".into(), child.into(), parent.into()],
        ));
        self
    }

    /// Sets the file's axis system: each `[axis, sign]`.
    pub fn axes(&mut self, up: [i32; 2], front: [i32; 2], coord: [i32; 2]) -> &mut Self {
        for (name, [axis, sign]) in [("Up", up), ("Front", front), ("Coord", coord)] {
            self.settings.push(int(&format!("{name}Axis"), axis));
            self.settings.push(int(&format!("{name}AxisSign"), sign));
        }
        self
    }

    /// The scene as binary FBX 7.4 bytes.
    ///
    /// # Errors
    /// The writer's, for a node it cannot write.
    pub fn bytes(&self) -> Result<Vec<u8>, Error> {
        let mut top = vec![
            Node::new("FBXHeaderExtension", Vec::new())
                .with([Node::new("FBXVersion", vec![7400_i32.into()])]),
        ];
        if !self.settings.is_empty() {
            top.push(
                Node::new("GlobalSettings", Vec::new()).with([properties(self.settings.clone())]),
            );
        }
        top.push(Node::new("Objects", Vec::new()).with(self.objects.clone()));
        top.push(Node::new("Connections", Vec::new()).with(self.connections.clone()));
        binary(&top)
    }
}

/// The bytes of a binary FBX 7.4 file whose top-level nodes are `top`. Arrays are
/// zlib-compressed, as exporters write them.
///
/// # Errors
/// The writer's, for a node it cannot write.
pub fn binary(top: &[Node]) -> Result<Vec<u8>, Error> {
    let mut writer = Writer::new(Cursor::new(Vec::new()), FbxVersion::V7_4)?;
    for node in top {
        write(&mut writer, node)?;
    }
    Ok(writer.finalize(&FbxFooter::default())?.into_inner())
}

fn write(writer: &mut Writer<Cursor<Vec<u8>>>, node: &Node) -> Result<(), Error> {
    let zlib = Some(ArrayAttributeEncoding::Zlib);
    let mut a = writer.new_node(&node.name)?;
    for value in &node.attrs {
        match value {
            Value::Bool(v) => a.append_bool(*v)?,
            Value::I16(v) => a.append_i16(*v)?,
            Value::I32(v) => a.append_i32(*v)?,
            Value::I64(v) => a.append_i64(*v)?,
            Value::F32(v) => a.append_f32(*v)?,
            Value::F64(v) => a.append_f64(*v)?,
            Value::ArrBool(v) => a.append_arr_bool_from_iter(zlib, v.iter().copied())?,
            Value::ArrI32(v) => a.append_arr_i32_from_iter(zlib, v.iter().copied())?,
            Value::ArrI64(v) => a.append_arr_i64_from_iter(zlib, v.iter().copied())?,
            Value::ArrF32(v) => a.append_arr_f32_from_iter(zlib, v.iter().copied())?,
            Value::ArrF64(v) => a.append_arr_f64_from_iter(zlib, v.iter().copied())?,
            Value::Binary(v) => a.append_binary_direct(v)?,
            Value::String(v) => a.append_string_direct(v)?,
        }
    }
    for child in &node.children {
        write(writer, child)?;
    }
    writer.close_node()
}

/// An object node: `kind` (`Model`, `Geometry`, …) with its id, its name joined to its
/// class as binary FBX writes it (`name\0\x01class`), and its subclass.
pub fn object(kind: &str, id: i64, name: &str, class: &str, subclass: &str) -> Node {
    Node::new(
        kind,
        vec![
            id.into(),
            format!("{name}\0\x01{class}").into(),
            subclass.into(),
        ],
    )
}

/// A `Mesh` model with `props` as its `Properties70`.
pub fn model(id: i64, name: &str, props: Vec<Node>) -> Node {
    object("Model", id, name, "Model", "Mesh").with([properties(props)])
}

/// A `Mesh` geometry with control points `vertices` (x, y, z each) and `polygons` as
/// `PolygonVertexIndex` writes them (a polygon's last index `i` as `-i - 1`).
pub fn mesh(id: i64, name: &str, vertices: Vec<f64>, polygons: Vec<i32>) -> Node {
    object("Geometry", id, name, "Geometry", "Mesh").with([
        Node::new("Vertices", vec![vertices.into()]),
        Node::new("PolygonVertexIndex", vec![polygons.into()]),
    ])
}

/// A material object.
pub fn material(id: i64, name: &str) -> Node {
    object("Material", id, name, "Material", "")
}

/// A layer element (`LayerElementUV`, `LayerElementMaterial`, …) with its mapping and
/// reference types, holding `data`.
pub fn layer(kind: &str, mapping: &str, reference: &str, data: Vec<Node>) -> Node {
    Node::new(kind, vec![0_i32.into()])
        .with([
            Node::new("Version", vec![101_i32.into()]),
            Node::new("Name", vec!["".into()]),
            Node::new("MappingInformationType", vec![mapping.into()]),
            Node::new("ReferenceInformationType", vec![reference.into()]),
        ])
        .with(data)
}

/// A `Properties70` node holding `props`.
pub fn properties(props: Vec<Node>) -> Node {
    Node::new("Properties70", Vec::new()).with(props)
}

/// A three-number property, such as `Lcl Translation`.
pub fn vector(name: &str, [x, y, z]: [f64; 3]) -> Node {
    Node::new(
        "P",
        vec![
            name.into(),
            name.into(),
            "".into(),
            "A".into(),
            x.into(),
            y.into(),
            z.into(),
        ],
    )
}

/// An integer property, such as `UpAxis` or `RotationOrder`.
pub fn int(name: &str, value: i32) -> Node {
    Node::new(
        "P",
        vec![
            name.into(),
            "int".into(),
            "Integer".into(),
            "".into(),
            value.into(),
        ],
    )
}
