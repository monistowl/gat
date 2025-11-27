//! IPOPT-based AC-OPF Solver
//!
//! Uses the IPOPT (Interior Point OPTimizer) library for solving the nonlinear
//! AC Optimal Power Flow problem. IPOPT provides faster convergence than penalty
//! methods by using second-order Newton methods with analytical Hessians.
//!
//! ## Advantages over Penalty Method
//!
//! - **Faster convergence**: Second-order Newton vs first-order L-BFGS
//! - **Better constraint handling**: Barrier methods handle inequalities natively
//! - **Sparse linear algebra**: Efficient for large networks
//! - **Mature implementation**: Well-tested on power system problems

#![cfg(feature = "solver-ipopt")]

use super::{hessian, AcOpfProblem};
use crate::opf::{OpfMethod, OpfSolution};
use crate::OpfError;
use ipopt::{BasicProblem, ConstrainedProblem, Index, Ipopt, Number, SolveStatus};

/// IPOPT problem wrapper for AC-OPF.
///
/// Implements the `ipopt::BasicProblem` and `ipopt::ConstrainedProblem` traits to provide:
/// - Variable bounds
/// - Constraint bounds
/// - Objective evaluation
/// - Gradient evaluation
/// - Constraint evaluation
/// - Jacobian evaluation (sparse)
pub struct IpoptAcOpf<'a> {
    /// Reference to the AC-OPF problem definition
    problem: &'a AcOpfProblem,
}

impl<'a> IpoptAcOpf<'a> {
    /// Create new IPOPT problem wrapper.
    pub fn new(problem: &'a AcOpfProblem) -> Self {
        Self { problem }
    }

    /// Get number of equality constraints (2*n_bus + 1)
    fn n_constraints(&self) -> usize {
        // Power balance: n_bus P equations + n_bus Q equations
        // Reference angle: 1 equation
        2 * self.problem.n_bus + 1
    }
}

impl<'a> BasicProblem for IpoptAcOpf<'a> {
    fn num_variables(&self) -> usize {
        self.problem.n_var
    }

    fn bounds(&self, x_l: &mut [Number], x_u: &mut [Number]) -> bool {
        // Initialize to large bounds
        for i in 0..self.problem.n_var {
            x_l[i] = -1e20;
            x_u[i] = 1e20;
        }

        // Voltage bounds
        for (i, bus) in self.problem.buses.iter().enumerate() {
            x_l[self.problem.v_offset + i] = bus.v_min;
            x_u[self.problem.v_offset + i] = bus.v_max;
        }

        // Angle bounds (typically ±π/2 for numerical stability)
        for i in 0..self.problem.n_bus {
            x_l[self.problem.theta_offset + i] = -std::f64::consts::FRAC_PI_2;
            x_u[self.problem.theta_offset + i] = std::f64::consts::FRAC_PI_2;
        }

        // Generator P bounds
        for (i, gen) in self.problem.generators.iter().enumerate() {
            x_l[self.problem.pg_offset + i] = gen.pmin_mw / self.problem.base_mva;
            x_u[self.problem.pg_offset + i] = gen.pmax_mw / self.problem.base_mva;
        }

        // Generator Q bounds
        for (i, gen) in self.problem.generators.iter().enumerate() {
            x_l[self.problem.qg_offset + i] = gen.qmin_mvar / self.problem.base_mva;
            x_u[self.problem.qg_offset + i] = gen.qmax_mvar / self.problem.base_mva;
        }

        true
    }

    fn initial_point(&self, x: &mut [Number]) -> bool {
        let x0 = self.problem.initial_point();
        x.copy_from_slice(&x0);
        true
    }

    fn objective(&self, x: &[Number], _new_x: bool, obj: &mut Number) -> bool {
        *obj = self.problem.objective(x);
        true
    }

    fn objective_grad(&self, x: &[Number], _new_x: bool, grad_f: &mut [Number]) -> bool {
        // Finite difference gradient for now
        // TODO: Implement analytical gradient
        let eps = 1e-7;
        let n = x.len();
        let f0 = self.problem.objective(x);

        let mut x_plus = x.to_vec();
        for i in 0..n {
            x_plus[i] = x[i] + eps;
            grad_f[i] = (self.problem.objective(&x_plus) - f0) / eps;
            x_plus[i] = x[i]; // Reset for next iteration
        }

        true
    }
}

impl<'a> ConstrainedProblem for IpoptAcOpf<'a> {
    fn num_constraints(&self) -> usize {
        self.n_constraints()
    }

    fn num_constraint_jacobian_non_zeros(&self) -> usize {
        // Dense Jacobian for now (all entries non-zero)
        // TODO: Implement sparse pattern
        self.n_constraints() * self.problem.n_var
    }

    fn constraint_bounds(&self, g_l: &mut [Number], g_u: &mut [Number]) -> bool {
        // All equality constraints: g(x) = 0
        for i in 0..self.n_constraints() {
            g_l[i] = 0.0;
            g_u[i] = 0.0;
        }
        true
    }

    fn constraint(&self, x: &[Number], _new_x: bool, g: &mut [Number]) -> bool {
        let constraints = self.problem.equality_constraints(x);
        g.copy_from_slice(&constraints);
        true
    }

    fn constraint_jacobian_indices(&self, irow: &mut [Index], jcol: &mut [Index]) -> bool {
        // Dense Jacobian for now (all entries non-zero)
        // TODO: Implement sparse pattern
        let nc = self.n_constraints();
        let nv = self.problem.n_var;

        let mut idx = 0;
        for i in 0..nc {
            for j in 0..nv {
                irow[idx] = i as Index;
                jcol[idx] = j as Index;
                idx += 1;
            }
        }

        true
    }

    fn constraint_jacobian_values(&self, x: &[Number], _new_x: bool, vals: &mut [Number]) -> bool {
        // Finite difference Jacobian for now
        // TODO: Implement analytical Jacobian
        let nc = self.n_constraints();
        let nv = self.problem.n_var;
        let eps = 1e-7;

        let g0 = self.problem.equality_constraints(x);
        let mut x_plus = x.to_vec();

        let mut idx = 0;
        for j in 0..nv {
            x_plus[j] = x[j] + eps;
            let g_plus = self.problem.equality_constraints(&x_plus);

            for i in 0..nc {
                vals[idx] = (g_plus[i] - g0[i]) / eps;
                idx += 1;
            }

            x_plus[j] = x[j]; // Reset for next iteration
        }

        true
    }

    fn num_hessian_non_zeros(&self) -> usize {
        // Use analytical Hessian for second-order convergence
        let (rows, _) = hessian::hessian_sparsity(self.problem);
        rows.len()
    }

    fn hessian_indices(&self, irow: &mut [Index], jcol: &mut [Index]) -> bool {
        let (rows, cols) = hessian::hessian_sparsity(self.problem);
        for (i, (&r, &c)) in rows.iter().zip(cols.iter()).enumerate() {
            irow[i] = r as Index;
            jcol[i] = c as Index;
        }
        true
    }

    fn hessian_values(
        &self,
        x: &[Number],
        _new_x: bool,
        obj_factor: Number,
        lambda: &[Number],
        vals: &mut [Number],
    ) -> bool {
        // Compute analytical Hessian of the Lagrangian
        let hess_vals = hessian::hessian_values(self.problem, x, obj_factor, lambda);
        vals[..hess_vals.len()].copy_from_slice(&hess_vals);
        true
    }
}

/// Solve AC-OPF using IPOPT.
///
/// # Arguments
/// * `problem` - AC-OPF problem definition
/// * `max_iter` - Maximum iterations (default: 200)
/// * `tol` - Convergence tolerance (default: 1e-6)
///
/// # Returns
/// * `Ok(OpfSolution)` - Optimal solution with dispatch and voltages
/// * `Err(OpfError)` - Solver failed or didn't converge
#[cfg(feature = "solver-ipopt")]
pub fn solve_with_ipopt(
    problem: &AcOpfProblem,
    max_iter: Option<usize>,
    tol: Option<f64>,
) -> Result<OpfSolution, OpfError> {
    let ipopt_problem = IpoptAcOpf::new(problem);

    let mut solver = Ipopt::new(ipopt_problem)
        .map_err(|e| OpfError::NumericalIssue(format!("IPOPT init failed: {}", e)))?;

    // Configure solver options
    solver.set_option("max_iter", max_iter.unwrap_or(200) as i32);
    solver.set_option("tol", tol.unwrap_or(1e-6));
    solver.set_option("print_level", 0); // Quiet mode
    solver.set_option("sb", "yes"); // Suppress banner

    // Use exact Hessian for second-order (Newton) convergence
    // This is faster than limited-memory BFGS for most power system sizes
    solver.set_option("hessian_approximation", "exact");

    // Solve
    let result = solver.solve();

    match result.status {
        SolveStatus::SolveSucceeded | SolveStatus::SolvedToAcceptableLevel => {
            let x = &result.solver_data.solution.primal_variables;

            // Extract solution components
            let (v, theta) = problem.extract_v_theta(x);

            let mut solution = OpfSolution {
                converged: true,
                method_used: OpfMethod::AcOpf,
                iterations: 0,    // TODO: Track via intermediate callback
                solve_time_ms: 0, // IPOPT doesn't expose timing easily
                objective_value: problem.objective(x),
                ..Default::default()
            };

            // Extract generator dispatch (convert from per-unit to MW/MVAr)
            for (i, gen) in problem.generators.iter().enumerate() {
                let pg_mw = x[problem.pg_offset + i] * problem.base_mva;
                let qg_mvar = x[problem.qg_offset + i] * problem.base_mva;
                solution.generator_p.insert(gen.name.clone(), pg_mw);
                solution.generator_q.insert(gen.name.clone(), qg_mvar);
            }

            // Extract bus voltages (magnitude in p.u., angle in degrees)
            for (i, bus) in problem.buses.iter().enumerate() {
                solution.bus_voltage_mag.insert(bus.name.clone(), v[i]);
                solution
                    .bus_voltage_ang
                    .insert(bus.name.clone(), theta[i].to_degrees());
            }

            // Estimate system LMP from marginal generator
            let mut system_lmp = 0.0;
            for (i, gen) in problem.generators.iter().enumerate() {
                let pg_mw = x[problem.pg_offset + i] * problem.base_mva;
                let at_min = (pg_mw - gen.pmin_mw).abs() < 1.0;
                let at_max = (pg_mw - gen.pmax_mw).abs() < 1.0;

                // Marginal generator: not at either limit
                if !at_min && !at_max {
                    let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
                    let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
                    // Marginal cost = c₁ + 2·c₂·P
                    system_lmp = c1 + 2.0 * c2 * pg_mw;
                    break;
                }
            }

            // Set all bus LMPs to system LMP (simplified - true LMPs vary by bus)
            for bus in problem.buses.iter() {
                solution.bus_lmp.insert(bus.name.clone(), system_lmp);
            }

            Ok(solution)
        }
        _ => Err(OpfError::NumericalIssue(format!(
            "IPOPT failed with status: {:?}",
            result.status
        ))),
    }
}

#[cfg(test)]
#[cfg(feature = "solver-ipopt")]
mod tests {
    use super::*;

    #[test]
    fn test_ipopt_problem_creation() {
        // This test just verifies the wrapper compiles correctly
        // Full integration tests require a valid network
    }
}
