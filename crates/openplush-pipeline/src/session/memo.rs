use std::collections::BTreeMap;

use openplush_flatten::StoppedBy;
use openplush_geom::DVec2;
use openplush_seams::ChartMesh;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Each laid-out chart's flat positions and why its flatten stopped, filed under its content's key and checked against the
/// whole chart it was laid out for, so two charts whose keys collide each get their own.
#[derive(Debug, Default)]
pub(crate) struct Memo {
    filed: BTreeMap<u64, Vec<Entry>>,
}

#[derive(Debug)]
struct Entry {
    chart: ChartMesh,
    uv: Vec<DVec2>,
    stopped: StoppedBy,
}

impl Memo {
    /// The positions laid out for `chart`, filed under `key`, and why its flatten stopped.
    pub(crate) fn get(&self, key: u64, chart: &ChartMesh) -> Option<(&[DVec2], StoppedBy)> {
        self.filed
            .get(&key)?
            .iter()
            .find(|e| e.chart == *chart)
            .map(|e| (&e.uv[..], e.stopped))
    }

    pub(crate) fn insert(
        &mut self,
        key: u64,
        chart: ChartMesh,
        uv: Vec<DVec2>,
        stopped: StoppedBy,
    ) {
        self.filed
            .entry(key)
            .or_default()
            .push(Entry { chart, uv, stopped });
    }

    /// Keeps only the entries laid out for one of `charts`, each beside its key.
    pub(crate) fn retain(&mut self, charts: &[ChartMesh], keys: &[u64]) {
        let mut kept: BTreeMap<u64, Vec<Entry>> = BTreeMap::new();
        for (chart, &key) in charts.iter().zip(keys) {
            let Some(filed) = self.filed.get_mut(&key) else {
                continue;
            };
            if let Some(at) = filed.iter().position(|e| e.chart == *chart) {
                kept.entry(key).or_default().push(filed.swap_remove(at));
            }
        }
        self.filed = kept;
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.filed.values().map(Vec::len).sum()
    }
}

/// 64-bit FNV-1a over each face's ID and three corners, then each position's three
/// coordinates' bits, every word little-endian.
pub(crate) fn key(chart: &ChartMesh) -> u64 {
    let mut h = FNV_OFFSET;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h = (h ^ u64::from(b)).wrapping_mul(FNV_PRIME);
        }
    };
    for (face, corners) in chart.faces() {
        eat(&face.index().to_le_bytes());
        for v in corners {
            eat(&v.index().to_le_bytes());
        }
    }
    for p in chart.xyz() {
        for c in [p.x, p.y, p.z] {
            eat(&c.to_bits().to_le_bytes());
        }
    }
    h
}
