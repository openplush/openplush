//! `PieceId` (T175, data-model section 13a): a newtype, ordered by index.

use openplush_pattern::PieceId;

#[test]
fn piece_ids_wrap_their_index_and_order_by_it() {
    let (a, b) = (PieceId::new(0), PieceId::new(3));
    assert_eq!(b.index(), 3);
    assert!(a < b);
}
