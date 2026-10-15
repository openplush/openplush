//! Native fuzz run: `cargo run --release -- [cases] [seed]`.
//! Prints counts per method and fault, a breakdown by family and scale, and the
//! smallest reproducer of each fault.

use std::collections::BTreeMap;

use s3_i_overlay_fuzz::allowance;
use s3_i_overlay_fuzz::generate::{Scale, case};
use s3_i_overlay_fuzz::geom::is_simple_ccw;
use s3_i_overlay_fuzz::oracle::Fault;
use s3_i_overlay_fuzz::{
    CaseReport, Method, SEED, corpus_hash, install_quiet_panic_hook, run_case,
};

type Key = (Method, Fault);

/// Outlines the post-flatten invariant excludes, to see what i_overlay does without validation.
fn invalid_inputs() {
    let cases: [(&str, Vec<[f64; 2]>); 6] = [
        // Upstream's regression contour for its outline fix: an exact 180° reversal.
        (
            "upstream outline test_real_case_1",
            vec![[53.0, 42.0], [35.0, 66.0], [26.0, 75.0], [27.0, 74.0]],
        ),
        (
            "zero-width spike",
            vec![
                [0.0, 0.0],
                [100.0, 0.0],
                [100.0, 50.0],
                [50.0, 50.0],
                [50.0, 120.0],
                [50.0, 50.0],
                [0.0, 50.0],
            ],
        ),
        (
            "repeated vertex",
            vec![
                [0.0, 0.0],
                [100.0, 0.0],
                [100.0, 0.0],
                [100.0, 50.0],
                [0.0, 50.0],
            ],
        ),
        (
            "clockwise",
            vec![[0.0, 0.0], [0.0, 50.0], [100.0, 50.0], [100.0, 0.0]],
        ),
        (
            "bow-tie",
            vec![[0.0, 0.0], [100.0, 50.0], [100.0, 0.0], [0.0, 50.0]],
        ),
        (
            "collinear only",
            vec![[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]],
        ),
    ];
    println!(
        "\ninvalid inputs, 6 mm on every edge (strips i32 | strips i64 | uniform outline i32):"
    );
    for (name, outline) in cases {
        let a = vec![6.0; outline.len()];
        let shape = |r: std::thread::Result<Vec<Vec<Vec<[f64; 2]>>>>| match r {
            Err(_) => "PANIC".to_owned(),
            Ok(s) => format!(
                "{} shapes, {} holes",
                s.len(),
                s.iter().map(|x| x.len() - 1).sum::<usize>()
            ),
        };
        let guard = |f: &dyn Fn() -> Vec<Vec<Vec<[f64; 2]>>>| {
            shape(std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)))
        };
        println!(
            "  {name}: valid={} | {} | {} | {}",
            is_simple_ccw(&outline),
            guard(&|| allowance::strips::<i32>(&outline, &a)),
            guard(&|| allowance::strips::<i64>(&outline, &a)),
            guard(&|| allowance::uniform_outline::<i32>(&outline, 6.0)),
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let count: u64 = args
        .get(1)
        .map_or(10_000, |s| s.parse().expect("cases: an integer"));
    let seed: u64 = args
        .get(2)
        .map_or(SEED, |s| s.parse().expect("seed: an integer"));
    install_quiet_panic_hook();

    let reports: Vec<CaseReport> = (0..count).map(|i| run_case(seed, i)).collect();
    let mut tally: BTreeMap<Key, Vec<&CaseReport>> = BTreeMap::new();
    let mut clean: BTreeMap<Method, u64> = BTreeMap::new();
    for r in &reports {
        for (m, faults) in &r.faults {
            if faults.is_empty() {
                *clean.entry(*m).or_default() += 1;
            }
            faults
                .iter()
                .for_each(|f| tally.entry((*m, f.clone())).or_default().push(r));
        }
    }
    println!("seed {seed}, {count} cases");
    for (m, n) in &clean {
        println!("{m:?}: {n} cases with no fault");
    }
    println!(
        "\nmethod | scale | edges | worst shortfall mm (case) | cases short > 1e-6 mm | > 1e-4 mm | > 1e-2 mm"
    );
    for m in clean.keys() {
        for scale in [Scale::Plush, Scale::Tiny, Scale::Huge] {
            for (label, k) in [("all", 0), ("long", 1)] {
                let s: Vec<(f64, u64)> = reports
                    .iter()
                    .filter(|r| r.scale == scale)
                    .filter_map(|r| r.shortfall.get(m).map(|x| (x[k], r.index)))
                    .collect();
                let worst = s.iter().copied().max_by(|a, b| a.0.total_cmp(&b.0));
                let over = |t: f64| s.iter().filter(|x| x.0 > t).count();
                println!(
                    "{m:?} | {scale:?} | {label} | {worst:?} | {} | {} | {}",
                    over(1e-6),
                    over(1e-4),
                    over(1e-2)
                );
            }
        }
    }
    println!(
        "\nmethod | fault | cases | with guards clear | by family | by scale | smallest reproducer (index, vertices)"
    );
    for ((m, f), rs) in &tally {
        let clear = rs
            .iter()
            .filter(|r| !r.faults[m].contains(&Fault::GuardHitsOutline))
            .count();
        let mut fam: BTreeMap<String, u64> = BTreeMap::new();
        let mut sc: BTreeMap<String, u64> = BTreeMap::new();
        for r in rs {
            *fam.entry(format!("{:?}", r.family)).or_default() += 1;
            *sc.entry(format!("{:?}", r.scale)).or_default() += 1;
        }
        let small = rs
            .iter()
            .min_by_key(|r| (r.vertices, r.index))
            .map(|r| (r.index, r.vertices));
        println!(
            "{m:?} | {f:?} | {} | {clear} | {fam:?} | {sc:?} | {small:?}",
            rs.len()
        );
    }
    println!("\nreproducers (outline, then per-edge allowance in mm):");
    for ((m, f), rs) in &tally {
        if let Some(r) = rs.iter().min_by_key(|r| (r.vertices, r.index)) {
            let c = case(seed, r.index);
            println!(
                "{m:?} {f:?} case {}:\n  {:?}\n  {:?}",
                r.index, c.outline, c.allowance
            );
        }
    }
    invalid_inputs();
    let n = count.min(1000);
    let (first, second) = (corpus_hash(seed, n), corpus_hash(seed, n));
    println!(
        "\ncorpus_hash(first {n}) = {first}, rerun = {second}, equal = {}",
        first == second
    );
}
