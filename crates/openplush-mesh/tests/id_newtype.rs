//! `id_newtype!` from another crate: the shape every crate's ID types rely on.

use std::collections::BTreeSet;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

openplush_mesh::id_newtype! {
    /// An ID defined only for this test.
    TestId
}

const THREE: TestId = TestId::new(3);
const THREE_INDEX: u32 = THREE.index();

#[test]
fn new_and_index_round_trip() {
    assert_eq!(TestId::new(7).index(), 7);
    assert_eq!(TestId::new(u32::MAX).index(), u32::MAX);
}

#[test]
fn new_and_index_are_const() {
    assert_eq!(THREE, TestId::new(3));
    assert_eq!(THREE_INDEX, 3);
}

#[test]
fn ids_order_by_index() {
    assert!(TestId::new(2) < TestId::new(7));
    let sorted: Vec<u32> = BTreeSet::from([TestId::new(9), TestId::new(1), TestId::new(4)])
        .into_iter()
        .map(TestId::index)
        .collect();
    assert_eq!(sorted, [1, 4, 9]);
}

#[test]
fn equal_ids_hash_equally() {
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    assert_eq!(
        hasher.hash_one(TestId::new(5)),
        hasher.hash_one(TestId::new(5))
    );
}

#[test]
fn debug_prints_name_and_index() {
    assert_eq!(format!("{:?}", TestId::new(3)), "TestId(3)");
}
