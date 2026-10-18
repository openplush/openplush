//! `SeamSet`: typed edge flags, ordered by edge, with mandatory seams that stay.

use openplush_mesh::EdgeId;
use openplush_seams::{SeamKind, SeamSet, SeamsError};

fn e(i: u32) -> EdgeId {
    EdgeId::new(i)
}

fn contents(set: &SeamSet) -> Vec<(EdgeId, SeamKind)> {
    set.iter().collect()
}

#[test]
fn a_new_set_is_empty() {
    let set = SeamSet::new();
    assert!(set.is_empty());
    assert_eq!(set.len(), 0);
    assert_eq!(set.get(e(0)), None);
    assert_eq!(set, SeamSet::default());
}

#[test]
fn insert_reports_what_it_replaced() {
    let mut set = SeamSet::new();
    assert_eq!(set.insert(e(3), SeamKind::Cut), Ok(None));
    assert_eq!(set.insert(e(3), SeamKind::Dart), Ok(Some(SeamKind::Cut)));
    assert_eq!(set.get(e(3)), Some(SeamKind::Dart));
    assert_eq!(set.len(), 1);
    assert!(!set.is_empty());
}

#[test]
fn iteration_is_in_ascending_edge_order_whatever_the_insertion_order() -> Result<(), SeamsError> {
    let mut set = SeamSet::new();
    for i in [9, 2, 5, 0] {
        set.insert(e(i), SeamKind::Cut)?;
    }
    set.insert(e(5), SeamKind::Opening)?;
    assert_eq!(
        contents(&set),
        [
            (e(0), SeamKind::Cut),
            (e(2), SeamKind::Cut),
            (e(5), SeamKind::Opening),
            (e(9), SeamKind::Cut)
        ]
    );
    Ok(())
}

#[test]
fn remove_returns_the_kind_and_leaves_the_rest() -> Result<(), SeamsError> {
    let mut set = SeamSet::new();
    set.insert(e(1), SeamKind::Cut)?;
    set.insert(e(2), SeamKind::FabricBoundary)?;
    assert_eq!(set.remove(e(1)), Ok(Some(SeamKind::Cut)));
    assert_eq!(set.remove(e(1)), Ok(None), "already gone");
    assert_eq!(set.remove(e(7)), Ok(None), "never there");
    assert_eq!(contents(&set), [(e(2), SeamKind::FabricBoundary)]);
    Ok(())
}

#[test]
fn a_mandatory_seam_cannot_be_removed() -> Result<(), SeamsError> {
    let mut set = SeamSet::new();
    set.insert(e(4), SeamKind::Mandatory)?;
    assert_eq!(
        set.remove(e(4)),
        Err(SeamsError::MandatorySeam { edge: e(4) })
    );
    assert_eq!(set.get(e(4)), Some(SeamKind::Mandatory));
    Ok(())
}

#[test]
fn a_mandatory_seam_cannot_be_given_another_kind() -> Result<(), SeamsError> {
    let mut set = SeamSet::new();
    set.insert(e(4), SeamKind::Mandatory)?;
    for kind in [SeamKind::Cut, SeamKind::Dart, SeamKind::Opening] {
        assert_eq!(
            set.insert(e(4), kind),
            Err(SeamsError::MandatorySeam { edge: e(4) })
        );
    }
    assert_eq!(set.get(e(4)), Some(SeamKind::Mandatory));
    Ok(())
}

#[test]
fn a_seam_can_become_mandatory_and_mandatory_can_be_set_again() -> Result<(), SeamsError> {
    let mut set = SeamSet::new();
    set.insert(e(4), SeamKind::Cut)?;
    assert_eq!(
        set.insert(e(4), SeamKind::Mandatory),
        Ok(Some(SeamKind::Cut))
    );
    assert_eq!(
        set.insert(e(4), SeamKind::Mandatory),
        Ok(Some(SeamKind::Mandatory))
    );
    Ok(())
}

#[test]
fn a_clone_is_independent() -> Result<(), SeamsError> {
    let mut set = SeamSet::new();
    set.insert(e(1), SeamKind::Cut)?;
    let mut copy = set.clone();
    copy.insert(e(2), SeamKind::Cut)?;
    assert_eq!(set.len(), 1);
    assert_ne!(set, copy);
    Ok(())
}
