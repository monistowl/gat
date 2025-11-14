use super::backend::{FaerSolver, GaussSolver, SolverBackend};
use anyhow::{anyhow, Result};
use std::sync::Arc;

/// Simple registry of available solvers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolverKind {
    Gauss,
    Faer,
}

impl Default for SolverKind {
    fn default() -> Self {
        SolverKind::Gauss
    }
}

impl SolverKind {
    pub fn from_str(input: &str) -> Result<Self> {
        match input.to_ascii_lowercase().as_str() {
            "gauss" | "default" => Ok(SolverKind::Gauss),
            "faer" => Ok(SolverKind::Faer),
            other => Err(anyhow!(
                "unknown solver '{}'; supported values: gauss, faer",
                other
            )),
        }
    }

    pub fn build_solver(self) -> Arc<dyn SolverBackend> {
        match self {
            SolverKind::Gauss => Arc::new(GaussSolver::default()),
            SolverKind::Faer => Arc::new(FaerSolver::default()),
        }
    }

    pub fn available() -> &'static [&'static str] {
        &["gauss", "faer"]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SolverKind::Gauss => "gauss",
            SolverKind::Faer => "faer",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solver_kind_parsing_supports_all_engines() {
        assert_eq!(SolverKind::from_str("gauss").unwrap(), SolverKind::Gauss);
        assert_eq!(SolverKind::from_str("faer").unwrap(), SolverKind::Faer);
        assert!(matches!(SolverKind::from_str("unknown"), Err(_)));
    }

    #[test]
    fn solver_backend_options_solve_diagonal_system() {
        let matrix = vec![vec![2.0, 0.0], vec![0.0, 3.0]];
        let rhs = vec![4.0, 6.0];

        let gauss = GaussSolver::default();
        assert_eq!(gauss.solve(&matrix, &rhs).unwrap(), vec![2.0, 2.0]);

        let faer = FaerSolver::default();
        assert_eq!(faer.solve(&matrix, &rhs).unwrap(), vec![2.0, 2.0]);
    }
}
