use std::collections::BTreeMap;

use openplush_mesh::{EdgeId, FaceId};
use openplush_seams::{ChartId, Proposal, ProposalKind, SeamKind};

use super::{NonBlank, Remedy, Warning, WarningCode, WarningSet, WarningTarget};

fn text(s: &str) -> NonBlank {
    NonBlank::new(s.to_owned()).expect("non-blank test text")
}

fn dart(chart: u32, edge: u32) -> Remedy {
    let add = BTreeMap::from([(EdgeId::new(edge), SeamKind::Dart)]);
    let kind = ProposalKind::Dart {
        chart: ChartId::new(chart),
    };
    let proposal = Proposal::new(kind, add, "a dart".into()).expect("a non-empty proposal");
    Remedy::Actionable { proposal }
}

fn advise(s: &str) -> Remedy {
    Remedy::Advisory {
        what_to_change: text(s),
    }
}

fn warning(code: WarningCode, target: WarningTarget, remedy: Remedy, why: &str) -> Warning {
    Warning::new(code, target, text(why), remedy)
}

/// Distinct warnings in the order the set must yield them, each differing from
/// the one before in exactly the key that orders them.
fn expected() -> Vec<Warning> {
    use WarningCode::{TestAlpha, TestZulu};
    use WarningTarget::{Edge, Face};
    let firmer = || advise("use a firmer fabric");
    vec![
        warning(TestAlpha, Face(FaceId::new(2)), dart(0, 5), "a"),
        // Same edit, so the same ProposalId; the chart orders it.
        warning(TestAlpha, Face(FaceId::new(2)), dart(9, 5), "a"),
        // Actionable before Advisory.
        warning(TestAlpha, Face(FaceId::new(2)), firmer(), "a"),
        // Explanation is the last tie-break.
        warning(TestAlpha, Face(FaceId::new(2)), firmer(), "b"),
        // Same target variant: by ID.
        warning(TestAlpha, Face(FaceId::new(7)), firmer(), "a"),
        // Target variant order beats ID: Face before Edge, though Edge's ID is lower.
        warning(TestAlpha, Edge(EdgeId::new(0)), firmer(), "a"),
        // Code by string: "test-zulu" after "test-alpha", though declared first.
        warning(TestZulu, Face(FaceId::new(0)), dart(0, 1), "a"),
    ]
}

fn for_each_permutation(order: &mut [usize], k: usize, visit: &mut impl FnMut(&[usize])) {
    if k == order.len() {
        visit(order);
        return;
    }
    for i in k..order.len() {
        order.swap(k, i);
        for_each_permutation(order, k + 1, visit);
        order.swap(k, i);
    }
}

#[test]
fn order_is_total_and_independent_of_insertion_order() {
    let want = expected();
    let Remedy::Actionable { proposal: first } = want[0].remedy() else {
        panic!("the first expected warning is actionable");
    };
    let Remedy::Actionable { proposal: second } = want[1].remedy() else {
        panic!("the second expected warning is actionable");
    };
    assert_eq!(
        first.id(),
        second.id(),
        "the chart must be the only difference"
    );

    let mut inputs = want.clone();
    inputs.push(want[3].clone()); // an exact duplicate, kept once
    let mut order: Vec<usize> = (0..inputs.len()).collect();
    let mut runs = 0;
    for_each_permutation(&mut order, 0, &mut |perm| {
        let mut set = WarningSet::default();
        for &i in perm {
            set.insert(inputs[i].clone());
        }
        assert!(set.iter().eq(want.iter()), "insertion order {perm:?}");
        runs += 1;
    });
    assert_eq!(runs, 40_320);
}

#[test]
fn an_empty_set_yields_nothing() {
    assert_eq!(WarningSet::default().iter().count(), 0);
}

#[test]
fn blank_text_is_refused() {
    assert_eq!(NonBlank::new(String::new()), None);
    assert_eq!(NonBlank::new(" \n\t ".into()), None);
    let kept = NonBlank::new(" padded ".into()).map(|t| t.as_str().to_owned());
    assert_eq!(kept.as_deref(), Some(" padded "));
}

#[test]
fn code_strings_are_distinct_kebab_case() {
    let codes = [WarningCode::TestZulu, WarningCode::TestAlpha];
    assert_eq!(codes.map(WarningCode::as_str), ["test-zulu", "test-alpha"]);
}

#[test]
fn a_warning_reads_back_what_it_was_built_from() {
    let w = warning(
        WarningCode::TestAlpha,
        WarningTarget::Project,
        advise("size up"),
        "too small",
    );
    assert_eq!(w.code(), WarningCode::TestAlpha);
    assert_eq!(w.target(), WarningTarget::Project);
    assert_eq!(w.explanation(), "too small");
    assert_eq!(w.remedy(), &advise("size up"));
}
