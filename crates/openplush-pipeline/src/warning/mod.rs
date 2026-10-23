//! Warnings: the output is valid but may sew badly. Every warning says where,
//! why, and what to do about it, and never blocks export.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use openplush_mesh::{ComponentId, EdgeId, FaceId};
use openplush_pattern::PieceId;
use openplush_seams::{Proposal, SeamChainId};

pub(crate) mod summary;
#[cfg(test)]
mod tests;

/// Plain-language text with at least one non-whitespace character.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonBlank(String);

impl NonBlank {
    /// `None` if `text` is empty or only whitespace; otherwise kept as given.
    pub fn new(text: String) -> Option<Self> {
        (!text.trim().is_empty()).then_some(Self(text))
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a warning reports. Each stage adds the codes it raises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WarningCode {
    /// The file has parts the pattern does not follow: skins, morph targets, deformers,
    /// animations, point and line primitives, or geometry that is not a polygon mesh.
    ImportIgnored,
    /// The finished size is outside the supported range of the longest dimension.
    SizeOutOfRange,
    /// Declared first, sorts last: proves order follows the string.
    #[cfg(test)]
    TestZulu,
    /// Declared last, sorts first.
    #[cfg(test)]
    TestAlpha,
}

impl WarningCode {
    /// The stable kebab-case form. Never changes once released: acknowledgements
    /// are stored by it.
    pub const fn as_str(self) -> &'static str {
        // Must stay injective: warnings order and deduplicate by this string.
        match self {
            Self::ImportIgnored => "import-ignored",
            Self::SizeOutOfRange => "size-out-of-range",
            #[cfg(test)]
            Self::TestZulu => "test-zulu",
            #[cfg(test)]
            Self::TestAlpha => "test-alpha",
        }
    }
}

/// Where a warning applies. Orders by variant as declared, then by ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum WarningTarget {
    /// A mesh face.
    Face(FaceId),
    /// A mesh edge.
    Edge(EdgeId),
    /// A seam chain.
    Chain(SeamChainId),
    /// A pattern piece.
    Piece(PieceId),
    /// A connected component of the repaired mesh.
    Component(ComponentId),
    /// The whole project.
    Project,
}

/// What to do about a warning. Never absent: a warning that only describes the
/// problem cannot be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Remedy {
    /// A seam edit the engine can make, offered preview-then-accept.
    Actionable {
        /// The edit.
        proposal: Proposal,
    },
    /// A material or sizing change the engine will not make.
    Advisory {
        /// The change, in plain language.
        what_to_change: NonBlank,
    },
}

impl Ord for Remedy {
    /// Actionable before Advisory. Proposals compare by ID, then by the rest of
    /// their content, so two with one ID (the same edit for another chart, or a
    /// hash collision) still order and are both kept.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Actionable { proposal: a }, Self::Actionable { proposal: b }) => {
                proposal_key(a).cmp(&proposal_key(b))
            }
            (Self::Actionable { .. }, Self::Advisory { .. }) => Ordering::Less,
            (Self::Advisory { .. }, Self::Actionable { .. }) => Ordering::Greater,
            (Self::Advisory { what_to_change: a }, Self::Advisory { what_to_change: b }) => {
                a.cmp(b)
            }
        }
    }
}

impl PartialOrd for Remedy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Every field of a proposal, ID first, so the order agrees with `Eq`.
fn proposal_key(p: &Proposal) -> impl Ord + '_ {
    (p.id(), p.kind(), p.add(), p.explanation())
}

/// A located, explained problem with its remedy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    code: WarningCode,
    target: WarningTarget,
    explanation: NonBlank,
    remedy: Remedy,
}

impl Warning {
    /// A warning; `explanation` says why it was flagged.
    pub fn new(
        code: WarningCode,
        target: WarningTarget,
        explanation: NonBlank,
        remedy: Remedy,
    ) -> Self {
        Self {
            code,
            target,
            explanation,
            remedy,
        }
    }

    /// What it reports.
    pub fn code(&self) -> WarningCode {
        self.code
    }

    /// Where it applies.
    pub fn target(&self) -> WarningTarget {
        self.target
    }

    /// Why it was flagged, in plain language.
    pub fn explanation(&self) -> &str {
        self.explanation.as_str()
    }

    /// What to do about it.
    pub fn remedy(&self) -> &Remedy {
        &self.remedy
    }
}

impl Ord for Warning {
    /// By code string (so a new code never reorders a printed summary), then
    /// target, then remedy, then explanation.
    fn cmp(&self, other: &Self) -> Ordering {
        self.code
            .as_str()
            .cmp(other.code.as_str())
            .then_with(|| self.target.cmp(&other.target))
            .then_with(|| self.remedy.cmp(&other.remedy))
            .then_with(|| self.explanation.cmp(&other.explanation))
    }
}

impl PartialOrd for Warning {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Warnings in one total order, whatever order they were raised in; equal
/// warnings are kept once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WarningSet {
    items: BTreeSet<Warning>,
}

impl WarningSet {
    /// Adds `warning` unless an equal one is already present.
    pub fn insert(&mut self, warning: Warning) {
        self.items.insert(warning);
    }

    /// The warnings, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Warning> {
        self.items.iter()
    }
}
