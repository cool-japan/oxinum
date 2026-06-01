//! Solve a 3x3 linear system exactly over `RBig` using Cramer's rule.
//!
//! Demonstrates exact (no-rounding) arithmetic through the `oxinum` facade.
//!
//! Run:
//!     cargo run --example exact_rational_linear_solve
//!
//! The example solves
//!     2x +  y -  z =  8
//!    -3x -  y + 2z = -11
//!    -2x +  y + 2z = -3
//! whose unique solution is (x, y, z) = (2, 3, -1).

use oxinum::{IBig, RBig};

fn det_3x3(m: &[[RBig; 3]; 3]) -> RBig {
    &m[0][0] * &(&(&m[1][1] * &m[2][2]) - &(&m[1][2] * &m[2][1]))
        - &m[0][1] * &(&(&m[1][0] * &m[2][2]) - &(&m[1][2] * &m[2][0]))
        + &m[0][2] * &(&(&m[1][0] * &m[2][1]) - &(&m[1][1] * &m[2][0]))
}

fn main() {
    let r = |n: i64, d: i64| RBig::from_parts_signed(IBig::from(n), IBig::from(d));
    let a = [
        [r(2, 1), r(1, 1), r(-1, 1)],
        [r(-3, 1), r(-1, 1), r(2, 1)],
        [r(-2, 1), r(1, 1), r(2, 1)],
    ];
    let b = [r(8, 1), r(-11, 1), r(-3, 1)];

    let det_a = det_3x3(&a);
    assert_ne!(
        det_a,
        RBig::from_parts_signed(IBig::from(0), IBig::from(1)),
        "system is singular"
    );

    // Cramer's rule: x_i = det(A_i) / det(A), where A_i replaces column i with b.
    let mut solution: [RBig; 3] = [
        RBig::from_parts_signed(IBig::from(0), IBig::from(1)),
        RBig::from_parts_signed(IBig::from(0), IBig::from(1)),
        RBig::from_parts_signed(IBig::from(0), IBig::from(1)),
    ];

    for i in 0..3 {
        let mut a_i = a.clone();
        for (row, b_val) in b.iter().enumerate() {
            a_i[row][i] = b_val.clone();
        }
        let det_i = det_3x3(&a_i);
        solution[i] = &det_i / &det_a;
    }

    println!("System:");
    println!("   2x +  y -  z =  8");
    println!("  -3x -  y + 2z = -11");
    println!("  -2x +  y + 2z = -3");
    println!();
    println!(
        "Exact solution: x = {}, y = {}, z = {}",
        solution[0], solution[1], solution[2]
    );

    // Verify A * solution == b exactly.
    for (row_idx, row) in a.iter().enumerate() {
        let lhs =
            &(&(&row[0] * &solution[0]) + &(&row[1] * &solution[1])) + &(&row[2] * &solution[2]);
        let rhs = &b[row_idx];
        assert_eq!(&lhs, rhs, "row {row_idx}: lhs={lhs}, rhs={rhs}");
    }
    println!("Verification: A * x == b (exact)");
}
