//! The file's objects, their object-to-object connections, and the walk over the models
//! under the document's root.

use std::collections::{BTreeMap, BTreeSet};

use openplush_mesh::MaterialIndex;

use super::transform::{self, Properties};
use super::tree::{Node, integer, text};
use crate::affine::{Mat, mul};

/// The id every top-level model is connected to.
const ROOT: i64 = 0;

/// One child of `Objects`.
#[derive(Clone, Copy)]
pub(super) struct Object<'a> {
    pub(super) node: &'a Node,
    pub(super) id: i64,
    /// Its name without the class FBX joins to it.
    pub(super) name: &'a str,
    /// Its third attribute, such as `Mesh` for a geometry or `Skin` for a deformer.
    pub(super) subclass: &'a str,
}

pub(super) struct Document<'a> {
    /// In file order.
    objects: Vec<Object<'a>>,
    /// Each parent's children, as positions in `objects`, in connection order.
    children: BTreeMap<i64, Vec<usize>>,
    settings: Option<&'a Node>,
    /// Each material object's index into [`Document::material_names`].
    materials: BTreeMap<i64, MaterialIndex>,
}

impl<'a> Document<'a> {
    /// The document of the file's top-level nodes.
    pub(super) fn new(top: &'a [Node]) -> Result<Self, String> {
        let find = |name: &str| top.iter().find(|n| n.name == name);
        let objects = find("Objects").map_or(Ok(Vec::new()), |o| {
            o.children.iter().map(object).collect::<Result<Vec<_>, _>>()
        })?;
        let mut by_id = BTreeMap::new();
        for (i, o) in objects.iter().enumerate() {
            if by_id.insert(o.id, i).is_some() {
                return Err(format!("object id {} is used by two objects", o.id));
            }
        }
        let children = connections(find("Connections"), &by_id)?;
        let mut materials = BTreeMap::new();
        for (k, m) in objects
            .iter()
            .filter(|o| o.node.name == "Material")
            .enumerate()
        {
            let k = u32::try_from(k).map_err(|_| "too many materials")?;
            materials.insert(m.id, MaterialIndex::new(k));
        }
        Ok(Self {
            objects,
            children,
            settings: find("GlobalSettings"),
            materials,
        })
    }

    /// Every model under the root, depth first with children in connection order, and its
    /// world transform in Y-up axes.
    pub(super) fn models(&self) -> Result<Vec<(Object<'a>, Mat)>, String> {
        let axes = transform::axes(Properties::of(self.settings))
            .map_err(|e| format!("the axis system: {e}"))?;
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        let mut stack: Vec<(Object<'a>, Mat)> =
            self.connected(ROOT, "Model").map(|m| (m, axes)).collect();
        stack.reverse();
        while let Some((model, parent)) = stack.pop() {
            if !seen.insert(model.id) {
                return Err(format!(
                    "model \"{}\" is reached twice, so the model hierarchy is not a tree",
                    model.name
                ));
            }
            let local = transform::local(Properties::of(Some(model.node)))
                .map_err(|e| format!("model \"{}\": {e}", model.name))?;
            let world = mul(&parent, &local);
            let first = stack.len();
            stack.extend(self.connected(model.id, "Model").map(|c| (c, world)));
            stack[first..].reverse();
            out.push((model, world));
        }
        Ok(out)
    }

    /// The objects named `kind` (`Model`, `Geometry`, …) connected to `parent`, in
    /// connection order.
    pub(super) fn connected<'s>(
        &'s self,
        parent: i64,
        kind: &'s str,
    ) -> impl Iterator<Item = Object<'a>> + 's {
        // INVARIANT: `connections` stored only positions it found in `objects`.
        let objects = &self.objects;
        self.children
            .get(&parent)
            .into_iter()
            .flatten()
            .map(move |&i| objects[i])
            .filter(move |o| o.node.name == kind)
    }

    /// The materials connected to `model`, in connection order: its material slots.
    pub(super) fn slots(&self, model: &Object<'_>) -> Vec<MaterialIndex> {
        self.connected(model.id, "Material")
            .filter_map(|m| self.materials.get(&m.id).copied())
            .collect()
    }

    /// Every material's name in file order; an unnamed one is `material N`.
    pub(super) fn material_names(&self) -> Vec<String> {
        self.objects
            .iter()
            .filter(|o| o.node.name == "Material")
            .enumerate()
            .map(|(k, m)| match m.name {
                "" => format!("material {k}"),
                name => name.to_owned(),
            })
            .collect()
    }

    pub(super) fn animation_stacks(&self) -> impl Iterator<Item = Object<'a>> + '_ {
        self.objects
            .iter()
            .copied()
            .filter(|o| o.node.name == "AnimationStack")
    }
}

fn object(node: &Node) -> Result<Object<'_>, String> {
    let id = integer(node.attr(0)).ok_or_else(|| format!("a {} object has no id", node.name))?;
    let raw = text(node.attr(1)).ok_or_else(|| format!("{} object {id} has no name", node.name))?;
    // Binary FBX writes `name\0\x01class`; some writers put `class::name` instead.
    let name = match raw.split_once("\0\x01") {
        Some((name, _)) => name,
        None => raw.split_once("::").map_or(raw, |(_, name)| name),
    };
    Ok(Object {
        node,
        id,
        name,
        subclass: text(node.attr(2)).unwrap_or_default(),
    })
}

/// The object-to-object (`OO`) connections as each parent's children. Property
/// connections are not read.
fn connections(
    node: Option<&Node>,
    by_id: &BTreeMap<i64, usize>,
) -> Result<BTreeMap<i64, Vec<usize>>, String> {
    let mut children: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (k, c) in node
        .into_iter()
        .flat_map(|n| n.children_named("C"))
        .enumerate()
    {
        let (Some(kind), Some(child), Some(parent)) =
            (text(c.attr(0)), integer(c.attr(1)), integer(c.attr(2)))
        else {
            return Err(format!(
                "connection {k} does not name its kind, child and parent"
            ));
        };
        if kind != "OO" {
            continue;
        }
        let missing =
            |id: i64| format!("connection {k} names object {id}, which is not in Objects");
        let &at = by_id.get(&child).ok_or_else(|| missing(child))?;
        if parent != ROOT && !by_id.contains_key(&parent) {
            return Err(missing(parent));
        }
        children.entry(parent).or_default().push(at);
    }
    Ok(children)
}
