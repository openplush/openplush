//! Native run: prints each rotation set's nest and wall time.

use std::time::Instant;

use jagua_rs_wasm::{RotationSet, nest};

fn main() {
    for set in [RotationSet::Fixed, RotationSet::HalfTurn] {
        let t = Instant::now();
        let result = nest(set);
        let elapsed = t.elapsed();
        match result {
            Ok(n) => {
                println!(
                    "{set:?}: length {} mm, hash {}  {elapsed:?}",
                    n.length, n.hash
                );
                for p in &n.placements {
                    println!("  {p:?}");
                }
            }
            Err(e) => println!("{set:?}: error {e}  {elapsed:?}"),
        }
    }
}
