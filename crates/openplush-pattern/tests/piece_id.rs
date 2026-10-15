//! `PieceId`: a newtype, ordered by index.

use openplush_pattern::PieceId;

#[test]
fn piece_ids_wrap_their_index_and_order_by_it() {
    let (a, b) = (PieceId::new(0), PieceId::new(3));
    assert_eq!(b.index(), 3);
    assert!(a < b);
}

#[test]
fn piece_ids_are_const_and_print_their_name() {
    const PIECE: PieceId = PieceId::new(7);
    assert_eq!(format!("{PIECE:?}"), "PieceId(7)");
}
