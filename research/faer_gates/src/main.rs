//! Prints the fingerprint and residual native code produces, for the cross-target test.

fn main() {
    for n in [10, 50, 100] {
        let x = faer_gates::solve(n);
        println!(
            "n={n} unknowns={} fingerprint={:#018x} residual={:e}",
            n * n,
            faer_gates::fingerprint(&x),
            faer_gates::residual(n, &x)
        );
    }
}
