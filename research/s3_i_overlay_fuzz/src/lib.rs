//! Spike S3: fuzz i_overlay with per-edge seam allowances and catalogue what goes wrong.

pub mod allowance;
pub mod generate;
pub mod geom;
pub mod oracle;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

use i_overlay::i_shape::base::data::Shapes;

use crate::generate::{Family, Scale, case};
use crate::geom::{P, bbox_diag, is_simple_ccw};
use crate::oracle::{Fault, LONG_EDGE, Verdict, check_strips, check_uniform};

/// The seed of the committed corpus.
pub const SEED: u64 = 0x0057_3F00_2026_1015;

/// Which construction and integer engine produced a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    StripsI32,
    StripsI64,
    /// `StripsI64` after merging edges shorter than `LONG_EDGE` × the diagonal.
    StripsI64Merged,
    UniformOutlineI32,
}

pub struct CaseReport {
    pub index: u64,
    pub family: Family,
    pub scale: Scale,
    pub vertices: usize,
    pub faults: BTreeMap<Method, BTreeSet<Fault>>,
    /// Worst allowance shortfall in mm over all edges and over long edges, per method that returned.
    pub shortfall: BTreeMap<Method, [f64; 2]>,
    /// FNV-1a over every method's output coordinates, in output order.
    pub hash: u64,
}

thread_local! {
    static LAST_PANIC: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Records a panic's location and message instead of printing it; native only.
pub fn install_quiet_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        LAST_PANIC.with(|p| *p.borrow_mut() = format!("{loc}: {msg}"));
    }));
}

fn guarded(f: impl FnOnce() -> Shapes<P>) -> Result<Shapes<P>, Fault> {
    catch_unwind(AssertUnwindSafe(f))
        .map_err(|_| Fault::Panic(LAST_PANIC.with(|p| p.borrow().clone())))
}

fn fnv(hash: &mut u64, bits: u64) {
    for byte in bits.to_le_bytes() {
        *hash = (*hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
    }
}

fn fingerprint(hash: &mut u64, shapes: &Shapes<P>) {
    for shape in shapes {
        fnv(hash, shape.len() as u64);
        for contour in shape {
            fnv(hash, contour.len() as u64);
            contour
                .iter()
                .flatten()
                .for_each(|x| fnv(hash, x.to_bits()));
        }
    }
}

/// Runs every method on case `index` of the corpus for `seed`.
pub fn run_case(seed: u64, index: u64) -> CaseReport {
    let c = case(seed, index);
    let pieces = allowance::pieces(&c.outline, &c.allowance);
    let widest = c.allowance.iter().copied().fold(1e-3, f64::max);
    let eps = LONG_EDGE * bbox_diag(c.outline.iter().chain(pieces.iter().flatten()).copied());
    let (m_outline, m_allowance) = allowance::merge_short_edges(&c.outline, &c.allowance, eps);
    let mut report = CaseReport {
        index,
        family: c.family,
        scale: c.scale,
        vertices: c.outline.len(),
        faults: BTreeMap::new(),
        shortfall: BTreeMap::new(),
        hash: 0xCBF2_9CE4_8422_2325,
    };
    let (o, a) = (&c.outline, &c.allowance);
    report.record(
        Method::StripsI32,
        || allowance::strips::<i32>(o, a),
        |s| check_strips(o, a, s),
    );
    report.record(
        Method::StripsI64,
        || allowance::strips::<i64>(o, a),
        |s| check_strips(o, a, s),
    );
    let (mo, ma) = (&m_outline, &m_allowance);
    if is_simple_ccw(mo) {
        report.record(
            Method::StripsI64Merged,
            || allowance::strips::<i64>(mo, ma),
            |s| check_strips(mo, ma, s),
        );
    } else {
        report.faults.insert(
            Method::StripsI64Merged,
            BTreeSet::from([Fault::InvalidAfterMerge]),
        );
    }
    report.record(
        Method::UniformOutlineI32,
        || allowance::uniform_outline::<i32>(o, widest),
        |s| check_uniform(o, widest, s),
    );
    report
}

impl CaseReport {
    fn record(
        &mut self,
        method: Method,
        run: impl FnOnce() -> Shapes<P>,
        check: impl FnOnce(&Shapes<P>) -> Verdict,
    ) {
        let found = match guarded(run) {
            Err(panic) => BTreeSet::from([panic]),
            Ok(shapes) => {
                fingerprint(&mut self.hash, &shapes);
                let verdict = check(&shapes);
                self.shortfall
                    .insert(method, [verdict.shortfall, verdict.shortfall_long]);
                verdict.faults
            }
        };
        self.faults.insert(method, found);
    }
}

/// Fingerprint of cases `0..count`: native and wasm32 must agree on it.
pub fn corpus_hash(seed: u64, count: u64) -> u64 {
    let mut hash = 0xCBF2_9CE4_8422_2325;
    (0..count).for_each(|i| fnv(&mut hash, run_case(seed, i).hash));
    hash
}
