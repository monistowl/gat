use anyhow::{anyhow, Result};
use faer::{prelude::*, solvers::PartialPivLu, Mat};

/// Trait for solving dense linear systems (Ax = b).
///
/// This is for linear algebra, not optimization solvers.
/// For OPF solvers, see `gat_algo::opf::dispatch::SolverBackend`.
pub trait LinearSystemBackend: Send + Sync {
    /// Solve the linear system Ax = b
    fn solve(&self, matrix: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>>;
}


#[derive(Debug, Clone, Default)]
pub struct GaussSolver;

impl LinearSystemBackend for GaussSolver {
    fn solve(&self, matrix: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>> {
        let n = matrix.len();
        if n == 0 {
            return Ok(Vec::new());
        }
        if rhs.len() != n {
            return Err(anyhow!(
                "rhs length ({}) does not match matrix dimension {}",
                rhs.len(),
                n
            ));
        }
        if matrix.iter().any(|row| row.len() != n) {
            return Err(anyhow!("matrix must be square"));
        }

        let mut a = matrix.to_vec();
        let mut b = rhs.to_vec();

        for i in 0..n {
            let mut pivot = i;
            for row in i + 1..n {
                if a[row][i].abs() > a[pivot][i].abs() {
                    pivot = row;
                }
            }
            if pivot != i {
                a.swap(i, pivot);
                b.swap(i, pivot);
            }

            let diag = a[i][i];
            if diag.abs() < 1e-12 {
                return Err(anyhow!("singular matrix"));
            }

            for value in a[i][i..].iter_mut() {
                *value /= diag;
            }
            b[i] /= diag;

            let pivot_segment = a[i][i..].to_vec();
            for row in 0..n {
                if row == i {
                    continue;
                }
                let factor = a[row][i];
                for (target, &pivot) in a[row][i..].iter_mut().zip(pivot_segment.iter()) {
                    *target -= factor * pivot;
                }
                b[row] -= factor * b[i];
            }
        }

        Ok(b)
    }
}

#[derive(Debug, Clone, Default)]
pub struct FaerSolver;

impl LinearSystemBackend for FaerSolver {
    fn solve(&self, matrix: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>> {
        let n = matrix.len();
        if n == 0 {
            return Ok(Vec::new());
        }
        if rhs.len() != n {
            return Err(anyhow!(
                "rhs length ({}) does not match matrix dimension {}",
                rhs.len(),
                n
            ));
        }
        if matrix.iter().any(|row| row.len() != n) {
            return Err(anyhow!("matrix must be square"));
        }

        let mat = Mat::from_fn(n, n, |i, j| matrix[i][j]);
        let rhs_mat = Mat::from_fn(n, 1, |i, _| rhs[i]);
        let lu = PartialPivLu::new(mat.as_ref());
        let sol = lu.solve(&rhs_mat);

        let mut solution = Vec::with_capacity(n);
        for i in 0..n {
            solution.push(sol.read(i, 0));
        }
        Ok(solution)
    }
}
