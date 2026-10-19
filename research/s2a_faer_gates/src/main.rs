//! Prints the fingerprint and residual native code produces, for the cross-target test.

fn main() {
    for n in [10, 50, 100] {
        let x = s2a_faer_gates::solve(n);
        println!(
            "n={n} unknowns={} fingerprint={:#018x} residual={:e}",
            n * n,
            s2a_faer_gates::fingerprint(&x),
            s2a_faer_gates::residual(n, &x)
        );
    }
}
