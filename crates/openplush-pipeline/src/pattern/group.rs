//! Pieces grouped into templates: one shape, as printed or flipped, whose seams sew to the same
//! templates edge for edge, and the near mirror images that stay apart.

use std::collections::{BTreeMap, BTreeSet};

use openplush_geom::epsilon::SeamParity;
use openplush_geom::units::Mm;
use openplush_pattern::Outline;
use openplush_pattern::dedupe::{Facing, Fit, NEAR_MM, fit};
use openplush_seams::SeamChainId;

/// What sews along one edge of a sewing line: its seam's lead chain, and the pieces, by index,
/// across it. Unsewn edges have neither.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct EdgeSeam {
    pub(super) chain: Option<SeamChainId>,
    pub(super) partners: BTreeSet<usize>,
}

/// A piece's place in its group: the template's index, its own for a template, and how it lies
/// on the template, `None` for the template itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Member {
    pub(super) template: usize,
    pub(super) fit: Option<Fit>,
}

/// Two templates that are mirror images to within `NEAR_MM` but further apart than the
/// seam-parity floor, and whether their seams sew to the same templates edge for edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Near {
    pub(super) a: usize,
    pub(super) b: usize,
    pub(super) gap_mm: f64,
    pub(super) seams_mirror: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Grouping {
    pub(super) members: Vec<Member>,
    pub(super) near: Vec<Near>,
}

/// Each piece, in order, joins the first earlier template it fits within the seam-parity floor,
/// as printed before flipped, whose seams sew to the same templates edge for edge; otherwise it
/// is a template. Which templates the seams sew to depends on the grouping, so it starts from
/// shape alone and regroups until nothing changes; each pass can only split groups, so it ends.
/// `edges[i]` has one entry per edge of `outlines[i]`.
pub(super) fn group(outlines: &[&Outline], edges: &[Vec<EdgeSeam>]) -> Grouping {
    let same = SeamParity::DEFAULT
        .allowed(Mm::try_new(0.0).unwrap_or_else(|_| unreachable!()))
        .get();
    let mut members: Option<Vec<Member>> = None;
    loop {
        let next = pass(outlines, edges, members.as_deref(), same);
        if members.as_ref() == Some(&next) {
            break;
        }
        members = Some(next);
    }
    // INVARIANT: the loop runs at least once.
    let members = members.unwrap_or_else(|| unreachable!());
    let near = near_mirrors(outlines, edges, &members, same);
    Grouping { members, near }
}

/// One grouping pass; with no earlier grouping, seams are not compared.
fn pass(
    outlines: &[&Outline],
    edges: &[Vec<EdgeSeam>],
    earlier: Option<&[Member]>,
    same: f64,
) -> Vec<Member> {
    let mut members: Vec<Member> = Vec::with_capacity(outlines.len());
    for (i, outline) in outlines.iter().enumerate() {
        let templates = members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.fit.is_none())
            .map(|(t, _)| t);
        let joined = templates
            .flat_map(|t| [(t, Facing::AsPrinted), (t, Facing::Flipped)])
            .find_map(|(t, facing)| {
                let f = fit(outlines[t], outline, facing)?;
                let seams = earlier.is_none_or(|e| seams_match(edges, e, t, i, &f));
                (f.gap_mm() <= same && seams).then_some(Member {
                    template: t,
                    fit: Some(f),
                })
            });
        members.push(joined.unwrap_or(Member {
            template: i,
            fit: None,
        }));
    }
    members
}

/// Whether every edge of piece `t` and the edge of piece `i` it lands on through `f` sew along a
/// seam or not alike, to pieces of the same templates under `members`.
fn seams_match(edges: &[Vec<EdgeSeam>], members: &[Member], t: usize, i: usize, f: &Fit) -> bool {
    let templates = |partners: &BTreeSet<usize>| -> BTreeSet<usize> {
        partners.iter().map(|&p| members[p].template).collect()
    };
    edges[t].iter().enumerate().all(|(e, ours)| {
        let theirs = &edges[i][f.edge(e)];
        ours.chain.is_some() == theirs.chain.is_some()
            && templates(&ours.partners) == templates(&theirs.partners)
    })
}

/// Every pair of templates, lower first, that are mirror images to within `NEAR_MM` and further
/// apart than `same`.
fn near_mirrors(
    outlines: &[&Outline],
    edges: &[Vec<EdgeSeam>],
    members: &[Member],
    same: f64,
) -> Vec<Near> {
    let templates: Vec<usize> = (0..members.len())
        .filter(|&i| members[i].fit.is_none())
        .collect();
    let mut out = Vec::new();
    for (k, &a) in templates.iter().enumerate() {
        for &b in &templates[k + 1..] {
            let Some(f) = fit(outlines[a], outlines[b], Facing::Flipped) else {
                continue;
            };
            if f.gap_mm() > same && f.gap_mm() <= NEAR_MM {
                out.push(Near {
                    a,
                    b,
                    gap_mm: f.gap_mm(),
                    seams_mirror: seams_match(edges, members, a, b, &f),
                });
            }
        }
    }
    out
}

/// How the seams of grouped pieces correspond. Without a grouping every chain is its own and
/// every piece its own template.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Alike {
    root: BTreeMap<SeamChainId, SeamChainId>,
    names: BTreeMap<(usize, SeamChainId), Vec<SeamChainId>>,
    template: Vec<usize>,
}

impl Alike {
    /// The chain `chain` shares its notch marks with: the lowest of the chains matched to it
    /// through the grouping.
    pub(super) fn of(&self, chain: SeamChainId) -> SeamChainId {
        let mut at = chain;
        while let Some(&up) = self.root.get(&at) {
            at = up;
        }
        at
    }

    /// The names to print for `chain` on piece `piece`: its own, then those of the chains on its
    /// members' matching edges, in member order, each once.
    pub(super) fn names(&self, piece: usize, chain: SeamChainId) -> Vec<SeamChainId> {
        let mut out = vec![chain];
        for &c in self.names.get(&(piece, chain)).into_iter().flatten() {
            if !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }

    /// The template piece `piece` is cut from.
    pub(super) fn template_of(&self, piece: usize) -> usize {
        self.template.get(piece).copied().unwrap_or(piece)
    }
}

/// The seams of `grouping`'s members matched, edge for edge, to their templates'.
pub(super) fn alike(grouping: &Grouping, edges: &[Vec<EdgeSeam>]) -> Alike {
    let mut out = Alike {
        template: grouping.members.iter().map(|m| m.template).collect(),
        ..Alike::default()
    };
    for (i, member) in grouping.members.iter().enumerate() {
        let Some(f) = member.fit else {
            continue;
        };
        let t = member.template;
        for (e, ours) in edges[t].iter().enumerate() {
            let (Some(a), Some(b)) = (ours.chain, edges[i][f.edge(e)].chain) else {
                continue;
            };
            out.names.entry((t, a)).or_default().push(b);
            let (ra, rb) = (out.of(a), out.of(b));
            // The lower root stays the root, so the result does not depend on the order.
            if ra != rb {
                out.root.insert(ra.max(rb), ra.min(rb));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
