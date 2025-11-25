//! Common types and utilities for benchmark commands.

use serde::Serialize;

/// Base timing and convergence fields shared by all benchmarks
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkTiming {
    /// Time to load/parse the case (ms)
    pub load_time_ms: f64,
    /// Time to solve (ms)
    pub solve_time_ms: f64,
    /// Total time (ms)
    pub total_time_ms: f64,
}

/// Base convergence fields
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkConvergence {
    /// Whether the solver converged
    pub converged: bool,
    /// Number of iterations
    pub iterations: u32,
}

/// Base size fields
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkSize {
    /// Number of buses
    pub num_buses: usize,
    /// Number of branches
    pub num_branches: usize,
    /// Number of generators
    pub num_gens: usize,
}

/// Tolerance configuration for benchmarks
#[derive(Debug, Clone)]
pub struct BenchmarkTolerances {
    /// Objective value relative tolerance
    pub obj_tol: f64,
    /// Constraint violation tolerance
    pub constraint_tol: f64,
    /// Voltage magnitude tolerance (p.u.)
    pub voltage_tol: f64,
    /// Voltage angle tolerance (degrees)
    pub angle_tol_deg: f64,
}

impl Default for BenchmarkTolerances {
    fn default() -> Self {
        Self {
            obj_tol: 1e-6,
            constraint_tol: 1e-4,
            voltage_tol: 1e-4,
            angle_tol_deg: 0.01, // ~0.01 degrees
        }
    }
}

impl BenchmarkTolerances {
    /// Create from CLI arguments with defaults
    pub fn from_args(
        obj_tol: Option<f64>,
        constraint_tol: Option<f64>,
        voltage_tol: Option<f64>,
    ) -> Self {
        let defaults = Self::default();
        Self {
            obj_tol: obj_tol.unwrap_or(defaults.obj_tol),
            constraint_tol: constraint_tol.unwrap_or(defaults.constraint_tol),
            voltage_tol: voltage_tol.unwrap_or(defaults.voltage_tol),
            angle_tol_deg: defaults.angle_tol_deg,
        }
    }
}
