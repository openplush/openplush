//! `ChartId` and `SeamChainId` keep the shape every ID shares, whoever generates them.

use openplush_seams::{ChartId, SeamChainId};

const CHART: ChartId = ChartId::new(4);
const CHAIN: SeamChainId = SeamChainId::new(2);

#[test]
fn ids_wrap_their_index_order_by_it_and_print_their_name() {
    assert_eq!(CHART.index(), 4);
    assert_eq!(CHAIN.index(), 2);
    assert!(ChartId::new(1) < CHART);
    assert!(SeamChainId::new(1) < CHAIN);
    assert_eq!(format!("{CHART:?} {CHAIN:?}"), "ChartId(4) SeamChainId(2)");
    shared::<ChartId>();
    shared::<SeamChainId>();
}

/// Compiles only if `T` has every trait the IDs share.
fn shared<T: Copy + Eq + Ord + core::hash::Hash + core::fmt::Debug>() {}
