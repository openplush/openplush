use super::proxy;

type Mat = [[f64; 2]; 2];

fn rot(a: f64) -> Mat {
    let (s, c) = libm::sincos(a);
    [[c, -s], [s, c]]
}

fn mul(a: Mat, b: Mat) -> Mat {
    let at = |i: usize, j: usize| a[i][0] * b[0][j] + a[i][1] * b[1][j];
    [[at(0, 0), at(0, 1)], [at(1, 0), at(1, 1)]]
}

fn diag(x: f64, y: f64) -> Mat {
    [[x, 0.0], [0.0, y]]
}

fn near(a: Mat, b: Mat) -> bool {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .all(|(x, y)| (x - y).abs() <= 1e-12)
}

#[test]
fn a_rotation_is_its_own_target_and_weighs_four() {
    let j = rot(0.7);
    let p = proxy(j);
    assert!(near(p.rotation, j));
    assert!(near(p.weight, diag(4.0, 4.0)));
}

#[test]
fn the_target_is_the_rotation_of_the_polar_decomposition_and_the_weights_follow_the_singular_values()
 {
    // J = U diag(2, 1/2) Vᵀ with U turned by φ and Vᵀ by θ.
    let (phi, theta) = (0.4, -1.1);
    let p = proxy(mul(mul(rot(phi), diag(2.0, 0.5)), rot(theta)));
    assert!(near(p.rotation, rot(phi + theta)));
    // The weight is U diag(w(2), w(1/2)) Uᵀ with w(s) = (s - s⁻³) / (s - 1): 15/8 and 15.
    let want = mul(mul(rot(phi), diag(15.0 / 8.0, 15.0)), rot(-phi));
    assert!(near(p.weight, want));
}

#[test]
fn the_weight_is_symmetric() {
    let p = proxy([[1.3, 0.4], [-0.2, 0.9]]);
    assert_eq!(p.weight[0][1], p.weight[1][0]);
}

#[test]
fn a_uniform_scale_weighs_by_the_scale() {
    // J = s I: both singular values are s, the weight is w(s) I.
    let s = 3.0;
    let p = proxy(diag(s, s));
    let w = (s - libm::pow(s, -3.0)) / (s - 1.0);
    assert!(near(p.weight, diag(w, w)));
    assert!(near(p.rotation, diag(1.0, 1.0)));
}
