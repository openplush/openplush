//! Binary FBX bytes to the nodes the reader uses, through `fbxcel`'s pull parser.

use std::io::{self, Cursor, Read, Seek, SeekFrom};

pub(super) use fbxcel::low::v7400::AttributeValue as Value;
use fbxcel::pull_parser::Result as PullResult;
use fbxcel::pull_parser::any::AnyParser;
use fbxcel::pull_parser::v7400::{Event, LoadAttribute, Parser};

const MAGIC: &[u8] = b"Kaydara FBX Binary  \x00\x1a\x00";

/// The top-level nodes kept; the others are read past without loading their attributes.
const KEPT: [&str; 3] = ["GlobalSettings", "Objects", "Connections"];

/// One node of the file: its name, attributes and children in file order.
#[derive(Debug, Default)]
pub(super) struct Node {
    pub(super) name: String,
    pub(super) attrs: Vec<Value>,
    pub(super) children: Vec<Node>,
}

impl Node {
    /// The first child named `name`.
    pub(super) fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    pub(super) fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |c| c.name == name)
    }

    pub(super) fn attr(&self, i: usize) -> Option<&Value> {
        self.attrs.get(i)
    }

    /// The first attribute of the first child named `name`, if it is text.
    pub(super) fn child_text(&self, name: &str) -> Option<&str> {
        text(self.child(name)?.attr(0))
    }
}

pub(super) fn text(v: Option<&Value>) -> Option<&str> {
    match v? {
        Value::String(s) => Some(s),
        _ => None,
    }
}

pub(super) fn integer(v: Option<&Value>) -> Option<i64> {
    match *v? {
        Value::I16(i) => Some(i.into()),
        Value::I32(i) => Some(i.into()),
        Value::I64(i) => Some(i),
        _ => None,
    }
}

pub(super) fn number(v: Option<&Value>) -> Option<f64> {
    match *v? {
        Value::F32(x) => Some(x.into()),
        Value::F64(x) => Some(x),
        _ => integer(v).map(|i| i as f64),
    }
}

pub(super) fn floats(v: Option<&Value>) -> Option<Vec<f64>> {
    match v? {
        Value::ArrF64(a) => Some(a.clone()),
        Value::ArrF32(a) => Some(a.iter().map(|&x| f64::from(x)).collect()),
        _ => None,
    }
}

pub(super) fn ints(v: Option<&Value>) -> Option<&[i32]> {
    match v? {
        Value::ArrI32(a) => Some(a),
        _ => None,
    }
}

/// The kept top-level nodes of a binary FBX 7.x file, in file order.
pub(super) fn parse(bytes: &[u8]) -> Result<Vec<Node>, String> {
    if !bytes.starts_with(MAGIC) {
        return Err(if is_ascii_fbx(bytes) {
            "the file is ASCII FBX, which is not read: export it as binary FBX".into()
        } else {
            "the file does not start with the binary FBX header".into()
        });
    }
    let version = bytes
        .get(MAGIC.len()..MAGIC.len() + 4)
        .and_then(|v| v.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or("the binary FBX header is cut short")?;
    if !(7000..8000).contains(&version) {
        return Err(format!(
            "binary FBX version {version} is not read: only version 7 is"
        ));
    }
    let reader = Bounded(Cursor::new(bytes));
    match AnyParser::from_seekable_reader(reader).map_err(|e| e.to_string())? {
        AnyParser::V7400(parser) => nodes(parser).map_err(|e| match e.position() {
            Some(at) => format!("byte {}: {e}", at.byte_pos()),
            None => e.to_string(),
        }),
        _ => Err(format!("binary FBX version {version} is not read")),
    }
}

/// ASCII FBX starts with the SDK's `; FBX` comment line, or with its first node.
fn is_ascii_fbx(bytes: &[u8]) -> bool {
    let start = bytes.trim_ascii_start();
    start.starts_with(b"; FBX") || start.starts_with(b"FBXHeaderExtension:")
}

/// The bytes, refusing a seek past their end. The parser skips unread attributes by
/// seeking to the end the file declares for them; a declared end past the bytes then fails
/// as a read error here, before the parser adds the distance to its own position counter,
/// which panics on overflow where `usize` is 32 bits. The seekable path is used because
/// `fbxcel` 0.9.0's plain skip does not advance that counter.
struct Bounded<'a>(Cursor<&'a [u8]>);

impl Read for Bounded<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Seek for Bounded<'_> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let from = self.0.position();
        let at = self.0.seek(to)?;
        if at > self.0.get_ref().len() as u64 {
            self.0.set_position(from);
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "a node or attribute ends past the end of the file",
            ));
        }
        Ok(at)
    }
}

/// Every node is visited with `next_event`: the parser's own skip trusts the node's end
/// offset and panics on one that points backwards.
fn nodes<R: Read>(mut parser: Parser<R>) -> PullResult<Vec<Node>> {
    let mut open = vec![Node::default()];
    let mut skipping = 0_usize;
    loop {
        match parser.next_event()? {
            Event::StartNode(start) => {
                if skipping > 0 || (open.len() == 1 && !KEPT.contains(&start.name())) {
                    skipping += 1;
                    continue;
                }
                let name = start.name().to_owned();
                let mut attributes = start.attributes();
                let mut attrs = Vec::new();
                while let Some(value) = attributes.load_next(Loader)? {
                    attrs.push(value);
                }
                open.push(Node {
                    name,
                    attrs,
                    children: Vec::new(),
                });
            }
            Event::EndNode if skipping > 0 => skipping -= 1,
            Event::EndNode => {
                // INVARIANT: the parser ends only nodes it started, and the file's own top
                // level ends with `EndFbx`, so a node is open below the root here.
                if let Some(node) = open.pop()
                    && let Some(parent) = open.last_mut()
                {
                    parent.children.push(node);
                }
            }
            Event::EndFbx(_) => break,
        }
    }
    Ok(open.swap_remove(0).children)
}

/// Loads each attribute as its value, except that a binary attribute (an embedded file,
/// which the reader never uses) is left unread, and a string is read without reserving
/// the length the file declares for it, which the bytes may not hold.
#[derive(Debug)]
struct Loader;

impl LoadAttribute for Loader {
    type Output = Value;

    fn expecting(&self) -> String {
        "any attribute".into()
    }

    fn load_bool(self, v: bool) -> PullResult<Value> {
        Ok(Value::Bool(v))
    }

    fn load_i16(self, v: i16) -> PullResult<Value> {
        Ok(Value::I16(v))
    }

    fn load_i32(self, v: i32) -> PullResult<Value> {
        Ok(Value::I32(v))
    }

    fn load_i64(self, v: i64) -> PullResult<Value> {
        Ok(Value::I64(v))
    }

    fn load_f32(self, v: f32) -> PullResult<Value> {
        Ok(Value::F32(v))
    }

    fn load_f64(self, v: f64) -> PullResult<Value> {
        Ok(Value::F64(v))
    }

    fn load_seq_bool(
        self,
        v: impl Iterator<Item = PullResult<bool>>,
        _: usize,
    ) -> PullResult<Value> {
        Ok(Value::ArrBool(v.collect::<PullResult<_>>()?))
    }

    fn load_seq_i32(self, v: impl Iterator<Item = PullResult<i32>>, _: usize) -> PullResult<Value> {
        Ok(Value::ArrI32(v.collect::<PullResult<_>>()?))
    }

    fn load_seq_i64(self, v: impl Iterator<Item = PullResult<i64>>, _: usize) -> PullResult<Value> {
        Ok(Value::ArrI64(v.collect::<PullResult<_>>()?))
    }

    fn load_seq_f32(self, v: impl Iterator<Item = PullResult<f32>>, _: usize) -> PullResult<Value> {
        Ok(Value::ArrF32(v.collect::<PullResult<_>>()?))
    }

    fn load_seq_f64(self, v: impl Iterator<Item = PullResult<f64>>, _: usize) -> PullResult<Value> {
        Ok(Value::ArrF64(v.collect::<PullResult<_>>()?))
    }

    fn load_binary(self, _: impl Read, _: u64) -> PullResult<Value> {
        Ok(Value::Binary(Vec::new()))
    }

    fn load_string(self, mut reader: impl Read, _: u64) -> PullResult<Value> {
        let mut s = String::new();
        reader.read_to_string(&mut s)?;
        Ok(Value::String(s))
    }
}
