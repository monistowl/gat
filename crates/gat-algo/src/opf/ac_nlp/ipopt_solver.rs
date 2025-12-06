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

use super::{hessian, jacobian, AcOpfProblem};
use crate::opf::{OpfMethod, OpfSolution};
use crate::OpfError;
use gat_ipopt_sys::{BasicProblem, ConstrainedProblem, Index, Ipopt, Number, SolveStatus};

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
    fn n_equality_constraints(&self) -> usize {
        // Power balance: n_bus P equations + n_bus Q equations
        // Reference angle: 1 equation
        2 * self.problem.n_bus + 1
    }

    /// Get number of inequality constraints (2 per branch with thermal limit)
    fn n_inequality_constraints(&self) -> usize {
        // Each thermally-constrained branch has 2 constraints (from and to sides)
        2 * self.problem.n_thermal_constrained_branches()
    }

    /// Get total number of constraints
    fn n_constraints(&self) -> usize {
        self.n_equality_constraints() + self.n_inequality_constraints()
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
            x_l[self.problem.pg_offset + i] = gen.pmin / self.problem.base_mva;
            x_u[self.problem.pg_offset + i] = gen.pmax / self.problem.base_mva;
        }

        // Generator Q bounds
        for (i, gen) in self.problem.generators.iter().enumerate() {
            x_l[self.problem.qg_offset + i] = gen.qmin / self.problem.base_mva;
            x_u[self.problem.qg_offset + i] = gen.qmax / self.problem.base_mva;
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
        // Use analytical gradient for performance (O(n_gen) vs O(n_var) for finite-diff)
        // The objective only depends on generator P, so most entries are zero
        let grad = self.problem.objective_gradient(x);
        grad_f.copy_from_slice(&grad);
        true
    }
}

impl<'a> ConstrainedProblem for IpoptAcOpf<'a> {
    fn num_constraints(&self) -> usize {
        self.n_constraints()
    }

    fn num_constraint_jacobian_non_zeros(&self) -> usize {
        // Use analytical sparse Jacobian
        jacobian::jacobian_nnz(self.problem)
    }

    fn constraint_bounds(&self, g_l: &mut [Number], g_u: &mut [Number]) -> bool {
        let n_eq = self.n_equality_constraints();

        // Equality constraints: g(x) = 0
        for i in 0..n_eq {
            g_l[i] = 0.0;
            g_u[i] = 0.0;
        }

        // Inequality constraints (thermal limits): h(x) ≤ 0
        // IPOPT expects: g_l ≤ g(x) ≤ g_u
        // For h(x) ≤ 0, we set g_l = -∞, g_u = 0
        for i in n_eq..self.n_constraints() {
            g_l[i] = f64::NEG_INFINITY;
            g_u[i] = 0.0;
        }
        true
    }

    fn constraint(&self, x: &[Number], _new_x: bool, g: &mut [Number]) -> bool {
        // Equality constraints (power balance + reference angle)
        let eq_constraints = self.problem.equality_constraints(x);
        let n_eq = eq_constraints.len();
        g[..n_eq].copy_from_slice(&eq_constraints);

        // Inequality constraints (thermal limits)
        let ineq_constraints = self.problem.thermal_constraints(x);
        if !ineq_constraints.is_empty() {
            g[n_eq..n_eq + ineq_constraints.len()].copy_from_slice(&ineq_constraints);
        }
        true
    }

    fn constraint_jacobian_indices(&self, irow: &mut [Index], jcol: &mut [Index]) -> bool {
        // Use analytical sparse Jacobian pattern
        let (rows, cols) = jacobian::jacobian_sparsity(self.problem);
        for (i, (&r, &c)) in rows.iter().zip(cols.iter()).enumerate() {
            irow[i] = r as Index;
            jcol[i] = c as Index;
        }
        true
    }

    fn constraint_jacobian_values(&self, x: &[Number], _new_x: bool, vals: &mut [Number]) -> bool {
        // Use analytical Jacobian for better accuracy and performance
        let jac_vals = jacobian::jacobian_values(self.problem, x);
        vals[..jac_vals.len()].copy_from_slice(&jac_vals);
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
    solver.set_option("max_iter", max_iter.unwrap_or(500) as i32);
    solver.set_option("tol", tol.unwrap_or(1e-6));
    // Print level: 0=quiet, 3=medium, 5=verbose
    let print_level = std::env::var("IPOPT_PRINT_LEVEL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    solver.set_option("print_level", print_level);
    solver.set_option("sb", if print_level > 0 { "no" } else { "yes" }); // Suppress banner

    // Derivative test: check analytical Jacobian/Hessian against finite differences
    // Set IPOPT_DERIVATIVE_TEST=first-order (Jacobian only) or second-order (both)
    if let Ok(deriv_test) = std::env::var("IPOPT_DERIVATIVE_TEST") {
        solver.set_option("derivative_test", deriv_test.as_str());
        solver.set_option("derivative_test_print_all", "yes");
        solver.set_option("derivative_test_tol", 1e-4);
    }

    // L-BFGS mode: use quasi-Newton approximation instead of exact Hessian
    // Set IPOPT_LBFGS=1 to enable (useful for debugging Hessian issues)
    if std::env::var("IPOPT_LBFGS").is_ok() {
        solver.set_option("hessian_approximation", "limited-memory");
    }

    // Use exact Hessian (default). The Hessian includes:
    // - Objective: quadratic generator cost terms (∂²f/∂Pg²)
    // - Power balance: nodal injection second derivatives (∂²g/∂V∂V, ∂²g/∂V∂θ, ∂²g/∂θ∂θ)
    // - Thermal constraints: branch flow squared magnitude Hessians
    // This enables quadratic convergence for better performance on large networks.

    // NLP scaling helps with ill-conditioned problems (power balance in p.u.)
    solver.set_option("nlp_scaling_method", "gradient-based");

    // Barrier parameter tuning for power systems
    // mu_init=1e-4 matches PowerModels.jl warm-start settings
    // Smaller mu starts closer to central path, helps with near-feasible points
    solver.set_option("mu_strategy", "adaptive");
    solver.set_option("mu_init", 1e-4);

    // Accept solutions that are "good enough" when optimal is hard to reach
    solver.set_option("acceptable_tol", 1e-4);
    solver.set_option("acceptable_iter", 10);

    // Bound relaxation helps with tight voltage/angle bounds
    solver.set_option("bound_relax_factor", 1e-8);

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
                let at_min = (pg_mw - gen.pmin).abs() < 1.0;
                let at_max = (pg_mw - gen.pmax).abs() < 1.0;

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

// ============================================================================
// WARM-START SUPPORT FOR IPOPT
// ============================================================================
//
// These functions enable warm-starting IPOPT from SOCP solutions, which
// dramatically improves convergence speed and reliability.
//
// The key insight is that SOCP provides a near-feasible AC solution,
// so IPOPT can refine it rather than starting from scratch.

use crate::opf::{DcWarmStart, SocpWarmStart};

/// Configuration for IPOPT solver with warm-start support.
#[derive(Debug, Clone)]
#[cfg(feature = "solver-ipopt")]
pub struct IpoptConfig {
    /// Maximum iterations (default: 200)
    pub max_iter: i32,
    /// Convergence tolerance (default: 1e-6)
    pub tol: f64,
    /// Enable warm-start mode (default: true when initial point provided)
    pub warm_start: bool,
    /// Bound push for warm-start (default: 1e-6)
    /// Controls how close to bounds the initial point can be
    pub warm_start_bound_push: f64,
    /// Bound fraction for warm-start (default: 1e-6)
    pub warm_start_bound_frac: f64,
    /// Slack bound push (default: 1e-6)
    pub warm_start_slack_bound_push: f64,
    /// Print level (0 = quiet, 5 = verbose)
    pub print_level: i32,
    /// Use exact Hessian (faster) vs L-BFGS approximation
    pub use_exact_hessian: bool,
}

#[cfg(feature = "solver-ipopt")]
impl Default for IpoptConfig {
    fn default() -> Self {
        Self {
            max_iter: 500,
            tol: 1e-6,
            warm_start: false,
            warm_start_bound_push: 1e-6,
            warm_start_bound_frac: 1e-6,
            warm_start_slack_bound_push: 1e-6,
            print_level: 0,
            use_exact_hessian: true, // Exact Hessian with thermal constraints
        }
    }
}

#[cfg(feature = "solver-ipopt")]
impl IpoptConfig {
    /// Create config optimized for warm-starting from SOCP.
    ///
    /// IMPORTANT: Like DC warm-start, we DON'T enable IPOPT's warm_start options
    /// because the SOCP relaxation solution is too far from AC-feasibility.
    /// SOCP uses w = v² variables and relaxes power balance constraints, so the
    /// solution may violate AC constraints significantly. IPOPT's barrier
    /// parameter tuning works better with normal initialization.
    ///
    /// The benefit of SOCP warm-start is the initial point itself (voltages,
    /// angles, and dispatch), not the IPOPT warm-start heuristics.
    pub fn warm_start_from_socp() -> Self {
        Self {
            max_iter: 500, // More iterations - SOCP point may need more work
            tol: 1e-6,
            warm_start: false, // IMPORTANT: Don't use IPOPT warm-start options
            warm_start_bound_push: 1e-6,
            warm_start_bound_frac: 1e-6,
            warm_start_slack_bound_push: 1e-6,
            print_level: 0,
            use_exact_hessian: true, // Exact Hessian with thermal constraints
        }
    }

    /// Create config optimized for warm-starting from DC-OPF.
    ///
    /// DC warm-start provides generator dispatch and angles, but not
    /// voltage magnitudes or reactive power. Unlike SOCP warm-start,
    /// we DON'T enable IPOPT's warm_start options because the DC
    /// solution is too far from the AC solution - IPOPT's barrier
    /// parameter tuning works better with normal initialization.
    ///
    /// The benefit of DC warm-start is the initial point itself (angles
    /// and dispatch), not the IPOPT warm-start heuristics.
    pub fn warm_start_from_dc() -> Self {
        Self {
            max_iter: 200, // Fewer iterations needed with exact Hessian
            tol: 1e-6,
            warm_start: false, // IMPORTANT: Don't use IPOPT warm-start options
            warm_start_bound_push: 1e-6,
            warm_start_bound_frac: 1e-6,
            warm_start_slack_bound_push: 1e-6,
            print_level: 0,
            use_exact_hessian: true, // Exact Hessian with thermal constraints
        }
    }
}

/// Convert SOCP warm-start to IPOPT initial point.
///
/// Maps the SOCP solution values to the AC-OPF variable vector:
/// - Voltage magnitudes: directly from SOCP (in p.u.)
/// - Voltage angles: from SOCP (convert degrees → radians)
/// - Generator P: from SOCP (convert MW → p.u.)
/// - Generator Q: from SOCP (convert MVAr → p.u.)
///
/// # Arguments
/// * `warm_start` - SOCP solution data
/// * `problem` - AC-OPF problem definition (provides variable ordering)
///
/// # Returns
/// Initial point vector for IPOPT.
#[cfg(feature = "solver-ipopt")]
pub fn warm_start_from_socp(warm_start: &SocpWarmStart, problem: &AcOpfProblem) -> Vec<f64> {
    let mut x = vec![0.0; problem.n_var];

    // Voltage magnitudes (SOCP stores in p.u., IPOPT expects p.u.)
    for (i, bus) in problem.buses.iter().enumerate() {
        let vm = warm_start
            .bus_voltage_mag
            .get(&bus.name)
            .copied()
            .unwrap_or(1.0);
        // Clamp to bounds for numerical safety
        let vm_clamped = vm.max(bus.v_min).min(bus.v_max);
        x[problem.v_offset + i] = vm_clamped;
    }

    // Voltage angles (SOCP stores in degrees, IPOPT expects radians)
    for (i, bus) in problem.buses.iter().enumerate() {
        let va_deg = warm_start
            .bus_voltage_angle
            .get(&bus.name)
            .copied()
            .unwrap_or(0.0);
        let va_rad = va_deg.to_radians();
        // Clamp to bounds
        let va_clamped = va_rad
            .max(-std::f64::consts::FRAC_PI_2)
            .min(std::f64::consts::FRAC_PI_2);
        x[problem.theta_offset + i] = va_clamped;
    }

    // Generator real power (SOCP stores in MW, IPOPT expects p.u.)
    for (i, gen) in problem.generators.iter().enumerate() {
        let pg_mw = warm_start
            .generator_p
            .get(&gen.name)
            .copied()
            .unwrap_or(0.0);
        let pg_pu = pg_mw / problem.base_mva;
        // Clamp to bounds
        let pg_min = gen.pmin / problem.base_mva;
        let pg_max = gen.pmax / problem.base_mva;
        let pg_clamped = pg_pu.max(pg_min).min(pg_max);
        x[problem.pg_offset + i] = pg_clamped;
    }

    // Generator reactive power (SOCP stores in MVAr, IPOPT expects p.u.)
    for (i, gen) in problem.generators.iter().enumerate() {
        let qg_mvar = warm_start
            .generator_q
            .get(&gen.name)
            .copied()
            .unwrap_or(0.0);
        let qg_pu = qg_mvar / problem.base_mva;
        // Clamp to bounds
        let qg_min = gen.qmin / problem.base_mva;
        let qg_max = gen.qmax / problem.base_mva;
        let qg_clamped = qg_pu.max(qg_min).min(qg_max);
        x[problem.qg_offset + i] = qg_clamped;
    }

    x
}

/// Convert DC-OPF warm-start to IPOPT initial point.
///
/// DC-OPF provides angles and generator real power, but not voltage magnitudes
/// or reactive power (since DC-OPF ignores losses and reactive power).
///
/// The mapping is:
/// - Voltage magnitudes: flat start at 1.0 p.u. (DC assumption)
/// - Voltage angles: from DC solution (already in radians)
/// - Generator P: from DC solution (convert MW → p.u.)
/// - Generator Q: initialized to middle of bounds (DC-OPF doesn't compute Q)
///
/// This provides a better starting point than flat start by getting the
/// generator dispatch and angle differences approximately correct.
///
/// # Arguments
/// * `warm_start` - DC-OPF solution data
/// * `problem` - AC-OPF problem definition (provides variable ordering)
///
/// # Returns
/// Initial point vector for IPOPT.
#[cfg(feature = "solver-ipopt")]
pub fn warm_start_from_dc(warm_start: &DcWarmStart, problem: &AcOpfProblem) -> Vec<f64> {
    let mut x = vec![0.0; problem.n_var];

    // Voltage magnitudes: DC-OPF assumes V = 1.0 everywhere
    // Use middle of bounds for better starting point
    for (i, bus) in problem.buses.iter().enumerate() {
        let v_mid = (bus.v_min + bus.v_max) / 2.0;
        x[problem.v_offset + i] = v_mid;
    }

    // Voltage angles: from DC solution (radians)
    // Note: DC-OPF stores angles in radians
    for (i, bus) in problem.buses.iter().enumerate() {
        let theta = warm_start.bus_angles.get(&bus.name).copied().unwrap_or(0.0);
        // Clamp to angle bounds
        let theta_clamped = theta
            .max(-std::f64::consts::FRAC_PI_2)
            .min(std::f64::consts::FRAC_PI_2);
        x[problem.theta_offset + i] = theta_clamped;
    }

    // Generator real power: from DC solution (MW → p.u.)
    for (i, gen) in problem.generators.iter().enumerate() {
        let pg_mw = warm_start
            .generator_p
            .get(&gen.name)
            .copied()
            .unwrap_or(0.0);
        let pg_pu = pg_mw / problem.base_mva;
        // Clamp to bounds
        let pg_min = gen.pmin / problem.base_mva;
        let pg_max = gen.pmax / problem.base_mva;
        let pg_clamped = pg_pu.max(pg_min).min(pg_max);
        x[problem.pg_offset + i] = pg_clamped;
    }

    // Generator reactive power: DC-OPF doesn't compute Q
    // Initialize to middle of bounds for robustness
    for (i, gen) in problem.generators.iter().enumerate() {
        let qg_min = gen.qmin / problem.base_mva;
        let qg_max = gen.qmax / problem.base_mva;
        let qg_mid = (qg_min + qg_max) / 2.0;
        x[problem.qg_offset + i] = qg_mid;
    }

    x
}

/// Solve AC-OPF using IPOPT with warm-start from DC-OPF.
///
/// DC-OPF provides a fast approximation that captures the essential dispatch
/// pattern (generator outputs, angle differences) without the complexity of
/// full AC power flow. This makes it a good initialization for IPOPT.
///
/// Benefits over flat start:
/// - Generator dispatch is already near-optimal for MW balance
/// - Angle differences reflect approximate power flows
/// - Much faster to compute than SOCP warm-start
///
/// Limitations:
/// - Voltage magnitudes start at flat (1.0 p.u.)
/// - Reactive power starts at middle of bounds
/// - May not help for voltage-constrained problems
///
/// # Arguments
/// * `problem` - AC-OPF problem definition
/// * `warm_start` - DC-OPF solution to initialize from
/// * `config` - IPOPT configuration
///
/// # Returns
/// * `Ok(OpfSolution)` - Optimal solution
/// * `Err(OpfError)` - Solver failed
#[cfg(feature = "solver-ipopt")]
pub fn solve_with_dc_warm_start(
    problem: &AcOpfProblem,
    warm_start: &DcWarmStart,
    config: &IpoptConfig,
) -> Result<OpfSolution, OpfError> {
    // Create initial point from DC solution
    let x0 = warm_start_from_dc(warm_start, problem);

    // Create IPOPT wrapper with custom initial point
    let ipopt_problem = IpoptAcOpfWarmStart::new(problem, x0);

    let mut solver = Ipopt::new(ipopt_problem)
        .map_err(|e| OpfError::NumericalIssue(format!("IPOPT init failed: {}", e)))?;

    // Configure solver - same options as SOCP warm-start
    solver.set_option("max_iter", config.max_iter);
    solver.set_option("tol", config.tol);
    solver.set_option("print_level", config.print_level);
    solver.set_option("sb", "yes");

    // Warm-start options
    if config.warm_start {
        solver.set_option("warm_start_init_point", "yes");
        solver.set_option("warm_start_bound_push", config.warm_start_bound_push);
        solver.set_option("warm_start_bound_frac", config.warm_start_bound_frac);
        solver.set_option(
            "warm_start_slack_bound_push",
            config.warm_start_slack_bound_push,
        );
    }

    // Hessian approximation
    if config.use_exact_hessian {
        solver.set_option("hessian_approximation", "exact");
    } else {
        solver.set_option("hessian_approximation", "limited-memory");
    }

    // NLP scaling and barrier tuning
    // mu_init=1e-4 matches PowerModels.jl warm-start settings
    solver.set_option("nlp_scaling_method", "gradient-based");
    solver.set_option("mu_strategy", "adaptive");
    solver.set_option("mu_init", 1e-4);
    solver.set_option("acceptable_tol", 1e-4);
    solver.set_option("acceptable_iter", 10);
    solver.set_option("bound_relax_factor", 1e-8);

    // Solve
    let result = solver.solve();

    match result.status {
        SolveStatus::SolveSucceeded | SolveStatus::SolvedToAcceptableLevel => {
            let x = &result.solver_data.solution.primal_variables;
            extract_solution(problem, x)
        }
        _ => Err(OpfError::NumericalIssue(format!(
            "IPOPT failed with status: {:?}",
            result.status
        ))),
    }
}

/// Solve AC-OPF using IPOPT with warm-start from SOCP.
///
/// This is the recommended approach for production use:
/// 1. Solve SOCP to get a near-feasible initial point
/// 2. Refine with IPOPT using that initial point
///
/// Benefits:
/// - Faster convergence (fewer iterations)
/// - Higher reliability (starting closer to solution)
/// - Better solutions (avoids poor local minima)
///
/// # Arguments
/// * `problem` - AC-OPF problem definition
/// * `warm_start` - SOCP solution to initialize from
/// * `config` - IPOPT configuration (use IpoptConfig::warm_start_from_socp())
///
/// # Returns
/// * `Ok(OpfSolution)` - Optimal solution
/// * `Err(OpfError)` - Solver failed
///
/// # Example
/// ```ignore
/// // First solve SOCP
/// let socp_solution = socp::solve(&network)?;
/// let warm_start = SocpWarmStart::from(&socp_solution);
///
/// // Then refine with IPOPT
/// let problem = AcOpfProblem::from_network(&network)?;
/// let config = IpoptConfig::warm_start_from_socp();
/// let ac_solution = solve_with_warm_start(&problem, &warm_start, &config)?;
/// ```
#[cfg(feature = "solver-ipopt")]
pub fn solve_with_socp_warm_start(
    problem: &AcOpfProblem,
    warm_start: &SocpWarmStart,
    config: &IpoptConfig,
) -> Result<OpfSolution, OpfError> {
    // Create initial point from SOCP solution
    let x0 = warm_start_from_socp(warm_start, problem);

    // Create IPOPT wrapper with custom initial point
    let ipopt_problem = IpoptAcOpfWarmStart::new(problem, x0);

    let mut solver = Ipopt::new(ipopt_problem)
        .map_err(|e| OpfError::NumericalIssue(format!("IPOPT init failed: {}", e)))?;

    // Configure solver with warm-start options
    solver.set_option("max_iter", config.max_iter);
    solver.set_option("tol", config.tol);
    solver.set_option("print_level", config.print_level);
    solver.set_option("sb", "yes"); // Suppress banner

    // Warm-start options - critical for good performance
    if config.warm_start {
        solver.set_option("warm_start_init_point", "yes");
        solver.set_option("warm_start_bound_push", config.warm_start_bound_push);
        solver.set_option("warm_start_bound_frac", config.warm_start_bound_frac);
        solver.set_option(
            "warm_start_slack_bound_push",
            config.warm_start_slack_bound_push,
        );
    }

    // Hessian approximation
    if config.use_exact_hessian {
        solver.set_option("hessian_approximation", "exact");
    } else {
        solver.set_option("hessian_approximation", "limited-memory");
    }

    // NLP scaling helps with ill-conditioned problems
    solver.set_option("nlp_scaling_method", "gradient-based");

    // Barrier parameter tuning for power systems
    // mu_init=1e-4 matches PowerModels.jl warm-start settings
    solver.set_option("mu_strategy", "adaptive");
    solver.set_option("mu_init", 1e-4);

    // Accept solutions that are "good enough"
    solver.set_option("acceptable_tol", 1e-4);
    solver.set_option("acceptable_iter", 10);

    // Bound relaxation
    solver.set_option("bound_relax_factor", 1e-8);

    // Solve
    let result = solver.solve();

    match result.status {
        SolveStatus::SolveSucceeded | SolveStatus::SolvedToAcceptableLevel => {
            let x = &result.solver_data.solution.primal_variables;
            extract_solution(problem, x)
        }
        _ => Err(OpfError::NumericalIssue(format!(
            "IPOPT failed with status: {:?}",
            result.status
        ))),
    }
}

/// Helper to extract OpfSolution from IPOPT result vector.
#[cfg(feature = "solver-ipopt")]
fn extract_solution(problem: &AcOpfProblem, x: &[f64]) -> Result<OpfSolution, OpfError> {
    let (v, theta) = problem.extract_v_theta(x);

    let mut solution = OpfSolution {
        converged: true,
        method_used: OpfMethod::AcOpf,
        iterations: 0,
        solve_time_ms: 0,
        objective_value: problem.objective(x),
        ..Default::default()
    };

    // Extract generator dispatch
    for (i, gen) in problem.generators.iter().enumerate() {
        let pg_mw = x[problem.pg_offset + i] * problem.base_mva;
        let qg_mvar = x[problem.qg_offset + i] * problem.base_mva;
        solution.generator_p.insert(gen.name.clone(), pg_mw);
        solution.generator_q.insert(gen.name.clone(), qg_mvar);
    }

    // Extract bus voltages
    for (i, bus) in problem.buses.iter().enumerate() {
        solution.bus_voltage_mag.insert(bus.name.clone(), v[i]);
        solution
            .bus_voltage_ang
            .insert(bus.name.clone(), theta[i].to_degrees());
    }

    // Estimate LMP
    let mut system_lmp = 0.0;
    for (i, gen) in problem.generators.iter().enumerate() {
        let pg_mw = x[problem.pg_offset + i] * problem.base_mva;
        let at_min = (pg_mw - gen.pmin).abs() < 1.0;
        let at_max = (pg_mw - gen.pmax).abs() < 1.0;

        if !at_min && !at_max {
            let c1 = gen.cost_coeffs.get(1).copied().unwrap_or(0.0);
            let c2 = gen.cost_coeffs.get(2).copied().unwrap_or(0.0);
            system_lmp = c1 + 2.0 * c2 * pg_mw;
            break;
        }
    }

    for bus in problem.buses.iter() {
        solution.bus_lmp.insert(bus.name.clone(), system_lmp);
    }

    Ok(solution)
}

/// IPOPT wrapper with custom initial point.
#[cfg(feature = "solver-ipopt")]
pub struct IpoptAcOpfWarmStart<'a> {
    problem: &'a AcOpfProblem,
    initial_x: Vec<f64>,
}

#[cfg(feature = "solver-ipopt")]
impl<'a> IpoptAcOpfWarmStart<'a> {
    pub fn new(problem: &'a AcOpfProblem, initial_x: Vec<f64>) -> Self {
        Self { problem, initial_x }
    }

    fn n_equality_constraints(&self) -> usize {
        2 * self.problem.n_bus + 1
    }

    fn n_inequality_constraints(&self) -> usize {
        2 * self.problem.n_thermal_constrained_branches()
    }

    fn n_constraints(&self) -> usize {
        self.n_equality_constraints() + self.n_inequality_constraints()
    }
}

#[cfg(feature = "solver-ipopt")]
impl<'a> BasicProblem for IpoptAcOpfWarmStart<'a> {
    fn num_variables(&self) -> usize {
        self.problem.n_var
    }

    fn bounds(&self, x_l: &mut [Number], x_u: &mut [Number]) -> bool {
        for i in 0..self.problem.n_var {
            x_l[i] = -1e20;
            x_u[i] = 1e20;
        }

        for (i, bus) in self.problem.buses.iter().enumerate() {
            x_l[self.problem.v_offset + i] = bus.v_min;
            x_u[self.problem.v_offset + i] = bus.v_max;
        }

        for i in 0..self.problem.n_bus {
            x_l[self.problem.theta_offset + i] = -std::f64::consts::FRAC_PI_2;
            x_u[self.problem.theta_offset + i] = std::f64::consts::FRAC_PI_2;
        }

        for (i, gen) in self.problem.generators.iter().enumerate() {
            x_l[self.problem.pg_offset + i] = gen.pmin / self.problem.base_mva;
            x_u[self.problem.pg_offset + i] = gen.pmax / self.problem.base_mva;
        }

        for (i, gen) in self.problem.generators.iter().enumerate() {
            x_l[self.problem.qg_offset + i] = gen.qmin / self.problem.base_mva;
            x_u[self.problem.qg_offset + i] = gen.qmax / self.problem.base_mva;
        }

        true
    }

    fn initial_point(&self, x: &mut [Number]) -> bool {
        // Use the warm-start initial point instead of default
        x.copy_from_slice(&self.initial_x);
        true
    }

    fn objective(&self, x: &[Number], _new_x: bool, obj: &mut Number) -> bool {
        *obj = self.problem.objective(x);
        true
    }

    fn objective_grad(&self, x: &[Number], _new_x: bool, grad_f: &mut [Number]) -> bool {
        // Use analytical gradient for performance (O(n_gen) vs O(n_var) for finite-diff)
        let grad = self.problem.objective_gradient(x);
        grad_f.copy_from_slice(&grad);
        true
    }
}

#[cfg(feature = "solver-ipopt")]
impl<'a> ConstrainedProblem for IpoptAcOpfWarmStart<'a> {
    fn num_constraints(&self) -> usize {
        self.n_constraints()
    }

    fn num_constraint_jacobian_non_zeros(&self) -> usize {
        // Use analytical sparse Jacobian
        jacobian::jacobian_nnz(self.problem)
    }

    fn constraint_bounds(&self, g_l: &mut [Number], g_u: &mut [Number]) -> bool {
        let n_eq = self.n_equality_constraints();

        // Equality constraints: g(x) = 0
        for i in 0..n_eq {
            g_l[i] = 0.0;
            g_u[i] = 0.0;
        }

        // Inequality constraints (thermal limits): h(x) ≤ 0
        for i in n_eq..self.n_constraints() {
            g_l[i] = f64::NEG_INFINITY;
            g_u[i] = 0.0;
        }
        true
    }

    fn constraint(&self, x: &[Number], _new_x: bool, g: &mut [Number]) -> bool {
        // Equality constraints (power balance + reference angle)
        let eq_constraints = self.problem.equality_constraints(x);
        let n_eq = eq_constraints.len();
        g[..n_eq].copy_from_slice(&eq_constraints);

        // Inequality constraints (thermal limits)
        let ineq_constraints = self.problem.thermal_constraints(x);
        if !ineq_constraints.is_empty() {
            g[n_eq..n_eq + ineq_constraints.len()].copy_from_slice(&ineq_constraints);
        }
        true
    }

    fn constraint_jacobian_indices(&self, irow: &mut [Index], jcol: &mut [Index]) -> bool {
        // Use analytical sparse Jacobian pattern
        let (rows, cols) = jacobian::jacobian_sparsity(self.problem);
        for (i, (&r, &c)) in rows.iter().zip(cols.iter()).enumerate() {
            irow[i] = r as Index;
            jcol[i] = c as Index;
        }
        true
    }

    fn constraint_jacobian_values(&self, x: &[Number], _new_x: bool, vals: &mut [Number]) -> bool {
        // Use analytical Jacobian for better accuracy and performance
        let jac_vals = jacobian::jacobian_values(self.problem, x);
        vals[..jac_vals.len()].copy_from_slice(&jac_vals);
        true
    }

    fn num_hessian_non_zeros(&self) -> usize {
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
        let hess_vals = hessian::hessian_values(self.problem, x, obj_factor, lambda);
        vals[..hess_vals.len()].copy_from_slice(&hess_vals);
        true
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
