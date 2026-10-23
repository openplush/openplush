//! Prints one case, its pieces and guards, the i64 strip result, and where a result contour
//! touches itself: `cargo run --example inspect -- <index>`.

use i_overlay_fuzz::generate::case;
use i_overlay_fuzz::geom::first_self_contact;
use i_overlay_fuzz::{SEED, allowance};

fn main() {
    let index: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let c = case(SEED, index);
    println!("outline {:?}\nallowance {:?}", c.outline, c.allowance);
    for p in allowance::pieces(&c.outline, &c.allowance) {
        println!("piece {p:?}");
    }
    let guards = allowance::fold_guards(&c.outline, &c.allowance);
    println!(
        "guards clear: {}",
        allowance::guards_clear(&c.outline, &guards)
    );
    for g in guards {
        println!("guard {g:?}");
    }
    for (k, shape) in allowance::strips::<i64>(&c.outline, &c.allowance)
        .iter()
        .enumerate()
    {
        for (h, contour) in shape.iter().enumerate() {
            println!("result shape {k} contour {h}: {contour:?}");
            if let Some((i, j)) = first_self_contact(contour) {
                let n = contour.len();
                let seg = |k: usize| (contour[k], contour[(k + 1) % n]);
                println!(
                    "  self-contact: edge {i} {:?} meets edge {j} {:?}",
                    seg(i),
                    seg(j)
                );
            }
        }
    }
}
