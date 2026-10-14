/// What a seam edge is for (data-model section 4). Each kind's byte in
/// `ProposalId`'s derivation (section 13a) is fixed by `hash_byte` and pinned by
/// tests; a new kind gets the next unused byte, and no byte ever changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SeamKind {
    /// An ordinary cut between two pieces.
    Cut,
    /// A dart: a seam ending inside a piece.
    Dart,
    /// An opening left unsewn (for stuffing or turning).
    Opening,
    /// A boundary between two fabrics.
    FabricBoundary,
    /// Required by repair (a non-manifold edge split into two); never removable.
    Mandatory,
}

impl SeamKind {
    /// The byte `ProposalId` hashes for this kind: its declaration index.
    pub(crate) const fn hash_byte(self) -> u8 {
        match self {
            Self::Cut => 0,
            Self::Dart => 1,
            Self::Opening => 2,
            Self::FabricBoundary => 3,
            Self::Mandatory => 4,
        }
    }
}

/// Why a chart cannot be flattened as one piece (data-model section 4, engine-api
/// section 5): it must be a topological disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ChartFailure {
    /// No boundary at all: a closed, sphere-like region.
    Closed,
    /// More than one boundary loop, such as a sleeve cut at both ends.
    MultipleBoundaries {
        /// How many loops.
        count: u32,
    },
    /// One or more handles, such as an arm touching the body.
    PositiveGenus {
        /// How many handles.
        genus: u32,
    },
}
