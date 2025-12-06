//! Safe Rust wrapper providing traits compatible with existing IPOPT integration.
//!
//! This module provides `BasicProblem` and `ConstrainedProblem` traits that mirror
//! the API from the crates.io `ipopt` crate, allowing existing code to work with
//! minimal changes.
//!
//! # Usage
//!
//! ```ignore
//! use gat_ipopt_sys::{BasicProblem, ConstrainedProblem, Ipopt, Number, Index};
//!
//! struct MyProblem { /* ... */ }
//!
//! impl BasicProblem for MyProblem {
//!     fn num_variables(&self) -> usize { 2 }
//!     fn bounds(&self, x_l: &mut [Number], x_u: &mut [Number]) -> bool { /* ... */ }
//!     // ...
//! }
//!
//! impl ConstrainedProblem for MyProblem {
//!     fn num_constraints(&self) -> usize { 1 }
//!     // ...
//! }
//!
//! let problem = MyProblem { /* ... */ };
//! let mut solver = Ipopt::new(problem)?;
//! solver.set_option("max_iter", 100);
//! let result = solver.solve();
//! ```

use crate::{
    AddIpoptIntOption, AddIpoptNumOption, AddIpoptStrOption, ApplicationReturnStatus,
    CreateIpoptProblem, FreeIpoptProblem, Index, IpoptProblem, IpoptSolve, Number, UserDataPtr,
};
use std::ffi::CString;
use std::os::raw::c_int;

/// Trait for defining the basic NLP problem structure.
///
/// Implement this trait to define your optimization problem's:
/// - Number of variables
/// - Variable bounds
/// - Initial point
/// - Objective function and gradient
pub trait BasicProblem {
    /// Returns the number of decision variables.
    fn num_variables(&self) -> usize;

    /// Sets the lower and upper bounds for variables.
    ///
    /// Use large values (e.g., 1e20) for unbounded variables.
    ///
    /// # Arguments
    /// * `x_l` - Lower bounds (length = num_variables)
    /// * `x_u` - Upper bounds (length = num_variables)
    ///
    /// # Returns
    /// `true` on success, `false` on error.
    fn bounds(&self, x_l: &mut [Number], x_u: &mut [Number]) -> bool;

    /// Sets the initial point for optimization.
    ///
    /// # Arguments
    /// * `x` - Initial variable values (length = num_variables)
    ///
    /// # Returns
    /// `true` on success, `false` on error.
    fn initial_point(&self, x: &mut [Number]) -> bool;

    /// Evaluates the objective function f(x).
    ///
    /// # Arguments
    /// * `x` - Variable values (length = num_variables)
    /// * `new_x` - True if x has changed since last call
    /// * `obj` - Output: objective value
    ///
    /// # Returns
    /// `true` on success, `false` on evaluation error.
    fn objective(&self, x: &[Number], new_x: bool, obj: &mut Number) -> bool;

    /// Evaluates the gradient of the objective function ∇f(x).
    ///
    /// # Arguments
    /// * `x` - Variable values (length = num_variables)
    /// * `new_x` - True if x has changed since last call
    /// * `grad_f` - Output: gradient values (length = num_variables)
    ///
    /// # Returns
    /// `true` on success, `false` on evaluation error.
    fn objective_grad(&self, x: &[Number], new_x: bool, grad_f: &mut [Number]) -> bool;
}

/// Trait for defining constrained NLP problems.
///
/// Implement this trait along with `BasicProblem` to add constraints.
pub trait ConstrainedProblem: BasicProblem {
    /// Returns the number of constraints.
    fn num_constraints(&self) -> usize;

    /// Returns the number of non-zeros in the constraint Jacobian.
    fn num_constraint_jacobian_non_zeros(&self) -> usize;

    /// Sets the lower and upper bounds for constraints.
    ///
    /// For equality constraints g(x) = 0, set both bounds to 0.
    /// For inequality g(x) ≤ 0, set g_l = -∞ and g_u = 0.
    ///
    /// # Arguments
    /// * `g_l` - Lower bounds (length = num_constraints)
    /// * `g_u` - Upper bounds (length = num_constraints)
    fn constraint_bounds(&self, g_l: &mut [Number], g_u: &mut [Number]) -> bool;

    /// Evaluates the constraint functions g(x).
    ///
    /// # Arguments
    /// * `x` - Variable values
    /// * `new_x` - True if x has changed since last call
    /// * `g` - Output: constraint values (length = num_constraints)
    fn constraint(&self, x: &[Number], new_x: bool, g: &mut [Number]) -> bool;

    /// Returns the sparsity structure of the constraint Jacobian.
    ///
    /// # Arguments
    /// * `irow` - Row indices (length = num_constraint_jacobian_non_zeros)
    /// * `jcol` - Column indices (length = num_constraint_jacobian_non_zeros)
    fn constraint_jacobian_indices(&self, irow: &mut [Index], jcol: &mut [Index]) -> bool;

    /// Evaluates the constraint Jacobian values.
    ///
    /// # Arguments
    /// * `x` - Variable values
    /// * `new_x` - True if x has changed since last call
    /// * `vals` - Output: Jacobian values (length = num_constraint_jacobian_non_zeros)
    fn constraint_jacobian_values(&self, x: &[Number], new_x: bool, vals: &mut [Number]) -> bool;

    /// Returns the number of non-zeros in the Hessian of the Lagrangian.
    ///
    /// The Hessian is symmetric, so only the lower triangle is stored.
    fn num_hessian_non_zeros(&self) -> usize;

    /// Returns the sparsity structure of the Hessian.
    ///
    /// # Arguments
    /// * `irow` - Row indices (lower triangle)
    /// * `jcol` - Column indices (lower triangle)
    fn hessian_indices(&self, irow: &mut [Index], jcol: &mut [Index]) -> bool;

    /// Evaluates the Hessian of the Lagrangian.
    ///
    /// Computes: σ ∇²f(x) + Σᵢ λᵢ ∇²gᵢ(x)
    ///
    /// # Arguments
    /// * `x` - Variable values
    /// * `new_x` - True if x has changed since last call
    /// * `obj_factor` - σ, scaling factor for objective Hessian
    /// * `lambda` - Constraint multipliers (length = num_constraints)
    /// * `vals` - Output: Hessian values (length = num_hessian_non_zeros)
    fn hessian_values(
        &self,
        x: &[Number],
        new_x: bool,
        obj_factor: Number,
        lambda: &[Number],
        vals: &mut [Number],
    ) -> bool;
}

/// Solve status returned by IPOPT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveStatus {
    /// Algorithm terminated successfully at a locally optimal point.
    SolveSucceeded,
    /// Algorithm terminated with a point satisfying "acceptable" level of optimality.
    SolvedToAcceptableLevel,
    /// Algorithm converged to a point of local infeasibility.
    InfeasibleProblemDetected,
    /// Algorithm stopped because search direction became too small.
    SearchDirectionBecomesTooSmall,
    /// Iterates seem to be diverging.
    DivergingIterates,
    /// User requested termination.
    UserRequestedStop,
    /// Feasible point found (for feasibility problems).
    FeasiblePointFound,
    /// Maximum iterations exceeded.
    MaximumIterationsExceeded,
    /// Restoration phase failed.
    RestorationFailed,
    /// Error in step computation.
    ErrorInStepComputation,
    /// Maximum CPU time exceeded.
    MaximumCpuTimeExceeded,
    /// Not enough degrees of freedom.
    NotEnoughDegreesOfFreedom,
    /// Invalid problem definition.
    InvalidProblemDefinition,
    /// Invalid option.
    InvalidOption,
    /// Invalid number detected.
    InvalidNumberDetected,
    /// Unrecoverable exception.
    UnrecoverableException,
    /// Non-IPOPT exception thrown.
    NonIpoptExceptionThrown,
    /// Insufficient memory.
    InsufficientMemory,
    /// Internal error.
    InternalError,
}

impl From<ApplicationReturnStatus> for SolveStatus {
    fn from(status: ApplicationReturnStatus) -> Self {
        match status {
            ApplicationReturnStatus::SolveSucceeded => SolveStatus::SolveSucceeded,
            ApplicationReturnStatus::SolvedToAcceptableLevel => {
                SolveStatus::SolvedToAcceptableLevel
            }
            ApplicationReturnStatus::InfeasibleProblemDetected => {
                SolveStatus::InfeasibleProblemDetected
            }
            ApplicationReturnStatus::SearchDirectionBecomesTooSmall => {
                SolveStatus::SearchDirectionBecomesTooSmall
            }
            ApplicationReturnStatus::DivergingIterates => SolveStatus::DivergingIterates,
            ApplicationReturnStatus::UserRequestedStop => SolveStatus::UserRequestedStop,
            ApplicationReturnStatus::FeasiblePointFound => SolveStatus::FeasiblePointFound,
            ApplicationReturnStatus::MaximumIterationsExceeded => {
                SolveStatus::MaximumIterationsExceeded
            }
            ApplicationReturnStatus::RestorationFailed => SolveStatus::RestorationFailed,
            ApplicationReturnStatus::ErrorInStepComputation => SolveStatus::ErrorInStepComputation,
            ApplicationReturnStatus::MaximumCpuTimeExceeded => SolveStatus::MaximumCpuTimeExceeded,
            ApplicationReturnStatus::MaximumWallTimeExceeded => SolveStatus::MaximumCpuTimeExceeded,
            ApplicationReturnStatus::NotEnoughDegreesOfFreedom => {
                SolveStatus::NotEnoughDegreesOfFreedom
            }
            ApplicationReturnStatus::InvalidProblemDefinition => {
                SolveStatus::InvalidProblemDefinition
            }
            ApplicationReturnStatus::InvalidOption => SolveStatus::InvalidOption,
            ApplicationReturnStatus::InvalidNumberDetected => SolveStatus::InvalidNumberDetected,
            ApplicationReturnStatus::UnrecoverableException => SolveStatus::UnrecoverableException,
            ApplicationReturnStatus::NonIpoptExceptionThrown => {
                SolveStatus::NonIpoptExceptionThrown
            }
            ApplicationReturnStatus::InsufficientMemory => SolveStatus::InsufficientMemory,
            ApplicationReturnStatus::InternalError => SolveStatus::InternalError,
        }
    }
}

/// Solution data from IPOPT solve.
pub struct SolverData {
    /// Solution values
    pub solution: Solution,
}

/// Solution values from IPOPT.
pub struct Solution {
    /// Primal variables at solution
    pub primal_variables: Vec<Number>,
    /// Constraint values at solution
    pub constraint_values: Vec<Number>,
    /// Constraint multipliers
    pub constraint_multipliers: Vec<Number>,
    /// Lower bound multipliers
    pub lower_bound_multipliers: Vec<Number>,
    /// Upper bound multipliers
    pub upper_bound_multipliers: Vec<Number>,
}

/// Result of IPOPT solve operation.
pub struct SolveResult {
    /// Solve status
    pub status: SolveStatus,
    /// Objective value at solution
    pub objective_value: Number,
    /// Solution data
    pub solver_data: SolverData,
}

/// IPOPT solver wrapper.
///
/// Wraps an IPOPT problem and provides a safe interface for setting options
/// and solving.
pub struct Ipopt<P: ConstrainedProblem> {
    problem: IpoptProblem,
    user_problem: Box<P>,
    n: usize,
    m: usize,
}

impl<P: ConstrainedProblem> Ipopt<P> {
    /// Create a new IPOPT solver for the given problem.
    ///
    /// # Errors
    /// Returns an error if the problem definition is invalid.
    pub fn new(problem: P) -> Result<Self, String> {
        let n = problem.num_variables();
        let m = problem.num_constraints();
        let nele_jac = problem.num_constraint_jacobian_non_zeros();
        let nele_hess = problem.num_hessian_non_zeros();

        // Get bounds
        let mut x_l = vec![0.0; n];
        let mut x_u = vec![0.0; n];
        if !problem.bounds(&mut x_l, &mut x_u) {
            return Err("Failed to get variable bounds".to_string());
        }

        let mut g_l = vec![0.0; m];
        let mut g_u = vec![0.0; m];
        if !problem.constraint_bounds(&mut g_l, &mut g_u) {
            return Err("Failed to get constraint bounds".to_string());
        }

        // Box the problem for stable pointer
        let user_problem = Box::new(problem);

        // Create IPOPT problem with callbacks
        let ipopt_problem = unsafe {
            CreateIpoptProblem(
                n as Index,
                x_l.as_ptr(),
                x_u.as_ptr(),
                m as Index,
                g_l.as_ptr(),
                g_u.as_ptr(),
                nele_jac as Index,
                nele_hess as Index,
                0, // C-style indexing
                eval_f_callback::<P>,
                eval_g_callback::<P>,
                eval_grad_f_callback::<P>,
                eval_jac_g_callback::<P>,
                eval_h_callback::<P>,
            )
        };

        if ipopt_problem.is_null() {
            return Err("Failed to create IPOPT problem".to_string());
        }

        Ok(Ipopt {
            problem: ipopt_problem,
            user_problem,
            n,
            m,
        })
    }

    /// Set an option (generic version that accepts strings, integers, and floats).
    ///
    /// # Examples
    /// ```ignore
    /// solver.set_option("max_iter", 100);        // integer
    /// solver.set_option("tol", 1e-6);            // float
    /// solver.set_option("linear_solver", "mumps"); // string
    /// ```
    pub fn set_option<V: IpoptOption>(&mut self, name: &str, value: V) {
        value.set_option(self, name);
    }

    /// Set a string option (explicit version).
    pub fn set_string_option(&mut self, name: &str, value: &str) {
        let name_c = CString::new(name).unwrap();
        let value_c = CString::new(value).unwrap();
        unsafe {
            AddIpoptStrOption(self.problem, name_c.as_ptr(), value_c.as_ptr());
        }
    }

    /// Set an integer option (explicit version).
    pub fn set_int_option(&mut self, name: &str, value: i32) {
        let name_c = CString::new(name).unwrap();
        unsafe {
            AddIpoptIntOption(self.problem, name_c.as_ptr(), value);
        }
    }

    /// Set a numeric option (explicit version).
    pub fn set_num_option(&mut self, name: &str, value: f64) {
        let name_c = CString::new(name).unwrap();
        unsafe {
            AddIpoptNumOption(self.problem, name_c.as_ptr(), value);
        }
    }

    /// Solve the optimization problem.
    ///
    /// # Returns
    /// `SolveResult` containing the solution status, objective value, and solution data.
    pub fn solve(self) -> SolveResult {
        let n = self.n;
        let m = self.m;

        // Allocate solution vectors
        let mut x = vec![0.0; n];
        let mut g = vec![0.0; m];
        let mut mult_g = vec![0.0; m];
        let mut mult_x_l = vec![0.0; n];
        let mut mult_x_u = vec![0.0; n];
        let mut obj_val = 0.0;

        // Get initial point
        if !self.user_problem.initial_point(&mut x) {
            return SolveResult {
                status: SolveStatus::InvalidProblemDefinition,
                objective_value: f64::NAN,
                solver_data: SolverData {
                    solution: Solution {
                        primal_variables: x,
                        constraint_values: g,
                        constraint_multipliers: mult_g,
                        lower_bound_multipliers: mult_x_l,
                        upper_bound_multipliers: mult_x_u,
                    },
                },
            };
        }

        // Get pointer to user problem for callbacks
        let user_data = self.user_problem.as_ref() as *const P as UserDataPtr;

        // Solve
        let status = unsafe {
            IpoptSolve(
                self.problem,
                x.as_mut_ptr(),
                g.as_mut_ptr(),
                &mut obj_val,
                mult_g.as_mut_ptr(),
                mult_x_l.as_mut_ptr(),
                mult_x_u.as_mut_ptr(),
                user_data,
            )
        };

        SolveResult {
            status: status.into(),
            objective_value: obj_val,
            solver_data: SolverData {
                solution: Solution {
                    primal_variables: x,
                    constraint_values: g,
                    constraint_multipliers: mult_g,
                    lower_bound_multipliers: mult_x_l,
                    upper_bound_multipliers: mult_x_u,
                },
            },
        }
    }
}

impl<P: ConstrainedProblem> Drop for Ipopt<P> {
    fn drop(&mut self) {
        if !self.problem.is_null() {
            unsafe {
                FreeIpoptProblem(self.problem);
            }
        }
    }
}

// Option setting trait for type inference
pub trait IpoptOption {
    fn set_option<P: ConstrainedProblem>(&self, solver: &mut Ipopt<P>, name: &str);
}

impl IpoptOption for i32 {
    fn set_option<P: ConstrainedProblem>(&self, solver: &mut Ipopt<P>, name: &str) {
        solver.set_int_option(name, *self);
    }
}

impl IpoptOption for f64 {
    fn set_option<P: ConstrainedProblem>(&self, solver: &mut Ipopt<P>, name: &str) {
        solver.set_num_option(name, *self);
    }
}

impl IpoptOption for &str {
    fn set_option<P: ConstrainedProblem>(&self, solver: &mut Ipopt<P>, name: &str) {
        solver.set_string_option(name, self);
    }
}

// ============================================================================
// CALLBACK TRAMPOLINES
// ============================================================================

/// Maximum reasonable problem size to prevent integer overflow in slice creation.
const MAX_PROBLEM_SIZE: usize = 10_000_000;

/// Callback for objective evaluation.
extern "C" fn eval_f_callback<P: ConstrainedProblem>(
    n: Index,
    x: *const Number,
    new_x: c_int,
    obj_value: *mut Number,
    user_data: UserDataPtr,
) -> c_int {
    // Safety checks for all pointers and sizes
    if user_data.is_null() || x.is_null() || obj_value.is_null() {
        return 0;
    }
    let n_usize = n as usize;
    if n < 0 || n_usize > MAX_PROBLEM_SIZE {
        return 0;
    }

    let problem = unsafe { &*(user_data as *const P) };
    let x_slice = unsafe { std::slice::from_raw_parts(x, n_usize) };
    let mut obj = 0.0;
    let success = problem.objective(x_slice, new_x != 0, &mut obj);
    if success {
        unsafe { *obj_value = obj };
        1
    } else {
        0
    }
}

/// Callback for gradient evaluation.
extern "C" fn eval_grad_f_callback<P: ConstrainedProblem>(
    n: Index,
    x: *const Number,
    new_x: c_int,
    grad_f: *mut Number,
    user_data: UserDataPtr,
) -> c_int {
    // Safety checks for all pointers and sizes
    if user_data.is_null() || x.is_null() || grad_f.is_null() {
        return 0;
    }
    let n_usize = n as usize;
    if n < 0 || n_usize > MAX_PROBLEM_SIZE {
        return 0;
    }

    let problem = unsafe { &*(user_data as *const P) };
    let x_slice = unsafe { std::slice::from_raw_parts(x, n_usize) };
    let grad_slice = unsafe { std::slice::from_raw_parts_mut(grad_f, n_usize) };
    if problem.objective_grad(x_slice, new_x != 0, grad_slice) {
        1
    } else {
        0
    }
}

/// Callback for constraint evaluation.
extern "C" fn eval_g_callback<P: ConstrainedProblem>(
    n: Index,
    x: *const Number,
    new_x: c_int,
    m: Index,
    g: *mut Number,
    user_data: UserDataPtr,
) -> c_int {
    // Safety checks for all pointers and sizes
    if user_data.is_null() || x.is_null() || g.is_null() {
        return 0;
    }
    let n_usize = n as usize;
    let m_usize = m as usize;
    if n < 0 || n_usize > MAX_PROBLEM_SIZE || m < 0 || m_usize > MAX_PROBLEM_SIZE {
        return 0;
    }

    let problem = unsafe { &*(user_data as *const P) };
    let x_slice = unsafe { std::slice::from_raw_parts(x, n_usize) };
    let g_slice = unsafe { std::slice::from_raw_parts_mut(g, m_usize) };
    if problem.constraint(x_slice, new_x != 0, g_slice) {
        1
    } else {
        0
    }
}

/// Callback for Jacobian evaluation.
extern "C" fn eval_jac_g_callback<P: ConstrainedProblem>(
    n: Index,
    x: *const Number,
    new_x: c_int,
    _m: Index,
    nele_jac: Index,
    iRow: *mut Index,
    jCol: *mut Index,
    values: *mut Number,
    user_data: UserDataPtr,
) -> c_int {
    // Safety checks for user_data and size
    if user_data.is_null() {
        return 0;
    }
    let nnz = nele_jac as usize;
    let n_usize = n as usize;
    if nele_jac < 0 || nnz > MAX_PROBLEM_SIZE || n < 0 || n_usize > MAX_PROBLEM_SIZE {
        return 0;
    }

    let problem = unsafe { &*(user_data as *const P) };

    if values.is_null() {
        // Structure query - iRow and jCol must be valid
        if iRow.is_null() || jCol.is_null() {
            return 0;
        }
        let irow_slice = unsafe { std::slice::from_raw_parts_mut(iRow, nnz) };
        let jcol_slice = unsafe { std::slice::from_raw_parts_mut(jCol, nnz) };
        if problem.constraint_jacobian_indices(irow_slice, jcol_slice) {
            1
        } else {
            0
        }
    } else {
        // Value query - x and values must be valid
        if x.is_null() {
            return 0;
        }
        let x_slice = unsafe { std::slice::from_raw_parts(x, n_usize) };
        let vals_slice = unsafe { std::slice::from_raw_parts_mut(values, nnz) };
        if problem.constraint_jacobian_values(x_slice, new_x != 0, vals_slice) {
            1
        } else {
            0
        }
    }
}

/// Callback for Hessian evaluation.
extern "C" fn eval_h_callback<P: ConstrainedProblem>(
    n: Index,
    x: *const Number,
    new_x: c_int,
    obj_factor: Number,
    m: Index,
    lambda: *const Number,
    _new_lambda: c_int,
    nele_hess: Index,
    iRow: *mut Index,
    jCol: *mut Index,
    values: *mut Number,
    user_data: UserDataPtr,
) -> c_int {
    // Safety: Validate user_data pointer before dereferencing
    if user_data.is_null() {
        return 0;
    }

    // Safety: Validate size parameters to prevent integer overflow
    let n_usize = n as usize;
    let m_usize = m as usize;
    let nnz = nele_hess as usize;
    if n < 0 || m < 0 || nele_hess < 0 {
        return 0;
    }
    if n_usize > MAX_PROBLEM_SIZE || m_usize > MAX_PROBLEM_SIZE || nnz > MAX_PROBLEM_SIZE {
        return 0;
    }

    let problem = unsafe { &*(user_data as *const P) };

    if values.is_null() {
        // Structure query - need iRow and jCol
        if iRow.is_null() || jCol.is_null() {
            return 0;
        }
        let irow_slice = unsafe { std::slice::from_raw_parts_mut(iRow, nnz) };
        let jcol_slice = unsafe { std::slice::from_raw_parts_mut(jCol, nnz) };
        if problem.hessian_indices(irow_slice, jcol_slice) {
            1
        } else {
            0
        }
    } else {
        // Value query - need x, lambda, and values
        if x.is_null() || lambda.is_null() {
            return 0;
        }
        let x_slice = unsafe { std::slice::from_raw_parts(x, n_usize) };
        let lambda_slice = unsafe { std::slice::from_raw_parts(lambda, m_usize) };
        let vals_slice = unsafe { std::slice::from_raw_parts_mut(values, nnz) };
        if problem.hessian_values(x_slice, new_x != 0, obj_factor, lambda_slice, vals_slice) {
            1
        } else {
            0
        }
    }
}
