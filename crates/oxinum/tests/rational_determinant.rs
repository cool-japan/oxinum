//! Integration test: exact 3x3 rational determinant via the `oxinum` facade.
//!
//! Uses the standard Hilbert(3) matrix
//!   H = [[1,   1/2, 1/3],
//!        [1/2, 1/3, 1/4],
//!        [1/3, 1/4, 1/5]]
//! whose determinant is the well-known exact value `1/2160`.

use oxinum::{IBig, RBig};

#[test]
fn hilbert3_determinant_is_one_over_2160() {
    let r = |n: i64, d: i64| RBig::from_parts_signed(IBig::from(n), IBig::from(d));
    let m = [
        [r(1, 1), r(1, 2), r(1, 3)],
        [r(1, 2), r(1, 3), r(1, 4)],
        [r(1, 3), r(1, 4), r(1, 5)],
    ];
    // det = a*(e*i - f*h) - b*(d*i - f*g) + c*(d*h - e*g)
    let det = &m[0][0] * &(&(&m[1][1] * &m[2][2]) - &(&m[1][2] * &m[2][1]))
        - &m[0][1] * &(&(&m[1][0] * &m[2][2]) - &(&m[1][2] * &m[2][0]))
        + &m[0][2] * &(&(&m[1][0] * &m[2][1]) - &(&m[1][1] * &m[2][0]));

    let expected = RBig::from_parts_signed(IBig::from(1), IBig::from(2160));
    assert_eq!(det, expected, "det(Hilbert(3)) = {det}, expected 1/2160");
}

#[test]
fn rational_determinant_handles_negative_entries() {
    // Matrix with a known exact integer determinant:
    //   [ 2,  1, -1]
    //   [-3, -1,  2]
    //   [-2,  1,  2]
    // det = 2*((-1)*2 - 2*1) - 1*((-3)*2 - 2*(-2)) + (-1)*((-3)*1 - (-1)*(-2))
    //     = 2*(-4) - 1*(-2) + (-1)*(-5) = -8 + 2 + 5 = -1
    let r = |n: i64, d: i64| RBig::from_parts_signed(IBig::from(n), IBig::from(d));
    let m = [
        [r(2, 1), r(1, 1), r(-1, 1)],
        [r(-3, 1), r(-1, 1), r(2, 1)],
        [r(-2, 1), r(1, 1), r(2, 1)],
    ];
    let det = &m[0][0] * &(&(&m[1][1] * &m[2][2]) - &(&m[1][2] * &m[2][1]))
        - &m[0][1] * &(&(&m[1][0] * &m[2][2]) - &(&m[1][2] * &m[2][0]))
        + &m[0][2] * &(&(&m[1][0] * &m[2][1]) - &(&m[1][1] * &m[2][0]));
    let expected = RBig::from_parts_signed(IBig::from(-1), IBig::from(1));
    assert_eq!(det, expected, "det = {det}, expected -1");
}
