//! Proposal types.

use std::collections::BTreeMap;

use openplush_mesh::EdgeId;
use openplush_seams::{
    ChartFailure, ChartId, Proposal, ProposalId, ProposalKind, SeamKind, SeamReason, SeamsError,
};

fn edits(entries: &[(u32, SeamKind)]) -> BTreeMap<EdgeId, SeamKind> {
    entries.iter().map(|&(e, k)| (EdgeId::new(e), k)).collect()
}

#[test]
fn proposal_id_matches_a_hand_computed_fnv1a_vector() -> Result<(), SeamsError> {
    // Bytes: kind Dart = 1; edge 3 (LE) + SeamKind::Dart = 1; edge 7 (LE) +
    // SeamKind::Cut = 0. FNV-1a 64 of [1, 3,0,0,0, 1, 7,0,0,0, 0], computed
    // independently (Python), is 0x2b32133af65baa57.
    let p = Proposal::new(
        ProposalKind::Dart {
            chart: ChartId::new(0),
        },
        edits(&[(7, SeamKind::Cut), (3, SeamKind::Dart)]),
        "relieve the crown".into(),
    )?;
    assert_eq!(p.id(), ProposalId::from_raw(0x2b32_133a_f65b_aa57));
    assert_eq!(p.id().to_hex(), "2b32133af65baa57");
    Ok(())
}

#[test]
fn proposal_id_ignores_chart_ids_and_text() -> Result<(), SeamsError> {
    // An unrelated seam edit can renumber charts; the same edit keeps its ID.
    let add = edits(&[(3, SeamKind::Dart)]);
    let a = Proposal::new(
        ProposalKind::Dart {
            chart: ChartId::new(0),
        },
        add.clone(),
        "a".into(),
    )?;
    let b = Proposal::new(
        ProposalKind::Dart {
            chart: ChartId::new(5),
        },
        add,
        "b".into(),
    )?;
    assert_eq!(a.id(), b.id());
    Ok(())
}

#[test]
fn proposal_id_depends_on_kind_and_every_edit() -> Result<(), SeamsError> {
    let chart = ChartId::new(0);
    let id = |kind, add| Proposal::new(kind, add, String::new()).map(|p| p.id());
    let base = id(ProposalKind::Dart { chart }, edits(&[(3, SeamKind::Cut)]))?;
    assert_ne!(
        base,
        id(
            ProposalKind::SplitPiece { chart },
            edits(&[(3, SeamKind::Cut)])
        )?
    );
    assert_ne!(
        base,
        id(ProposalKind::Dart { chart }, edits(&[(3, SeamKind::Dart)]))?
    );
    assert_ne!(
        base,
        id(ProposalKind::Dart { chart }, edits(&[(4, SeamKind::Cut)]))?
    );
    assert_ne!(
        base,
        id(
            ProposalKind::Dart { chart },
            edits(&[(3, SeamKind::Cut), (4, SeamKind::Cut)])
        )?
    );
    let closed = ProposalKind::CloseChart {
        chart,
        failure: ChartFailure::Closed,
    };
    let seam = ProposalKind::Seam {
        reason: SeamReason::Crease,
    };
    assert_ne!(
        id(closed, edits(&[(3, SeamKind::Cut)]))?,
        id(seam, edits(&[(3, SeamKind::Cut)]))?
    );
    Ok(())
}

#[test]
fn an_empty_proposal_cannot_be_built() {
    assert_eq!(
        Proposal::new(
            ProposalKind::Seam {
                reason: SeamReason::SeamHint
            },
            BTreeMap::new(),
            "x".into()
        )
        .err(),
        Some(SeamsError::EmptyProposal)
    );
}

#[test]
fn proposal_ids_round_trip_through_hex_and_refuse_anything_else() {
    let id = ProposalId::from_raw(0x00ab_cdef_0123_4567);
    assert_eq!(id.to_hex(), "00abcdef01234567");
    assert_eq!(ProposalId::from_hex("00abcdef01234567"), Ok(id));
    for bad in [
        "",
        "abc",
        "00ABCDEF01234567",
        "00abcdef0123456g",
        "00abcdef012345678",
        "+0abcdef01234567",
    ] {
        assert_eq!(
            ProposalId::from_hex(bad),
            Err(SeamsError::BadProposalId {
                text: bad.to_owned()
            }),
            "{bad:?}"
        );
    }
}

#[test]
fn accessors_return_what_was_built() -> Result<(), SeamsError> {
    let add = edits(&[(3, SeamKind::Dart)]);
    let kind = ProposalKind::Dart {
        chart: ChartId::new(2),
    };
    let p = Proposal::new(kind, add.clone(), "why".into())?;
    assert_eq!(p.kind(), &kind);
    assert_eq!(p.add(), &add);
    assert_eq!(p.explanation(), "why");
    Ok(())
}

#[test]
fn every_kind_byte_and_seam_kind_byte_is_pinned() -> Result<(), SeamsError> {
    // One edge, index 0, unless noted. Each vector computed independently
    // (Python FNV-1a 64) from the data model's encoding, so moving any
    // byte, or giving two kinds the same byte, fails here.
    let chart = ChartId::new(0);
    let seam = ProposalKind::Seam {
        reason: SeamReason::Crease,
    };
    let cases: [(ProposalKind, u32, SeamKind, &str); 9] = [
        (
            ProposalKind::CloseChart {
                chart,
                failure: ChartFailure::Closed,
            },
            0,
            SeamKind::Cut,
            "d7e4fcfa299d713d",
        ),
        (
            ProposalKind::Dart { chart },
            0,
            SeamKind::Cut,
            "fb4e98c73babab04",
        ),
        (
            ProposalKind::SplitPiece { chart },
            0,
            SeamKind::Cut,
            "9111c5600580fdaf",
        ),
        (seam, 0, SeamKind::Cut, "b47b612d178f3776"),
        (seam, 0, SeamKind::Dart, "b47b622d178f3929"),
        (seam, 0, SeamKind::Opening, "b47b5f2d178f3410"),
        (seam, 0, SeamKind::FabricBoundary, "b47b602d178f35c3"),
        (seam, 0, SeamKind::Mandatory, "b47b652d178f3e42"),
        // Edge 0x01020304 pins the little-endian edge encoding.
        (seam, 0x0102_0304, SeamKind::Cut, "35e5e5ceacc443f2"),
    ];
    for (kind, edge, seam_kind, want) in cases {
        let p = Proposal::new(kind, edits(&[(edge, seam_kind)]), String::new())?;
        assert_eq!(
            p.id().to_hex(),
            want,
            "{kind:?} edge {edge:#x} {seam_kind:?}"
        );
    }
    Ok(())
}
