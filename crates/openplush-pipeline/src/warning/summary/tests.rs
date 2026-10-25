//! How the summary names a warning's place.

use openplush_pattern::PieceId;

use super::target_words;
use crate::WarningTarget;

#[test]
fn two_pieces_are_named_lower_first_as_the_target_holds_them() {
    let target = WarningTarget::Pieces(PieceId::new(2), PieceId::new(7));
    assert_eq!(target_words(target), "pieces 2 and 7");
}
