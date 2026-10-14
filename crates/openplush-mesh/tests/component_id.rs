//! `ComponentId`: a newtype, ordered by index.

use openplush_mesh::ComponentId;

#[test]
fn component_ids_wrap_their_index_and_order_by_it() {
    let (a, b) = (ComponentId::new(2), ComponentId::new(7));
    assert_eq!(a.index(), 2);
    assert!(a < b);
    assert_eq!(a, ComponentId::new(2));
}
