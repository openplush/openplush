//! Proposals: every remedy the engine can carry out is a seam-set delta.
//! Accepting one inserts its edges into the
//! `SeamSet`; the mesh never changes.

use std::collections::BTreeMap;

use openplush_mesh::EdgeId;

use crate::{ChartFailure, ChartId, SeamKind, SeamsError};

/// What a proposal is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ProposalKind {
    /// Extra seams that make a chart a topological disk.
    CloseChart {
        /// The chart that is not a disk.
        chart: ChartId,
        /// Why not.
        failure: ChartFailure,
    },
    /// A dart to relieve distortion.
    Dart {
        /// The chart it relieves.
        chart: ChartId,
    },
    /// Split a piece whose nap is incoherent. Names the chart, because
    /// pieces (`openplush-pattern`) sit above this crate.
    SplitPiece {
        /// The chart whose piece splits.
        chart: ChartId,
    },
    /// Any other seam the engine suggests.
    Seam {
        /// Why.
        reason: SeamReason,
    },
}

impl ProposalKind {
    /// The byte `ProposalId` hashes for this kind: 0-3 in declaration order. The
    /// chart and failure are deliberately not hashed: they can change while the
    /// edit stays the same.
    const fn hash_byte(&self) -> u8 {
        match self {
            Self::CloseChart { .. } => 0,
            Self::Dart { .. } => 1,
            Self::SplitPiece { .. } => 2,
            Self::Seam { .. } => 3,
        }
    }
}

/// Why a [`ProposalKind::Seam`] is suggested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SeamReason {
    /// A concave region with no seam through it.
    Crease,
    /// Feature 002: an accepted seam-placement hint.
    SeamHint,
}

/// A proposal's identity: a 64-bit FNV-1a hash of its edit, so the same fix
/// proposed again (after an unrelated recompute) has the same ID. Stable for one
/// repaired mesh. On the wire it is 16 lowercase hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProposalId(u64);

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

impl ProposalId {
    /// Wraps a raw value, as read back from a stored or transmitted ID.
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// The raw value.
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// The ID of an edit: FNV-1a over the kind byte, then each entry of `add` in
    /// key order as the edge index (4 bytes little-endian) and the seam kind byte.
    fn of(kind: &ProposalKind, add: &BTreeMap<EdgeId, SeamKind>) -> Self {
        let mut h = FNV_OFFSET;
        let mut eat = |byte: u8| h = (h ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
        eat(kind.hash_byte());
        for (edge, seam) in add {
            edge.index().to_le_bytes().into_iter().for_each(&mut eat);
            eat(seam.hash_byte());
        }
        Self(h)
    }

    /// The wire form: 16 lowercase hex digits. (A `u64` as a JSON number loses
    /// precision above 2^53.)
    pub fn to_hex(self) -> String {
        format!("{:016x}", self.0)
    }

    /// Parses the wire form. Anything but exactly 16 lowercase hex digits is
    /// refused, so an ID has one spelling.
    pub fn from_hex(text: &str) -> Result<Self, SeamsError> {
        let ok = text.len() == 16
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        let bad = || SeamsError::BadProposalId {
            text: text.to_owned(),
        };
        if !ok {
            return Err(bad());
        }
        u64::from_str_radix(text, 16).map(Self).map_err(|_| bad())
    }
}

/// A remedy the engine can carry out: seam edges to add.
/// Built only through [`Proposal::new`], so the ID always matches the edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    id: ProposalId,
    kind: ProposalKind,
    add: BTreeMap<EdgeId, SeamKind>,
    explanation: String,
}

impl Proposal {
    /// A proposal to add `add`'s seam edges, for `kind`'s reason, explained to the
    /// maker in plain language. Errors with [`SeamsError::EmptyProposal`] if `add`
    /// is empty. (That no edge is already a seam is checked against the `SeamSet`
    /// by whoever proposes.)
    pub fn new(
        kind: ProposalKind,
        add: BTreeMap<EdgeId, SeamKind>,
        explanation: String,
    ) -> Result<Self, SeamsError> {
        if add.is_empty() {
            return Err(SeamsError::EmptyProposal);
        }
        Ok(Self {
            id: ProposalId::of(&kind, &add),
            kind,
            add,
            explanation,
        })
    }

    /// The content-derived ID.
    pub fn id(&self) -> ProposalId {
        self.id
    }

    /// What the proposal is for.
    pub fn kind(&self) -> &ProposalKind {
        &self.kind
    }

    /// The seam edges it adds, never empty.
    pub fn add(&self) -> &BTreeMap<EdgeId, SeamKind> {
        &self.add
    }

    /// Plain language: what it fixes and what it costs.
    pub fn explanation(&self) -> &str {
        &self.explanation
    }
}
