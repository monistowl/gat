//! Native FFI bindings to IPOPT (Interior Point OPTimizer).
//!
//! This crate provides low-level unsafe bindings to the IPOPT C interface,
//! plus a safe Rust wrapper with traits compatible with existing code.
//!
//! IPOPT is an open-source software package for large-scale nonlinear optimization.
//! It implements an interior-point line-search filter method.
//!
//! # Usage
//!
//! ```ignore
//! use gat_ipopt_sys::{BasicProblem, ConstrainedProblem, Ipopt, Number, Index, SolveStatus};
//!
//! struct MyProblem { /* ... */ }
//!
//! impl BasicProblem for MyProblem {
//!     fn num_variables(&self) -> usize { 2 }
//!     fn bounds(&self, x_l: &mut [Number], x_u: &mut [Number]) -> bool { /* ... */ true }
//!     fn initial_point(&self, x: &mut [Number]) -> bool { /* ... */ true }
//!     fn objective(&self, x: &[Number], new_x: bool, obj: &mut Number) -> bool { /* ... */ true }
//!     fn objective_grad(&self, x: &[Number], new_x: bool, grad_f: &mut [Number]) -> bool { /* ... */ true }
//! }
//!
//! impl ConstrainedProblem for MyProblem {
//!     fn num_constraints(&self) -> usize { 1 }
//!     // ... implement other methods
//! }
//!
//! let problem = MyProblem { /* ... */ };
//! let mut solver = Ipopt::new(problem).unwrap();
//! solver.set_option("print_level", 0i32);
//! let result = solver.solve();
//! ```
//!
//! # Building
//!
//! This crate links against IPOPT libraries from:
//! 1. `vendor/local/lib/` (pre-built from vendored sources)
//! 2. System IPOPT via pkg-config (fallback)
//!
//! # Reference
//!
//! Wächter, A., & Biegler, L. T. (2006). On the implementation of an interior-point
//! filter line-search algorithm for large-scale nonlinear programming.
//! *Mathematical Programming*, 106(1), 25-57.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod wrapper;

pub use wrapper::{
    BasicProblem, ConstrainedProblem, Ipopt, IpoptOption, Solution, SolveResult, SolveStatus,
    SolverData,
};

use std::os::raw::{c_char, c_double, c_int, c_void};

// ============================================================================
// TYPES
// ============================================================================

/// Floating-point number type (matches ipnumber in IPOPT).
pub type Number = c_double;

/// Index type for vectors/matrices (matches ipindex in IPOPT).
pub type Index = c_int;

/// Opaque pointer to IPOPT problem structure.
#[repr(C)]
pub struct IpoptProblemInfo {
    _private: [u8; 0],
}

/// Pointer to an IPOPT problem.
pub type IpoptProblem = *mut IpoptProblemInfo;

/// User data pointer passed to callbacks.
pub type UserDataPtr = *mut c_void;

// ============================================================================
// RETURN CODES
// ============================================================================

/// Return codes from IpoptSolve.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationReturnStatus {
    SolveSucceeded = 0,
    SolvedToAcceptableLevel = 1,
    InfeasibleProblemDetected = 2,
    SearchDirectionBecomesTooSmall = 3,
    DivergingIterates = 4,
    UserRequestedStop = 5,
    FeasiblePointFound = 6,

    MaximumIterationsExceeded = -1,
    RestorationFailed = -2,
    ErrorInStepComputation = -3,
    MaximumCpuTimeExceeded = -4,
    MaximumWallTimeExceeded = -5,

    NotEnoughDegreesOfFreedom = -10,
    InvalidProblemDefinition = -11,
    InvalidOption = -12,
    InvalidNumberDetected = -13,

    UnrecoverableException = -100,
    NonIpoptExceptionThrown = -101,
    InsufficientMemory = -102,
    InternalError = -199,
}

impl ApplicationReturnStatus {
    /// Returns true if the solve was successful (optimal or acceptable).
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            ApplicationReturnStatus::SolveSucceeded
                | ApplicationReturnStatus::SolvedToAcceptableLevel
        )
    }
}

// ============================================================================
// CALLBACK FUNCTION TYPES
// ============================================================================

/// Callback for evaluating objective function f(x).
///
/// # Arguments
/// * `n` - Number of variables
/// * `x` - Variable values (length n)
/// * `new_x` - True if x changed since last call
/// * `obj_value` - Output: objective value f(x)
/// * `user_data` - User data pointer
///
/// # Returns
/// True on success, false on evaluation error.
pub type Eval_F_CB = extern "C" fn(
    n: Index,
    x: *const Number,
    new_x: c_int,
    obj_value: *mut Number,
    user_data: UserDataPtr,
) -> c_int;

/// Callback for evaluating gradient of objective function ∇f(x).
///
/// # Arguments
/// * `n` - Number of variables
/// * `x` - Variable values (length n)
/// * `new_x` - True if x changed since last call
/// * `grad_f` - Output: gradient values (length n)
/// * `user_data` - User data pointer
pub type Eval_Grad_F_CB = extern "C" fn(
    n: Index,
    x: *const Number,
    new_x: c_int,
    grad_f: *mut Number,
    user_data: UserDataPtr,
) -> c_int;

/// Callback for evaluating constraint functions g(x).
///
/// # Arguments
/// * `n` - Number of variables
/// * `x` - Variable values (length n)
/// * `new_x` - True if x changed since last call
/// * `m` - Number of constraints
/// * `g` - Output: constraint values (length m)
/// * `user_data` - User data pointer
pub type Eval_G_CB = extern "C" fn(
    n: Index,
    x: *const Number,
    new_x: c_int,
    m: Index,
    g: *mut Number,
    user_data: UserDataPtr,
) -> c_int;

/// Callback for evaluating Jacobian of constraints.
///
/// Called in two modes:
/// 1. `values == NULL`: Fill iRow and jCol with sparsity structure
/// 2. `values != NULL`: Fill values with Jacobian entries
///
/// # Arguments
/// * `n` - Number of variables
/// * `x` - Variable values (length n)
/// * `new_x` - True if x changed since last call
/// * `m` - Number of constraints
/// * `nele_jac` - Number of non-zeros in Jacobian
/// * `iRow` - Row indices (length nele_jac)
/// * `jCol` - Column indices (length nele_jac)
/// * `values` - Non-zero values (length nele_jac), or NULL for structure query
/// * `user_data` - User data pointer
pub type Eval_Jac_G_CB = extern "C" fn(
    n: Index,
    x: *const Number,
    new_x: c_int,
    m: Index,
    nele_jac: Index,
    iRow: *mut Index,
    jCol: *mut Index,
    values: *mut Number,
    user_data: UserDataPtr,
) -> c_int;

/// Callback for evaluating Hessian of Lagrangian.
///
/// Computes: σ ∇²f(x) + Σᵢ λᵢ ∇²gᵢ(x)
///
/// Called in two modes:
/// 1. `values == NULL`: Fill iRow and jCol with sparsity structure (lower triangle)
/// 2. `values != NULL`: Fill values with Hessian entries
///
/// # Arguments
/// * `n` - Number of variables
/// * `x` - Variable values (length n)
/// * `new_x` - True if x changed since last call
/// * `obj_factor` - σ, scaling for objective Hessian
/// * `m` - Number of constraints
/// * `lambda` - Constraint multipliers (length m)
/// * `new_lambda` - True if lambda changed since last call
/// * `nele_hess` - Number of non-zeros in lower triangle of Hessian
/// * `iRow` - Row indices (length nele_hess)
/// * `jCol` - Column indices (length nele_hess)
/// * `values` - Non-zero values (length nele_hess), or NULL for structure query
/// * `user_data` - User data pointer
pub type Eval_H_CB = extern "C" fn(
    n: Index,
    x: *const Number,
    new_x: c_int,
    obj_factor: Number,
    m: Index,
    lambda: *const Number,
    new_lambda: c_int,
    nele_hess: Index,
    iRow: *mut Index,
    jCol: *mut Index,
    values: *mut Number,
    user_data: UserDataPtr,
) -> c_int;

/// Callback for intermediate iteration info.
///
/// Called once per iteration. Return false to terminate optimization.
pub type Intermediate_CB = extern "C" fn(
    alg_mod: Index,
    iter_count: Index,
    obj_value: Number,
    inf_pr: Number,
    inf_du: Number,
    mu: Number,
    d_norm: Number,
    regularization_size: Number,
    alpha_du: Number,
    alpha_pr: Number,
    ls_trials: Index,
    user_data: UserDataPtr,
) -> c_int;

// ============================================================================
// IPOPT C INTERFACE FUNCTIONS
// ============================================================================

extern "C" {
    /// Create a new IPOPT problem.
    ///
    /// # Arguments
    /// * `n` - Number of variables
    /// * `x_L` - Lower bounds on variables (length n)
    /// * `x_U` - Upper bounds on variables (length n)
    /// * `m` - Number of constraints
    /// * `g_L` - Lower bounds on constraints (length m)
    /// * `g_U` - Upper bounds on constraints (length m)
    /// * `nele_jac` - Number of non-zeros in constraint Jacobian
    /// * `nele_hess` - Number of non-zeros in Hessian of Lagrangian
    /// * `index_style` - 0 for C-style (0-based), 1 for Fortran-style (1-based)
    /// * `eval_f` - Callback for objective evaluation
    /// * `eval_g` - Callback for constraint evaluation
    /// * `eval_grad_f` - Callback for objective gradient
    /// * `eval_jac_g` - Callback for constraint Jacobian
    /// * `eval_h` - Callback for Hessian of Lagrangian
    ///
    /// # Returns
    /// Pointer to problem, or NULL on error.
    pub fn CreateIpoptProblem(
        n: Index,
        x_L: *const Number,
        x_U: *const Number,
        m: Index,
        g_L: *const Number,
        g_U: *const Number,
        nele_jac: Index,
        nele_hess: Index,
        index_style: Index,
        eval_f: Eval_F_CB,
        eval_g: Eval_G_CB,
        eval_grad_f: Eval_Grad_F_CB,
        eval_jac_g: Eval_Jac_G_CB,
        eval_h: Eval_H_CB,
    ) -> IpoptProblem;

    /// Free an IPOPT problem.
    pub fn FreeIpoptProblem(ipopt_problem: IpoptProblem);

    /// Add a string option.
    ///
    /// # Returns
    /// True if option was set successfully.
    pub fn AddIpoptStrOption(
        ipopt_problem: IpoptProblem,
        keyword: *const c_char,
        val: *const c_char,
    ) -> c_int;

    /// Add a numeric option.
    ///
    /// # Returns
    /// True if option was set successfully.
    pub fn AddIpoptNumOption(
        ipopt_problem: IpoptProblem,
        keyword: *const c_char,
        val: Number,
    ) -> c_int;

    /// Add an integer option.
    ///
    /// # Returns
    /// True if option was set successfully.
    pub fn AddIpoptIntOption(
        ipopt_problem: IpoptProblem,
        keyword: *const c_char,
        val: Index,
    ) -> c_int;

    /// Open output file.
    ///
    /// # Returns
    /// True if file was opened successfully.
    pub fn OpenIpoptOutputFile(
        ipopt_problem: IpoptProblem,
        file_name: *const c_char,
        print_level: c_int,
    ) -> c_int;

    /// Set problem scaling.
    pub fn SetIpoptProblemScaling(
        ipopt_problem: IpoptProblem,
        obj_scaling: Number,
        x_scaling: *const Number,
        g_scaling: *const Number,
    ) -> c_int;

    /// Set intermediate callback.
    pub fn SetIntermediateCallback(
        ipopt_problem: IpoptProblem,
        intermediate_cb: Option<Intermediate_CB>,
    ) -> c_int;

    /// Solve the optimization problem.
    ///
    /// # Arguments
    /// * `ipopt_problem` - Problem to solve
    /// * `x` - Input: starting point; Output: optimal solution (length n)
    /// * `g` - Output: constraint values at solution (length m), or NULL
    /// * `obj_val` - Output: objective value at solution, or NULL
    /// * `mult_g` - Input/Output: constraint multipliers (length m), or NULL
    /// * `mult_x_L` - Input/Output: lower bound multipliers (length n), or NULL
    /// * `mult_x_U` - Input/Output: upper bound multipliers (length n), or NULL
    /// * `user_data` - User data passed to callbacks
    ///
    /// # Returns
    /// Status code indicating success or failure reason.
    pub fn IpoptSolve(
        ipopt_problem: IpoptProblem,
        x: *mut Number,
        g: *mut Number,
        obj_val: *mut Number,
        mult_g: *mut Number,
        mult_x_L: *mut Number,
        mult_x_U: *mut Number,
        user_data: UserDataPtr,
    ) -> ApplicationReturnStatus;

    /// Get IPOPT version.
    pub fn GetIpoptVersion(major: *mut c_int, minor: *mut c_int, release: *mut c_int);
}

// ============================================================================
// SAFE RUST WRAPPER
// ============================================================================

/// Safe wrapper around IPOPT problem.
pub struct Problem {
    inner: IpoptProblem,
}

impl Problem {
    /// Get IPOPT library version.
    pub fn version() -> (i32, i32, i32) {
        let mut major = 0;
        let mut minor = 0;
        let mut release = 0;
        unsafe {
            GetIpoptVersion(&mut major, &mut minor, &mut release);
        }
        (major, minor, release)
    }

    /// Set a string option.
    pub fn set_option_str(&mut self, name: &str, value: &str) -> bool {
        use std::ffi::CString;
        let name_c = CString::new(name).unwrap();
        let value_c = CString::new(value).unwrap();
        unsafe { AddIpoptStrOption(self.inner, name_c.as_ptr(), value_c.as_ptr()) != 0 }
    }

    /// Set a numeric option.
    pub fn set_option_num(&mut self, name: &str, value: f64) -> bool {
        use std::ffi::CString;
        let name_c = CString::new(name).unwrap();
        unsafe { AddIpoptNumOption(self.inner, name_c.as_ptr(), value) != 0 }
    }

    /// Set an integer option.
    pub fn set_option_int(&mut self, name: &str, value: i32) -> bool {
        use std::ffi::CString;
        let name_c = CString::new(name).unwrap();
        unsafe { AddIpoptIntOption(self.inner, name_c.as_ptr(), value) != 0 }
    }
}

impl Drop for Problem {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                FreeIpoptProblem(self.inner);
            }
        }
    }
}

// Problem is not thread-safe (IPOPT internal state)
// but can be sent between threads
unsafe impl Send for Problem {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_status() {
        assert!(ApplicationReturnStatus::SolveSucceeded.is_success());
        assert!(ApplicationReturnStatus::SolvedToAcceptableLevel.is_success());
        assert!(!ApplicationReturnStatus::MaximumIterationsExceeded.is_success());
    }

    #[test]
    fn test_version() {
        let (major, minor, release) = Problem::version();
        assert!(major >= 3);
        println!("IPOPT version: {}.{}.{}", major, minor, release);
    }
}
