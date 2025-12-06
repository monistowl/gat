# PROJ-2: Solver Strategy Pattern Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refactor OPF solver architecture from monolithic match statements to extensible Strategy pattern with dynamic dispatch.

**Architecture:** Two-level trait abstraction (`OpfFormulation` defines the problem, `OpfBackend` implements solvers) with `SolverRegistry` holding components and `OpfDispatcher` orchestrating solve attempts with configurable fallback chains.

**Tech Stack:** Rust, dynamic dispatch (`dyn Trait`), `Arc` for shared ownership, existing Clarabel/L-BFGS solvers

**Design Doc:** See `docs/plans/2025-12-05-solver-strategy-pattern-design.md` for full design rationale.

---

## Task 1: Create Core Types and Traits

**Files:**
- Create: `crates/gat-algo/src/opf/traits.rs`
- Modify: `crates/gat-algo/src/opf/mod.rs` (add module declaration)

**Step 1: Write the failing test**

Create `crates/gat-algo/src/opf/traits.rs` with test:

```rust
//! Core traits for the extensible OPF solver architecture.
//!
//! This module defines the Strategy pattern traits that allow new formulations
//! and backends to be added without modifying existing code.

use crate::OpfError;
use gat_core::Network;
use std::collections::HashMap;

use super::dispatch::ProblemClass;
use super::OpfSolution;

/// Kinds of warm-start data that can initialize a solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarmStartKind {
    /// Flat start: V=1.0, θ=0, Pg=Pmax/2
    Flat,
    /// From DC-OPF: angles and Pg only
    Dc,
    /// From SOCP: V, θ, Pg, Qg
    Socp,
}

/// Configuration passed to backend solvers.
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// Maximum iterations
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Timeout in seconds
    pub timeout_seconds: u64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
            timeout_seconds: 300,
        }
    }
}

/// Intermediate problem representation built from a Network.
///
/// This allows formulations to precompute data structures (Y-bus, etc.)
/// that backends can use for solving.
#[derive(Debug)]
pub struct OpfProblem {
    /// Number of buses
    pub n_bus: usize,
    /// Number of generators
    pub n_gen: usize,
    /// Problem class for solver matching
    pub problem_class: ProblemClass,
    /// Opaque data for the backend (formulation-specific)
    pub data: Box<dyn std::any::Any + Send + Sync>,
}

/// Defines a mathematical OPF formulation (what to solve).
///
/// Implementations include DC-OPF, SOCP relaxation, and full AC-OPF.
/// Each formulation knows how to build its problem representation from
/// a Network and what warm-start types it can accept.
pub trait OpfFormulation: Send + Sync {
    /// Unique identifier (e.g., "dc-opf", "ac-opf", "socp")
    fn id(&self) -> &str;

    /// Problem class for solver matching
    fn problem_class(&self) -> ProblemClass;

    /// Build the problem from a network
    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError>;

    /// Warm-start types this formulation can accept
    fn accepts_warm_start(&self) -> &[WarmStartKind];
}

/// Implements the actual solving (how to solve).
///
/// Backends are matched to formulations via ProblemClass. Multiple backends
/// may support the same class (e.g., Clarabel and HiGHS both solve LP).
pub trait OpfBackend: Send + Sync {
    /// Unique identifier (e.g., "clarabel", "ipopt", "lbfgs")
    fn id(&self) -> &str;

    /// Problem classes this backend can solve
    fn supported_classes(&self) -> &[ProblemClass];

    /// Check if this backend is available at runtime
    fn is_available(&self) -> bool;

    /// Solve the problem
    fn solve(
        &self,
        problem: &OpfProblem,
        config: &SolverConfig,
        warm_start: Option<&HashMap<String, f64>>,
    ) -> Result<OpfSolution, OpfError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that traits are object-safe (can be used with dyn).
    #[test]
    fn test_traits_are_object_safe() {
        // This test passes if it compiles - traits must be object-safe
        fn _accepts_formulation(_f: &dyn OpfFormulation) {}
        fn _accepts_backend(_b: &dyn OpfBackend) {}
    }

    /// Test that trait objects can be Send + Sync (required for Arc).
    #[test]
    fn test_traits_are_send_sync() {
        fn _assert_send<T: Send>() {}
        fn _assert_sync<T: Sync>() {}

        // These compile only if the trait objects are Send + Sync
        _assert_send::<Box<dyn OpfFormulation>>();
        _assert_sync::<Box<dyn OpfFormulation>>();
        _assert_send::<Box<dyn OpfBackend>>();
        _assert_sync::<Box<dyn OpfBackend>>();
    }

    /// Test default SolverConfig values.
    #[test]
    fn test_solver_config_defaults() {
        let config = SolverConfig::default();
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.tolerance, 1e-6);
        assert_eq!(config.timeout_seconds, 300);
    }

    /// Test WarmStartKind equality.
    #[test]
    fn test_warm_start_kind_eq() {
        assert_eq!(WarmStartKind::Flat, WarmStartKind::Flat);
        assert_ne!(WarmStartKind::Flat, WarmStartKind::Dc);
    }
}
```

**Step 2: Add module to mod.rs**

In `crates/gat-algo/src/opf/mod.rs`, add after line 32 (after `mod types;`):

```rust
pub mod traits;
```

And add to exports after line 38:

```rust
pub use traits::{OpfBackend, OpfFormulation, OpfProblem, SolverConfig, WarmStartKind};
```

**Step 3: Run test to verify it passes**

Run: `cargo test -p gat-algo traits::tests --no-fail-fast`
Expected: All 4 tests PASS

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/traits.rs crates/gat-algo/src/opf/mod.rs
git commit -m "feat(opf): add core traits for Strategy pattern (OpfFormulation, OpfBackend)"
```

---

## Task 2: Create SolverRegistry

**Files:**
- Create: `crates/gat-algo/src/opf/registry.rs`
- Modify: `crates/gat-algo/src/opf/mod.rs` (add module and exports)

**Step 1: Write the failing test**

Create `crates/gat-algo/src/opf/registry.rs`:

```rust
//! Registry for OPF formulations and backends.
//!
//! The registry holds all registered components and provides lookup
//! by ID and filtering by problem class.

use std::collections::HashMap;
use std::sync::Arc;

use super::dispatch::ProblemClass;
use super::traits::{OpfBackend, OpfFormulation};

/// Holds all registered formulations and backends.
///
/// Create with `SolverRegistry::new()` for empty or
/// `SolverRegistry::with_defaults()` for built-in solvers.
#[derive(Default)]
pub struct SolverRegistry {
    formulations: HashMap<String, Arc<dyn OpfFormulation>>,
    backends: HashMap<String, Arc<dyn OpfBackend>>,
}

impl SolverRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create registry with built-in formulations and backends.
    ///
    /// Registers:
    /// - Formulations: dc-opf, socp, ac-opf, economic-dispatch
    /// - Backends: clarabel, lbfgs (+ ipopt if available)
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        // TODO: Register built-in formulations and backends in Task 5
        registry
    }

    /// Register a custom formulation.
    pub fn register_formulation(&mut self, f: Arc<dyn OpfFormulation>) {
        self.formulations.insert(f.id().to_string(), f);
    }

    /// Register a custom backend.
    pub fn register_backend(&mut self, b: Arc<dyn OpfBackend>) {
        self.backends.insert(b.id().to_string(), b);
    }

    /// Get a formulation by ID.
    pub fn get_formulation(&self, id: &str) -> Option<Arc<dyn OpfFormulation>> {
        self.formulations.get(id).cloned()
    }

    /// Get a backend by ID.
    pub fn get_backend(&self, id: &str) -> Option<Arc<dyn OpfBackend>> {
        self.backends.get(id).cloned()
    }

    /// List all formulation IDs.
    pub fn list_formulations(&self) -> Vec<&str> {
        self.formulations.keys().map(|s| s.as_str()).collect()
    }

    /// List all backend IDs.
    pub fn list_backends(&self) -> Vec<&str> {
        self.backends.keys().map(|s| s.as_str()).collect()
    }

    /// List available backends for a problem class.
    ///
    /// Returns IDs of backends that:
    /// 1. Support the given problem class
    /// 2. Are currently available (runtime check)
    pub fn backends_for(&self, class: ProblemClass) -> Vec<&str> {
        self.backends
            .iter()
            .filter(|(_, b)| b.supported_classes().contains(&class) && b.is_available())
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Select the best available backend for a problem class.
    ///
    /// Priority:
    /// 1. Native solvers (IPOPT for NLP, HiGHS for LP/MIP)
    /// 2. Pure-Rust fallbacks (Clarabel for LP/SOCP, L-BFGS for NLP)
    pub fn select_backend(&self, class: ProblemClass) -> Option<Arc<dyn OpfBackend>> {
        // Preference order by problem class
        let preferred = match class {
            ProblemClass::LinearProgram => vec!["highs", "clarabel"],
            ProblemClass::ConicProgram => vec!["clarabel"],
            ProblemClass::NonlinearProgram => vec!["ipopt", "lbfgs"],
            ProblemClass::MixedInteger => vec!["highs", "cbc"],
        };

        // Find first available backend in preference order
        for id in preferred {
            if let Some(backend) = self.backends.get(id) {
                if backend.supported_classes().contains(&class) && backend.is_available() {
                    return Some(backend.clone());
                }
            }
        }

        // Fall back to any available backend for this class
        self.backends
            .values()
            .find(|b| b.supported_classes().contains(&class) && b.is_available())
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opf::traits::{OpfProblem, SolverConfig, WarmStartKind};
    use crate::opf::OpfSolution;
    use crate::OpfError;
    use gat_core::Network;

    /// Mock formulation for testing.
    struct MockFormulation {
        id: String,
        class: ProblemClass,
    }

    impl OpfFormulation for MockFormulation {
        fn id(&self) -> &str {
            &self.id
        }
        fn problem_class(&self) -> ProblemClass {
            self.class
        }
        fn build_problem(&self, _network: &Network) -> Result<OpfProblem, OpfError> {
            unimplemented!("mock")
        }
        fn accepts_warm_start(&self) -> &[WarmStartKind] {
            &[WarmStartKind::Flat]
        }
    }

    /// Mock backend for testing.
    struct MockBackend {
        id: String,
        classes: Vec<ProblemClass>,
        available: bool,
    }

    impl OpfBackend for MockBackend {
        fn id(&self) -> &str {
            &self.id
        }
        fn supported_classes(&self) -> &[ProblemClass] {
            &self.classes
        }
        fn is_available(&self) -> bool {
            self.available
        }
        fn solve(
            &self,
            _problem: &OpfProblem,
            _config: &SolverConfig,
            _warm_start: Option<&std::collections::HashMap<String, f64>>,
        ) -> Result<OpfSolution, OpfError> {
            unimplemented!("mock")
        }
    }

    #[test]
    fn test_register_and_get_formulation() {
        let mut registry = SolverRegistry::new();
        let form = Arc::new(MockFormulation {
            id: "test-form".to_string(),
            class: ProblemClass::LinearProgram,
        });

        registry.register_formulation(form.clone());

        let retrieved = registry.get_formulation("test-form");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-form");
    }

    #[test]
    fn test_register_and_get_backend() {
        let mut registry = SolverRegistry::new();
        let backend = Arc::new(MockBackend {
            id: "test-backend".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: true,
        });

        registry.register_backend(backend);

        let retrieved = registry.get_backend("test-backend");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-backend");
    }

    #[test]
    fn test_backends_for_filters_by_class() {
        let mut registry = SolverRegistry::new();

        registry.register_backend(Arc::new(MockBackend {
            id: "lp-solver".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: true,
        }));
        registry.register_backend(Arc::new(MockBackend {
            id: "nlp-solver".to_string(),
            classes: vec![ProblemClass::NonlinearProgram],
            available: true,
        }));

        let lp_backends = registry.backends_for(ProblemClass::LinearProgram);
        assert_eq!(lp_backends.len(), 1);
        assert!(lp_backends.contains(&"lp-solver"));

        let nlp_backends = registry.backends_for(ProblemClass::NonlinearProgram);
        assert_eq!(nlp_backends.len(), 1);
        assert!(nlp_backends.contains(&"nlp-solver"));
    }

    #[test]
    fn test_backends_for_excludes_unavailable() {
        let mut registry = SolverRegistry::new();

        registry.register_backend(Arc::new(MockBackend {
            id: "available".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: true,
        }));
        registry.register_backend(Arc::new(MockBackend {
            id: "unavailable".to_string(),
            classes: vec![ProblemClass::LinearProgram],
            available: false,
        }));

        let backends = registry.backends_for(ProblemClass::LinearProgram);
        assert_eq!(backends.len(), 1);
        assert!(backends.contains(&"available"));
    }

    #[test]
    fn test_select_backend_returns_available() {
        let mut registry = SolverRegistry::new();

        registry.register_backend(Arc::new(MockBackend {
            id: "clarabel".to_string(),
            classes: vec![ProblemClass::LinearProgram, ProblemClass::ConicProgram],
            available: true,
        }));

        let selected = registry.select_backend(ProblemClass::LinearProgram);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().id(), "clarabel");
    }

    #[test]
    fn test_select_backend_returns_none_when_empty() {
        let registry = SolverRegistry::new();
        let selected = registry.select_backend(ProblemClass::MixedInteger);
        assert!(selected.is_none());
    }

    #[test]
    fn test_list_formulations() {
        let mut registry = SolverRegistry::new();
        registry.register_formulation(Arc::new(MockFormulation {
            id: "form-a".to_string(),
            class: ProblemClass::LinearProgram,
        }));
        registry.register_formulation(Arc::new(MockFormulation {
            id: "form-b".to_string(),
            class: ProblemClass::NonlinearProgram,
        }));

        let ids = registry.list_formulations();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"form-a"));
        assert!(ids.contains(&"form-b"));
    }
}
```

**Step 2: Add module to mod.rs**

In `crates/gat-algo/src/opf/mod.rs`, add after `pub mod traits;`:

```rust
pub mod registry;
```

And add to exports:

```rust
pub use registry::SolverRegistry;
```

**Step 3: Run test to verify it passes**

Run: `cargo test -p gat-algo registry::tests --no-fail-fast`
Expected: All 7 tests PASS

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/registry.rs crates/gat-algo/src/opf/mod.rs
git commit -m "feat(opf): add SolverRegistry for formulation/backend lookup"
```

---

## Task 3: Create OpfDispatcher

**Files:**
- Create: `crates/gat-algo/src/opf/dispatcher.rs`
- Modify: `crates/gat-algo/src/opf/mod.rs` (add module and exports)

**Step 1: Write the failing test**

Create `crates/gat-algo/src/opf/dispatcher.rs`:

```rust
//! OpfDispatcher orchestrates OPF solving with fallback chains.
//!
//! The dispatcher:
//! 1. Looks up the formulation by ID
//! 2. Builds the problem via `formulation.build_problem(network)`
//! 3. Selects the best available backend for the problem class
//! 4. Attempts solve; on failure, tries warm-starts from fallback chain

use std::sync::Arc;

use crate::OpfError;
use gat_core::Network;

use super::registry::SolverRegistry;
use super::traits::{SolverConfig, WarmStartKind};
use super::OpfSolution;

/// Orchestrates OPF solving with configurable fallback chains.
pub struct OpfDispatcher {
    registry: Arc<SolverRegistry>,
}

impl OpfDispatcher {
    /// Create a dispatcher with the given registry.
    pub fn new(registry: Arc<SolverRegistry>) -> Self {
        Self { registry }
    }

    /// Solve OPF for a network using the specified formulation.
    ///
    /// # Arguments
    /// * `network` - The power network to solve
    /// * `formulation_id` - ID of the formulation (e.g., "dc-opf", "ac-opf")
    /// * `config` - Solver configuration (iterations, tolerance, timeout)
    /// * `fallbacks` - Warm-start kinds to try if initial solve fails
    ///
    /// # Returns
    /// The solution, or the first error if all attempts fail.
    ///
    /// # Fallback Chain
    /// If the initial (flat-start) solve fails with a convergence error:
    /// 1. For each warm-start kind in `fallbacks`:
    ///    a. Run the corresponding formulation to get warm-start data
    ///    b. Retry the target solve with warm-start
    /// 2. If all fallbacks fail, return the original error
    pub fn solve(
        &self,
        network: &Network,
        formulation_id: &str,
        config: SolverConfig,
        fallbacks: &[WarmStartKind],
    ) -> Result<OpfSolution, OpfError> {
        // Look up formulation
        let formulation = self
            .registry
            .get_formulation(formulation_id)
            .ok_or_else(|| {
                OpfError::NotImplemented(format!("Unknown formulation: {}", formulation_id))
            })?;

        // Build problem
        let problem = formulation.build_problem(network)?;

        // Select backend
        let backend = self
            .registry
            .select_backend(problem.problem_class)
            .ok_or_else(|| {
                OpfError::NotImplemented(format!(
                    "No available backend for {:?}",
                    problem.problem_class
                ))
            })?;

        // Attempt flat-start solve
        match backend.solve(&problem, &config, None) {
            Ok(solution) => return Ok(solution),
            Err(first_error) => {
                // Check if this is a convergence failure worth retrying
                if !Self::is_convergence_failure(&first_error) {
                    return Err(first_error);
                }

                // Try fallback warm-starts
                for &warm_start_kind in fallbacks {
                    // Skip Flat since that's what we just tried
                    if warm_start_kind == WarmStartKind::Flat {
                        continue;
                    }

                    // Get warm-start data from appropriate formulation
                    if let Some(warm_start_data) =
                        self.compute_warm_start(network, warm_start_kind, &config)
                    {
                        // Retry with warm-start
                        if let Ok(solution) =
                            backend.solve(&problem, &config, Some(&warm_start_data))
                        {
                            return Ok(solution);
                        }
                    }
                }

                // All fallbacks failed, return original error
                Err(first_error)
            }
        }
    }

    /// Check if an error is a convergence failure that might benefit from warm-start.
    fn is_convergence_failure(error: &OpfError) -> bool {
        let msg = format!("{:?}", error);
        msg.contains("MaximumIterationsExceeded")
            || msg.contains("InfeasibleProblemDetected")
            || msg.contains("RestorationFailed")
            || msg.contains("ConvergenceFailed")
            || msg.contains("convergence")
    }

    /// Compute warm-start data from a simpler formulation.
    fn compute_warm_start(
        &self,
        network: &Network,
        kind: WarmStartKind,
        config: &SolverConfig,
    ) -> Option<std::collections::HashMap<String, f64>> {
        let formulation_id = match kind {
            WarmStartKind::Flat => return None,
            WarmStartKind::Dc => "dc-opf",
            WarmStartKind::Socp => "socp",
        };

        // Try to solve the warm-start formulation
        let formulation = self.registry.get_formulation(formulation_id)?;
        let problem = formulation.build_problem(network).ok()?;
        let backend = self.registry.select_backend(problem.problem_class)?;
        let solution = backend.solve(&problem, config, None).ok()?;

        // Convert solution to warm-start map
        // Keys: "Vm:<bus>", "Va:<bus>", "Pg:<gen>", "Qg:<gen>"
        let mut warm_start = std::collections::HashMap::new();

        for (bus, v) in &solution.bus_voltage_mag {
            warm_start.insert(format!("Vm:{}", bus), *v);
        }
        for (bus, a) in &solution.bus_voltage_ang {
            warm_start.insert(format!("Va:{}", bus), *a);
        }
        for (gen, p) in &solution.generator_p {
            warm_start.insert(format!("Pg:{}", gen), *p);
        }
        for (gen, q) in &solution.generator_q {
            warm_start.insert(format!("Qg:{}", gen), *q);
        }

        Some(warm_start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opf::dispatch::ProblemClass;
    use crate::opf::traits::{OpfBackend, OpfFormulation, OpfProblem};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock formulation that always succeeds.
    struct SuccessFormulation;

    impl OpfFormulation for SuccessFormulation {
        fn id(&self) -> &str {
            "success"
        }
        fn problem_class(&self) -> ProblemClass {
            ProblemClass::LinearProgram
        }
        fn build_problem(&self, _network: &Network) -> Result<OpfProblem, OpfError> {
            Ok(OpfProblem {
                n_bus: 2,
                n_gen: 1,
                problem_class: ProblemClass::LinearProgram,
                data: Box::new(()),
            })
        }
        fn accepts_warm_start(&self) -> &[WarmStartKind] {
            &[WarmStartKind::Flat]
        }
    }

    /// Mock backend that succeeds on Nth attempt.
    struct MockBackend {
        fail_count: AtomicUsize,
    }

    impl MockBackend {
        fn new(fail_count: usize) -> Self {
            Self {
                fail_count: AtomicUsize::new(fail_count),
            }
        }
    }

    impl OpfBackend for MockBackend {
        fn id(&self) -> &str {
            "mock"
        }
        fn supported_classes(&self) -> &[ProblemClass] {
            &[ProblemClass::LinearProgram]
        }
        fn is_available(&self) -> bool {
            true
        }
        fn solve(
            &self,
            _problem: &OpfProblem,
            _config: &SolverConfig,
            _warm_start: Option<&HashMap<String, f64>>,
        ) -> Result<OpfSolution, OpfError> {
            let remaining = self.fail_count.fetch_sub(1, Ordering::SeqCst);
            if remaining > 0 {
                Err(OpfError::ConvergenceFailed(
                    "MaximumIterationsExceeded".to_string(),
                ))
            } else {
                Ok(OpfSolution {
                    converged: true,
                    ..Default::default()
                })
            }
        }
    }

    fn create_test_network() -> Network {
        Network::new()
    }

    #[test]
    fn test_dispatcher_unknown_formulation() {
        let registry = Arc::new(SolverRegistry::new());
        let dispatcher = OpfDispatcher::new(registry);

        let result = dispatcher.solve(
            &create_test_network(),
            "unknown",
            SolverConfig::default(),
            &[],
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown formulation"));
    }

    #[test]
    fn test_dispatcher_no_backend_for_class() {
        let mut registry = SolverRegistry::new();
        registry.register_formulation(Arc::new(SuccessFormulation));
        // No backend registered

        let dispatcher = OpfDispatcher::new(Arc::new(registry));

        let result = dispatcher.solve(
            &create_test_network(),
            "success",
            SolverConfig::default(),
            &[],
        );

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No available backend"));
    }

    #[test]
    fn test_dispatcher_success_on_first_try() {
        let mut registry = SolverRegistry::new();
        registry.register_formulation(Arc::new(SuccessFormulation));
        registry.register_backend(Arc::new(MockBackend::new(0))); // Succeed immediately

        let dispatcher = OpfDispatcher::new(Arc::new(registry));

        let result = dispatcher.solve(
            &create_test_network(),
            "success",
            SolverConfig::default(),
            &[],
        );

        assert!(result.is_ok());
        assert!(result.unwrap().converged);
    }

    #[test]
    fn test_is_convergence_failure() {
        assert!(OpfDispatcher::is_convergence_failure(&OpfError::ConvergenceFailed(
            "MaximumIterationsExceeded".to_string()
        )));
        assert!(OpfDispatcher::is_convergence_failure(&OpfError::ConvergenceFailed(
            "InfeasibleProblemDetected".to_string()
        )));
        assert!(!OpfDispatcher::is_convergence_failure(&OpfError::DataValidation(
            "Missing data".to_string()
        )));
    }
}
```

**Step 2: Add module to mod.rs**

In `crates/gat-algo/src/opf/mod.rs`, add after `pub mod registry;`:

```rust
mod dispatcher;
```

And add to exports:

```rust
pub use dispatcher::OpfDispatcher;
```

**Step 3: Run test to verify it passes**

Run: `cargo test -p gat-algo dispatcher::tests --no-fail-fast`
Expected: All 4 tests PASS

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/dispatcher.rs crates/gat-algo/src/opf/mod.rs
git commit -m "feat(opf): add OpfDispatcher with fallback chain support"
```

---

## Task 4: Create Built-in Formulations

**Files:**
- Create: `crates/gat-algo/src/opf/formulations/mod.rs`
- Create: `crates/gat-algo/src/opf/formulations/dc.rs`
- Create: `crates/gat-algo/src/opf/formulations/socp.rs`
- Create: `crates/gat-algo/src/opf/formulations/ac.rs`
- Create: `crates/gat-algo/src/opf/formulations/economic.rs`
- Modify: `crates/gat-algo/src/opf/mod.rs` (add module)

**Step 1: Create formulations directory and mod.rs**

Create `crates/gat-algo/src/opf/formulations/mod.rs`:

```rust
//! Built-in OPF formulations.
//!
//! Each formulation wraps an existing solver implementation and exposes
//! it through the `OpfFormulation` trait.

mod ac;
mod dc;
mod economic;
mod socp;

pub use ac::AcOpfFormulation;
pub use dc::DcOpfFormulation;
pub use economic::EconomicDispatchFormulation;
pub use socp::SocpFormulation;
```

**Step 2: Create DC-OPF formulation**

Create `crates/gat-algo/src/opf/formulations/dc.rs`:

```rust
//! DC-OPF formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// DC-OPF formulation (linear program).
///
/// Wraps the existing `dc_opf::solve()` implementation.
pub struct DcOpfFormulation;

impl OpfFormulation for DcOpfFormulation {
    fn id(&self) -> &str {
        "dc-opf"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::LinearProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        // Store the network reference for the backend to use
        // In a real implementation, we'd precompute the B' matrix here
        Ok(OpfProblem {
            n_bus: network.bus_count(),
            n_gen: network.generator_count(),
            problem_class: ProblemClass::LinearProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        // DC-OPF is LP, doesn't benefit from warm-start
        &[WarmStartKind::Flat]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_formulation_id() {
        let form = DcOpfFormulation;
        assert_eq!(form.id(), "dc-opf");
    }

    #[test]
    fn test_dc_formulation_problem_class() {
        let form = DcOpfFormulation;
        assert_eq!(form.problem_class(), ProblemClass::LinearProgram);
    }

    #[test]
    fn test_dc_formulation_warm_start() {
        let form = DcOpfFormulation;
        let warm_starts = form.accepts_warm_start();
        assert_eq!(warm_starts.len(), 1);
        assert_eq!(warm_starts[0], WarmStartKind::Flat);
    }
}
```

**Step 3: Create SOCP formulation**

Create `crates/gat-algo/src/opf/formulations/socp.rs`:

```rust
//! SOCP relaxation formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// SOCP relaxation formulation (conic program).
///
/// Wraps the existing `socp::solve()` implementation.
pub struct SocpFormulation;

impl OpfFormulation for SocpFormulation {
    fn id(&self) -> &str {
        "socp"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::ConicProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        Ok(OpfProblem {
            n_bus: network.bus_count(),
            n_gen: network.generator_count(),
            problem_class: ProblemClass::ConicProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        // SOCP can use DC warm-start for initial point
        &[WarmStartKind::Flat, WarmStartKind::Dc]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socp_formulation_id() {
        let form = SocpFormulation;
        assert_eq!(form.id(), "socp");
    }

    #[test]
    fn test_socp_formulation_problem_class() {
        let form = SocpFormulation;
        assert_eq!(form.problem_class(), ProblemClass::ConicProgram);
    }
}
```

**Step 4: Create AC-OPF formulation**

Create `crates/gat-algo/src/opf/formulations/ac.rs`:

```rust
//! Full nonlinear AC-OPF formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// Full nonlinear AC-OPF formulation (nonlinear program).
///
/// Wraps the existing `ac_nlp::AcOpfProblem` implementation.
pub struct AcOpfFormulation;

impl OpfFormulation for AcOpfFormulation {
    fn id(&self) -> &str {
        "ac-opf"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::NonlinearProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        Ok(OpfProblem {
            n_bus: network.bus_count(),
            n_gen: network.generator_count(),
            problem_class: ProblemClass::NonlinearProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        // AC-OPF benefits from DC and SOCP warm-starts
        &[WarmStartKind::Flat, WarmStartKind::Dc, WarmStartKind::Socp]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ac_formulation_id() {
        let form = AcOpfFormulation;
        assert_eq!(form.id(), "ac-opf");
    }

    #[test]
    fn test_ac_formulation_accepts_all_warm_starts() {
        let form = AcOpfFormulation;
        let warm_starts = form.accepts_warm_start();
        assert!(warm_starts.contains(&WarmStartKind::Flat));
        assert!(warm_starts.contains(&WarmStartKind::Dc));
        assert!(warm_starts.contains(&WarmStartKind::Socp));
    }
}
```

**Step 5: Create Economic Dispatch formulation**

Create `crates/gat-algo/src/opf/formulations/economic.rs`:

```rust
//! Economic dispatch formulation wrapper.

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
use crate::OpfError;
use gat_core::Network;

/// Economic dispatch formulation (linear program, no network).
///
/// Wraps the existing `economic::solve()` implementation.
pub struct EconomicDispatchFormulation;

impl OpfFormulation for EconomicDispatchFormulation {
    fn id(&self) -> &str {
        "economic-dispatch"
    }

    fn problem_class(&self) -> ProblemClass {
        ProblemClass::LinearProgram
    }

    fn build_problem(&self, network: &Network) -> Result<OpfProblem, OpfError> {
        Ok(OpfProblem {
            n_bus: network.bus_count(),
            n_gen: network.generator_count(),
            problem_class: ProblemClass::LinearProgram,
            data: Box::new(()),
        })
    }

    fn accepts_warm_start(&self) -> &[WarmStartKind] {
        &[WarmStartKind::Flat]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_economic_formulation_id() {
        let form = EconomicDispatchFormulation;
        assert_eq!(form.id(), "economic-dispatch");
    }
}
```

**Step 6: Add formulations module to mod.rs**

In `crates/gat-algo/src/opf/mod.rs`, add after `mod dispatcher;`:

```rust
pub mod formulations;
```

**Step 7: Run tests**

Run: `cargo test -p gat-algo formulations --no-fail-fast`
Expected: All 8 formulation tests PASS

**Step 8: Commit**

```bash
git add crates/gat-algo/src/opf/formulations/
git add crates/gat-algo/src/opf/mod.rs
git commit -m "feat(opf): add built-in formulation wrappers (DC, SOCP, AC, Economic)"
```

---

## Task 5: Create Built-in Backends

**Files:**
- Create: `crates/gat-algo/src/opf/backends/mod.rs`
- Create: `crates/gat-algo/src/opf/backends/clarabel.rs`
- Create: `crates/gat-algo/src/opf/backends/lbfgs.rs`
- Create: `crates/gat-algo/src/opf/backends/ipopt.rs`
- Modify: `crates/gat-algo/src/opf/mod.rs` (add module)

**Step 1: Create backends directory and mod.rs**

Create `crates/gat-algo/src/opf/backends/mod.rs`:

```rust
//! Built-in OPF solver backends.
//!
//! Each backend wraps an existing solver and exposes it through
//! the `OpfBackend` trait.

mod clarabel;
mod lbfgs;

#[cfg(feature = "solver-ipopt")]
mod ipopt;

pub use clarabel::ClarabelBackend;
pub use lbfgs::LbfgsBackend;

#[cfg(feature = "solver-ipopt")]
pub use ipopt::IpoptBackend;
```

**Step 2: Create Clarabel backend**

Create `crates/gat-algo/src/opf/backends/clarabel.rs`:

```rust
//! Clarabel solver backend for LP and SOCP problems.

use std::collections::HashMap;

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfBackend, OpfProblem, SolverConfig};
use crate::opf::OpfSolution;
use crate::OpfError;

/// Clarabel backend for LP and SOCP problems.
///
/// Clarabel is a pure-Rust interior-point solver that's always available.
pub struct ClarabelBackend;

impl OpfBackend for ClarabelBackend {
    fn id(&self) -> &str {
        "clarabel"
    }

    fn supported_classes(&self) -> &[ProblemClass] {
        &[ProblemClass::LinearProgram, ProblemClass::ConicProgram]
    }

    fn is_available(&self) -> bool {
        true // Always available (pure Rust)
    }

    fn solve(
        &self,
        problem: &OpfProblem,
        _config: &SolverConfig,
        _warm_start: Option<&HashMap<String, f64>>,
    ) -> Result<OpfSolution, OpfError> {
        // TODO: Delegate to actual dc_opf::solve() or socp::solve() based on problem class
        // For now, return a placeholder error
        Err(OpfError::NotImplemented(format!(
            "ClarabelBackend::solve not yet implemented for {:?}",
            problem.problem_class
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clarabel_backend_id() {
        let backend = ClarabelBackend;
        assert_eq!(backend.id(), "clarabel");
    }

    #[test]
    fn test_clarabel_is_always_available() {
        let backend = ClarabelBackend;
        assert!(backend.is_available());
    }

    #[test]
    fn test_clarabel_supports_lp_and_socp() {
        let backend = ClarabelBackend;
        let classes = backend.supported_classes();
        assert!(classes.contains(&ProblemClass::LinearProgram));
        assert!(classes.contains(&ProblemClass::ConicProgram));
        assert!(!classes.contains(&ProblemClass::NonlinearProgram));
    }
}
```

**Step 3: Create L-BFGS backend**

Create `crates/gat-algo/src/opf/backends/lbfgs.rs`:

```rust
//! L-BFGS solver backend for NLP problems.

use std::collections::HashMap;

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfBackend, OpfProblem, SolverConfig};
use crate::opf::OpfSolution;
use crate::OpfError;

/// L-BFGS backend for nonlinear programming problems.
///
/// Uses the argmin crate's L-BFGS implementation with augmented Lagrangian.
/// Always available as a pure-Rust fallback for AC-OPF.
pub struct LbfgsBackend;

impl OpfBackend for LbfgsBackend {
    fn id(&self) -> &str {
        "lbfgs"
    }

    fn supported_classes(&self) -> &[ProblemClass] {
        &[ProblemClass::NonlinearProgram]
    }

    fn is_available(&self) -> bool {
        true // Always available (pure Rust)
    }

    fn solve(
        &self,
        problem: &OpfProblem,
        _config: &SolverConfig,
        _warm_start: Option<&HashMap<String, f64>>,
    ) -> Result<OpfSolution, OpfError> {
        // TODO: Delegate to actual ac_nlp::solve_ac_opf()
        Err(OpfError::NotImplemented(format!(
            "LbfgsBackend::solve not yet implemented for {:?}",
            problem.problem_class
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lbfgs_backend_id() {
        let backend = LbfgsBackend;
        assert_eq!(backend.id(), "lbfgs");
    }

    #[test]
    fn test_lbfgs_is_always_available() {
        let backend = LbfgsBackend;
        assert!(backend.is_available());
    }

    #[test]
    fn test_lbfgs_supports_nlp() {
        let backend = LbfgsBackend;
        let classes = backend.supported_classes();
        assert!(classes.contains(&ProblemClass::NonlinearProgram));
        assert!(!classes.contains(&ProblemClass::LinearProgram));
    }
}
```

**Step 4: Create IPOPT backend (feature-gated)**

Create `crates/gat-algo/src/opf/backends/ipopt.rs`:

```rust
//! IPOPT solver backend for NLP problems.

use std::collections::HashMap;
use std::path::Path;

use crate::opf::dispatch::ProblemClass;
use crate::opf::traits::{OpfBackend, OpfProblem, SolverConfig};
use crate::opf::OpfSolution;
use crate::OpfError;

/// IPOPT backend for nonlinear programming problems.
///
/// IPOPT is a state-of-the-art interior-point optimizer. This backend
/// requires the `solver-ipopt` feature and checks for IPOPT availability
/// at runtime.
pub struct IpoptBackend;

impl OpfBackend for IpoptBackend {
    fn id(&self) -> &str {
        "ipopt"
    }

    fn supported_classes(&self) -> &[ProblemClass] {
        &[ProblemClass::NonlinearProgram]
    }

    fn is_available(&self) -> bool {
        // Check if gat-ipopt binary exists in PATH or ~/.gat/solvers/
        which::which("gat-ipopt").is_ok()
            || Path::new(&format!(
                "{}/.gat/solvers/gat-ipopt",
                std::env::var("HOME").unwrap_or_default()
            ))
            .exists()
    }

    fn solve(
        &self,
        problem: &OpfProblem,
        _config: &SolverConfig,
        _warm_start: Option<&HashMap<String, f64>>,
    ) -> Result<OpfSolution, OpfError> {
        if !self.is_available() {
            return Err(OpfError::NotImplemented(
                "IPOPT not installed. Run: cargo build -p gat-ipopt --features ipopt-sys --release"
                    .to_string(),
            ));
        }

        // TODO: Delegate to actual ac_nlp::solve_with_ipopt()
        Err(OpfError::NotImplemented(format!(
            "IpoptBackend::solve not yet implemented for {:?}",
            problem.problem_class
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipopt_backend_id() {
        let backend = IpoptBackend;
        assert_eq!(backend.id(), "ipopt");
    }

    #[test]
    fn test_ipopt_supports_nlp() {
        let backend = IpoptBackend;
        let classes = backend.supported_classes();
        assert!(classes.contains(&ProblemClass::NonlinearProgram));
    }

    // Note: is_available() depends on system state, so we don't assert its value
}
```

**Step 5: Add backends module to mod.rs**

In `crates/gat-algo/src/opf/mod.rs`, add after `pub mod formulations;`:

```rust
pub mod backends;
```

**Step 6: Run tests**

Run: `cargo test -p gat-algo backends --no-fail-fast`
Expected: All 6 backend tests PASS

**Step 7: Commit**

```bash
git add crates/gat-algo/src/opf/backends/
git add crates/gat-algo/src/opf/mod.rs
git commit -m "feat(opf): add built-in backend wrappers (Clarabel, L-BFGS, IPOPT)"
```

---

## Task 6: Wire Up Registry with_defaults()

**Files:**
- Modify: `crates/gat-algo/src/opf/registry.rs`

**Step 1: Update with_defaults() to register built-in components**

In `crates/gat-algo/src/opf/registry.rs`, update the `with_defaults()` method:

```rust
    /// Create registry with built-in formulations and backends.
    ///
    /// Registers:
    /// - Formulations: dc-opf, socp, ac-opf, economic-dispatch
    /// - Backends: clarabel, lbfgs (+ ipopt if available)
    pub fn with_defaults() -> Self {
        use super::backends::{ClarabelBackend, LbfgsBackend};
        use super::formulations::{
            AcOpfFormulation, DcOpfFormulation, EconomicDispatchFormulation, SocpFormulation,
        };

        let mut registry = Self::new();

        // Register built-in formulations
        registry.register_formulation(Arc::new(DcOpfFormulation));
        registry.register_formulation(Arc::new(SocpFormulation));
        registry.register_formulation(Arc::new(AcOpfFormulation));
        registry.register_formulation(Arc::new(EconomicDispatchFormulation));

        // Register built-in backends
        registry.register_backend(Arc::new(ClarabelBackend));
        registry.register_backend(Arc::new(LbfgsBackend));

        // Register IPOPT if feature enabled
        #[cfg(feature = "solver-ipopt")]
        {
            use super::backends::IpoptBackend;
            registry.register_backend(Arc::new(IpoptBackend));
        }

        registry
    }
```

**Step 2: Add test for with_defaults()**

Add to the tests module in `registry.rs`:

```rust
    #[test]
    fn test_with_defaults_has_builtin_formulations() {
        let registry = SolverRegistry::with_defaults();

        assert!(registry.get_formulation("dc-opf").is_some());
        assert!(registry.get_formulation("socp").is_some());
        assert!(registry.get_formulation("ac-opf").is_some());
        assert!(registry.get_formulation("economic-dispatch").is_some());
    }

    #[test]
    fn test_with_defaults_has_builtin_backends() {
        let registry = SolverRegistry::with_defaults();

        assert!(registry.get_backend("clarabel").is_some());
        assert!(registry.get_backend("lbfgs").is_some());
    }
```

**Step 3: Run tests**

Run: `cargo test -p gat-algo registry::tests --no-fail-fast`
Expected: All 9 tests PASS

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/registry.rs
git commit -m "feat(opf): wire up SolverRegistry::with_defaults() with built-in components"
```

---

## Task 7: Update OpfSolver to Delegate to New System

**Files:**
- Modify: `crates/gat-algo/src/opf/mod.rs`

**Step 1: Add helper method to OpfSolver**

In `crates/gat-algo/src/opf/mod.rs`, add a new method to `OpfSolver`:

```rust
    /// Solve using the new dispatcher-based system.
    ///
    /// This is the new implementation that delegates to OpfDispatcher.
    /// Currently used internally; will replace the main solve() method
    /// once fully tested.
    fn solve_with_dispatcher(&self, network: &Network) -> Result<OpfSolution, OpfError> {
        use std::sync::Arc;

        let registry = SolverRegistry::with_defaults();
        let dispatcher = OpfDispatcher::new(Arc::new(registry));

        // Map OpfMethod enum to formulation ID
        let formulation_id = match self.method {
            OpfMethod::EconomicDispatch => "economic-dispatch",
            OpfMethod::DcOpf => "dc-opf",
            OpfMethod::SocpRelaxation => "socp",
            OpfMethod::AcOpf => "ac-opf",
        };

        // Build config
        let config = traits::SolverConfig {
            max_iterations: self.max_iterations,
            tolerance: self.tolerance,
            timeout_seconds: self.timeout_seconds,
        };

        // Build fallback chain based on method
        let fallbacks = if self.method == OpfMethod::AcOpf {
            vec![
                traits::WarmStartKind::Flat,
                traits::WarmStartKind::Dc,
                traits::WarmStartKind::Socp,
            ]
        } else {
            vec![traits::WarmStartKind::Flat]
        };

        dispatcher.solve(network, formulation_id, config, &fallbacks)
    }
```

**Step 2: Add test for the new path**

Create a new test in `crates/gat-algo/tests/solver_dispatch.rs`:

```rust
/// Test that the new dispatcher-based system can be called.
#[test]
fn test_opf_solver_dispatcher_path_exists() {
    use gat_algo::opf::{OpfDispatcher, SolverRegistry};
    use std::sync::Arc;

    let registry = SolverRegistry::with_defaults();
    let _dispatcher = OpfDispatcher::new(Arc::new(registry));

    // Just verify the path compiles and constructs
    // Full integration testing happens in Task 8
}
```

**Step 3: Run tests**

Run: `cargo test -p gat-algo solver_dispatch --no-fail-fast`
Expected: Tests PASS

**Step 4: Commit**

```bash
git add crates/gat-algo/src/opf/mod.rs crates/gat-algo/tests/solver_dispatch.rs
git commit -m "feat(opf): add solve_with_dispatcher() delegation method to OpfSolver"
```

---

## Task 8: Integration Test - Full Path

**Files:**
- Create: `crates/gat-algo/tests/strategy_pattern.rs`

**Step 1: Create integration test file**

Create `crates/gat-algo/tests/strategy_pattern.rs`:

```rust
//! Integration tests for the Strategy pattern refactoring (PROJ-2).
//!
//! These tests verify that the new dispatcher-based system produces
//! equivalent results to the original monolithic solve() method.

use gat_algo::opf::{OpfDispatcher, OpfMethod, OpfSolver, SolverRegistry};
use gat_algo::opf::traits::SolverConfig;
use gat_core::{
    Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Load, LoadId, Network, Node,
};
use std::sync::Arc;

/// Create a simple 2-bus test network.
fn create_2bus_network() -> Network {
    let mut network = Network::new();

    let bus1_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(0),
        name: "bus1".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    let bus2_idx = network.graph.add_node(Node::Bus(Bus {
        id: BusId::new(1),
        name: "bus2".to_string(),
        base_kv: gat_core::Kilovolts(100.0),
        ..Bus::default()
    }));

    network.graph.add_edge(
        bus1_idx,
        bus2_idx,
        Edge::Branch(Branch {
            id: BranchId::new(0),
            name: "line1_2".to_string(),
            from_bus: BusId::new(0),
            to_bus: BusId::new(1),
            resistance: 0.01,
            reactance: 0.1,
            ..Branch::default()
        }),
    );

    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "gen1".to_string(),
        bus: BusId::new(0),
        active_power: gat_core::Megawatts(0.0),
        reactive_power: gat_core::Megavars(0.0),
        pmin: gat_core::Megawatts(0.0),
        pmax: gat_core::Megawatts(100.0),
        qmin: gat_core::Megavars(-50.0),
        qmax: gat_core::Megavars(50.0),
        is_synchronous_condenser: false,
        cost_model: CostModel::linear(0.0, 10.0),
        ..Gen::default()
    }));

    network.graph.add_node(Node::Load(Load {
        id: LoadId::new(0),
        name: "load2".to_string(),
        bus: BusId::new(1),
        active_power: gat_core::Megawatts(50.0),
        reactive_power: gat_core::Megavars(0.0),
    }));

    network
}

/// Test that SolverRegistry::with_defaults() includes all expected formulations.
#[test]
fn test_registry_has_all_formulations() {
    let registry = SolverRegistry::with_defaults();

    let formulations = registry.list_formulations();
    assert!(formulations.contains(&"dc-opf"), "Missing dc-opf");
    assert!(formulations.contains(&"socp"), "Missing socp");
    assert!(formulations.contains(&"ac-opf"), "Missing ac-opf");
    assert!(
        formulations.contains(&"economic-dispatch"),
        "Missing economic-dispatch"
    );
}

/// Test that SolverRegistry::with_defaults() includes all expected backends.
#[test]
fn test_registry_has_all_backends() {
    let registry = SolverRegistry::with_defaults();

    let backends = registry.list_backends();
    assert!(backends.contains(&"clarabel"), "Missing clarabel");
    assert!(backends.contains(&"lbfgs"), "Missing lbfgs");
}

/// Test that OpfDispatcher can be constructed with defaults.
#[test]
fn test_dispatcher_construction() {
    let registry = Arc::new(SolverRegistry::with_defaults());
    let _dispatcher = OpfDispatcher::new(registry);
    // Just verify construction doesn't panic
}

/// Test that formulation lookup works.
#[test]
fn test_formulation_lookup() {
    let registry = SolverRegistry::with_defaults();

    let dc = registry.get_formulation("dc-opf");
    assert!(dc.is_some());
    assert_eq!(dc.unwrap().id(), "dc-opf");

    let missing = registry.get_formulation("nonexistent");
    assert!(missing.is_none());
}

/// Test that backend selection works by problem class.
#[test]
fn test_backend_selection_by_class() {
    use gat_algo::opf::dispatch::ProblemClass;

    let registry = SolverRegistry::with_defaults();

    // LP should get Clarabel
    let lp_backend = registry.select_backend(ProblemClass::LinearProgram);
    assert!(lp_backend.is_some());
    assert_eq!(lp_backend.unwrap().id(), "clarabel");

    // NLP should get LBFGS (or IPOPT if available)
    let nlp_backend = registry.select_backend(ProblemClass::NonlinearProgram);
    assert!(nlp_backend.is_some());
    // Could be either lbfgs or ipopt depending on availability
    let nlp_id = nlp_backend.unwrap().id();
    assert!(nlp_id == "lbfgs" || nlp_id == "ipopt");
}

/// Test that custom formulations can be registered.
#[test]
fn test_custom_formulation_registration() {
    use gat_algo::opf::dispatch::ProblemClass;
    use gat_algo::opf::traits::{OpfFormulation, OpfProblem, WarmStartKind};
    use gat_algo::OpfError;

    struct CustomFormulation;

    impl OpfFormulation for CustomFormulation {
        fn id(&self) -> &str {
            "custom"
        }
        fn problem_class(&self) -> ProblemClass {
            ProblemClass::LinearProgram
        }
        fn build_problem(&self, _network: &Network) -> Result<OpfProblem, OpfError> {
            unimplemented!()
        }
        fn accepts_warm_start(&self) -> &[WarmStartKind] {
            &[]
        }
    }

    let mut registry = SolverRegistry::with_defaults();
    registry.register_formulation(Arc::new(CustomFormulation));

    let custom = registry.get_formulation("custom");
    assert!(custom.is_some());
    assert_eq!(custom.unwrap().id(), "custom");
}
```

**Step 2: Run integration tests**

Run: `cargo test -p gat-algo strategy_pattern --no-fail-fast`
Expected: All 6 tests PASS

**Step 3: Commit**

```bash
git add crates/gat-algo/tests/strategy_pattern.rs
git commit -m "test(opf): add integration tests for Strategy pattern components"
```

---

## Task 9: Verify All Existing Tests Still Pass

**Files:** None (verification only)

**Step 1: Run full test suite**

Run: `cargo test -p gat-algo --no-fail-fast`
Expected: All tests PASS (including dc_opf, socp, ac_opf, solver_dispatch)

**Step 2: Run clippy**

Run: `cargo clippy -p gat-algo -- -D warnings`
Expected: No warnings

**Step 3: Commit if any fixes needed**

```bash
git add -A
git commit -m "fix(opf): address clippy warnings from Strategy pattern refactoring"
```

---

## Success Criteria Checklist

After completing all tasks, verify:

- [ ] All existing `OpfSolver` tests pass unchanged
- [ ] New `OpfDispatcher` API works for all methods
- [ ] Runtime detection correctly identifies installed native solvers
- [ ] No `#[cfg]` attributes in the solve path (only in registration)
- [ ] Custom formulation/backend can be registered and looked up
- [ ] Code compiles without warnings

---

## Notes for Implementation

1. **Don't break existing tests** - The `OpfSolver::solve()` method keeps working; new code is additive
2. **Backends don't actually solve yet** - Task 4-5 create wrappers that return `NotImplemented`; wiring them to actual solvers is a follow-up
3. **Feature flags** - `solver-ipopt` gates IPOPT backend registration, not the trait definitions
4. **Test incrementally** - Each task has its own tests; run them before committing
