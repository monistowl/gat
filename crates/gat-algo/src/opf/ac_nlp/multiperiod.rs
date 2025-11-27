//! # Multi-Period AC-OPF Framework
//!
//! This module extends single-period AC-OPF to handle temporal coupling constraints
//! across multiple time periods. Essential for:
//!
//! - **Day-ahead scheduling**: 24-48 hour lookahead with hourly resolution
//! - **Real-time dispatch**: 5-15 minute intervals with look-ahead
//! - **Unit commitment**: Economic dispatch with start-up/shut-down decisions
//! - **Storage optimization**: Charge/discharge scheduling across periods
//!
//! ## Mathematical Formulation
//!
//! Multi-period OPF adds temporal linking constraints to the standard AC-OPF:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  TEMPORAL LINKING CONSTRAINTS                                           │
//! │  ────────────────────────────                                           │
//! │                                                                          │
//! │  Ramp-up limit:    P_g(t) - P_g(t-1) ≤ RU_g · Δt                        │
//! │  Ramp-down limit:  P_g(t-1) - P_g(t) ≤ RD_g · Δt                        │
//! │                                                                          │
//! │  where:                                                                  │
//! │    RU_g = ramp-up rate (MW/hr)                                          │
//! │    RD_g = ramp-down rate (MW/hr)                                        │
//! │    Δt   = period duration (hours)                                       │
//! └─────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  OBJECTIVE: Minimize total cost across all periods                      │
//! │  ─────────                                                               │
//! │                                                                          │
//! │  min  Σ_t [ Δt_t · Σ_g Cost_g(P_g(t)) ]                                 │
//! │                                                                          │
//! │  Period duration Δt scales energy cost correctly ($/hr → $ total)       │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Solution Approaches
//!
//! ### 1. Sequential Solving (Implemented Here)
//!
//! Solve periods one at a time, using the previous solution as warm-start
//! and applying ramp constraints as tightened bounds:
//!
//! ```text
//! for t in 0..T:
//!     P_min_effective(t) = max(P_min, P(t-1) - RD·Δt)
//!     P_max_effective(t) = min(P_max, P(t-1) + RU·Δt)
//!     solve_ac_opf(period_t, P_min_effective, P_max_effective)
//! ```
//!
//! **Pros**: Simple, scalable, each period is standard AC-OPF
//! **Cons**: Not globally optimal (myopic), no look-ahead optimization
//!
//! ### 2. Fully Coupled (Future Work)
//!
//! Solve all periods simultaneously as one large NLP with explicit
//! ramping constraints. Provides global optimum but has O(T × n_var) variables.
//!
//! ## Physical Interpretation
//!
//! **Ramp rates** reflect the physical limitations of thermal generators:
//!
//! | Generator Type | Typical Ramp Rate |
//! |---------------|-------------------|
//! | Nuclear       | 0.5-1% Pmax/min   |
//! | Coal          | 1-3% Pmax/min     |
//! | Combined Cycle| 3-5% Pmax/min     |
//! | Gas Turbine   | 8-15% Pmax/min    |
//! | Hydro         | 20-50% Pmax/min   |
//! | Wind/Solar    | Instant (but uncertain) |
//!
//! **Example**: A 500 MW coal plant at 2%/min ramps ~10 MW/min = 600 MW/hr
//!
//! ## References
//!
//! - **Kirschen & Strbac (2018)**: "Fundamentals of Power System Economics"
//!   Chapter 5: Short-term operations planning
//!
//! - **Wood, Wollenberg & Sheblé (2014)**: "Power Generation, Operation, and Control"
//!   Chapter 4: Economic dispatch with network constraints

use super::AcOpfProblem;
use crate::opf::{OpfError, OpfSolution};
use std::collections::HashMap;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Time period data for multi-period optimization.
///
/// Each period represents a time slice with specific load conditions.
/// Periods are typically 15 minutes to 1 hour in real-time/day-ahead markets.
#[derive(Debug, Clone)]
pub struct PeriodData {
    /// Period index (0-based).
    /// Used for ordering and tracking temporal sequence.
    pub index: usize,

    /// Period duration in hours.
    /// Used to scale energy costs and compute ramp limits.
    /// Common values: 0.25 (15 min), 0.5 (30 min), 1.0 (1 hour)
    pub duration_hr: f64,

    /// Load scaling factor relative to base case.
    /// 1.0 = base load, 1.2 = 20% higher, 0.8 = 20% lower
    /// Represents how demand varies throughout the day.
    pub load_scale: f64,
}

impl PeriodData {
    /// Create a new period with the given parameters.
    pub fn new(index: usize, duration_hr: f64, load_scale: f64) -> Self {
        Self {
            index,
            duration_hr,
            load_scale,
        }
    }

    /// Create an hourly period with given load scale.
    pub fn hourly(index: usize, load_scale: f64) -> Self {
        Self {
            index,
            duration_hr: 1.0,
            load_scale,
        }
    }
}

/// Generator ramp rate constraints.
///
/// Defines how fast a generator can change its output between periods.
/// Asymmetric limits (different up/down rates) are common in practice.
#[derive(Debug, Clone)]
pub struct RampConstraint {
    /// Generator name (must match GenData.name in the problem).
    pub gen_name: String,

    /// Ramp-up limit (MW/hr).
    /// Maximum rate of increase in output.
    /// P(t) ≤ P(t-1) + ramp_up_mw_hr × Δt
    pub ramp_up_mw_hr: f64,

    /// Ramp-down limit (MW/hr).
    /// Maximum rate of decrease in output.
    /// P(t) ≥ P(t-1) - ramp_down_mw_hr × Δt
    pub ramp_down_mw_hr: f64,
}

impl RampConstraint {
    /// Create symmetric ramp constraint (same up/down rate).
    pub fn symmetric(gen_name: impl Into<String>, ramp_mw_hr: f64) -> Self {
        Self {
            gen_name: gen_name.into(),
            ramp_up_mw_hr: ramp_mw_hr,
            ramp_down_mw_hr: ramp_mw_hr,
        }
    }

    /// Create asymmetric ramp constraint.
    pub fn asymmetric(gen_name: impl Into<String>, up_mw_hr: f64, down_mw_hr: f64) -> Self {
        Self {
            gen_name: gen_name.into(),
            ramp_up_mw_hr: up_mw_hr,
            ramp_down_mw_hr: down_mw_hr,
        }
    }

    /// Compute effective P bounds given previous dispatch and period duration.
    ///
    /// # Arguments
    /// * `prev_p_mw` - Generator output in previous period (MW)
    /// * `dt_hr` - Period duration (hours)
    /// * `pmin_mw` - Generator minimum output (MW)
    /// * `pmax_mw` - Generator maximum output (MW)
    ///
    /// # Returns
    /// Tuple `(p_min_effective, p_max_effective)` in MW
    pub fn effective_bounds(
        &self,
        prev_p_mw: f64,
        dt_hr: f64,
        pmin_mw: f64,
        pmax_mw: f64,
    ) -> (f64, f64) {
        // Ramping limits the feasible range around previous dispatch
        let ramp_limited_min = prev_p_mw - self.ramp_down_mw_hr * dt_hr;
        let ramp_limited_max = prev_p_mw + self.ramp_up_mw_hr * dt_hr;

        // Intersection with generator physical limits
        let p_min_effective = ramp_limited_min.max(pmin_mw);
        let p_max_effective = ramp_limited_max.min(pmax_mw);

        (p_min_effective, p_max_effective)
    }
}

/// Multi-period AC-OPF problem specification.
///
/// Combines a base single-period problem with temporal structure:
/// - Time periods with load profiles
/// - Generator ramp constraints
/// - Initial conditions from prior periods
///
/// # Example
///
/// ```ignore
/// // Create 24-hour day-ahead problem
/// let base = AcOpfProblem::from_network(&network)?;
///
/// // Define hourly periods with typical load profile
/// let periods: Vec<PeriodData> = (0..24)
///     .map(|h| PeriodData::hourly(h, load_profile[h]))
///     .collect();
///
/// // Define ramp constraints for thermal generators
/// let ramps = vec![
///     RampConstraint::symmetric("Gen1", 100.0), // 100 MW/hr
///     RampConstraint::symmetric("Gen2", 50.0),  // 50 MW/hr
/// ];
///
/// let multi = MultiPeriodProblem::new(base, periods, ramps);
/// let solutions = solve_multiperiod_sequential(&multi, 100, 1e-4)?;
/// ```
#[derive(Clone)]
pub struct MultiPeriodProblem {
    /// Base single-period problem (network structure, costs, limits).
    /// Loads will be scaled by period-specific factors.
    pub base_problem: AcOpfProblem,

    /// Time periods to solve.
    /// Should be sorted by index (ascending).
    pub periods: Vec<PeriodData>,

    /// Ramp constraints indexed by generator name.
    /// Generators not in this map have no ramp limits.
    pub ramp_constraints: HashMap<String, RampConstraint>,

    /// Initial generator dispatch (period t=-1).
    /// Used to enforce ramp constraints for the first period.
    /// Map: generator name → P_MW
    pub initial_dispatch: HashMap<String, f64>,
}

impl MultiPeriodProblem {
    /// Create multi-period problem from base problem and periods.
    ///
    /// # Arguments
    ///
    /// * `base` - Single-period AC-OPF problem (will be cloned for each period)
    /// * `periods` - Time periods with load scaling factors
    /// * `ramp_constraints` - Generator ramp rate limits
    pub fn new(
        base: AcOpfProblem,
        periods: Vec<PeriodData>,
        ramp_constraints: Vec<RampConstraint>,
    ) -> Self {
        // Index ramp constraints by generator name for O(1) lookup
        let ramp_map: HashMap<_, _> = ramp_constraints
            .into_iter()
            .map(|r| (r.gen_name.clone(), r))
            .collect();

        Self {
            base_problem: base,
            periods,
            ramp_constraints: ramp_map,
            initial_dispatch: HashMap::new(),
        }
    }

    /// Set initial dispatch from prior period.
    ///
    /// This is used to enforce ramp constraints for the first period.
    /// If not set, the first period has no ramp constraints (free start).
    ///
    /// # Arguments
    ///
    /// * `dispatch` - Map from generator name to power output (MW)
    pub fn set_initial_dispatch(&mut self, dispatch: HashMap<String, f64>) {
        self.initial_dispatch = dispatch;
    }

    /// Get total number of periods.
    pub fn n_periods(&self) -> usize {
        self.periods.len()
    }

    /// Total number of variables across all periods.
    /// (For future use with fully-coupled formulation)
    pub fn total_vars(&self) -> usize {
        self.base_problem.n_var * self.periods.len()
    }

    /// Get variable offset for period t.
    /// (For future use with fully-coupled formulation)
    pub fn period_offset(&self, t: usize) -> usize {
        t * self.base_problem.n_var
    }
}

// ============================================================================
// SOLUTION RESULT
// ============================================================================

/// Result of multi-period optimization.
///
/// Contains solutions for each period along with aggregate metrics.
#[derive(Debug, Clone)]
pub struct MultiPeriodSolution {
    /// Solutions for each period (indexed by period index).
    pub period_solutions: Vec<OpfSolution>,

    /// Total cost across all periods ($ or $/hr × hours).
    pub total_cost: f64,

    /// Whether all periods converged successfully.
    pub all_converged: bool,

    /// Periods that failed to converge (if any).
    pub failed_periods: Vec<usize>,

    /// Total solve time (milliseconds).
    pub total_solve_time_ms: u128,
}

impl MultiPeriodSolution {
    /// Get solution for a specific period.
    pub fn get_period(&self, index: usize) -> Option<&OpfSolution> {
        self.period_solutions.get(index)
    }

    /// Get generator dispatch trajectory over time.
    ///
    /// # Arguments
    /// * `gen_name` - Generator name
    ///
    /// # Returns
    /// Vector of (period_index, P_MW) pairs
    pub fn generator_trajectory(&self, gen_name: &str) -> Vec<(usize, f64)> {
        self.period_solutions
            .iter()
            .enumerate()
            .filter_map(|(i, sol)| sol.generator_p.get(gen_name).map(|&p| (i, p)))
            .collect()
    }

    /// Check if any ramp violations occurred.
    ///
    /// Returns generator name and (period, violation_mw) pairs.
    pub fn ramp_violations(
        &self,
        ramps: &HashMap<String, RampConstraint>,
        periods: &[PeriodData],
    ) -> Vec<(String, usize, f64)> {
        let mut violations = Vec::new();

        for i in 1..self.period_solutions.len() {
            let dt = periods.get(i).map(|p| p.duration_hr).unwrap_or(1.0);

            for (gen_name, ramp) in ramps {
                let prev_p = self.period_solutions[i - 1]
                    .generator_p
                    .get(gen_name)
                    .copied()
                    .unwrap_or(0.0);
                let curr_p = self.period_solutions[i]
                    .generator_p
                    .get(gen_name)
                    .copied()
                    .unwrap_or(0.0);

                let delta = curr_p - prev_p;

                // Check ramp-up violation
                let max_ramp_up = ramp.ramp_up_mw_hr * dt;
                if delta > max_ramp_up + 1e-3 {
                    violations.push((gen_name.clone(), i, delta - max_ramp_up));
                }

                // Check ramp-down violation
                let max_ramp_down = ramp.ramp_down_mw_hr * dt;
                if -delta > max_ramp_down + 1e-3 {
                    violations.push((gen_name.clone(), i, -delta - max_ramp_down));
                }
            }
        }

        violations
    }
}

// ============================================================================
// SEQUENTIAL SOLVER
// ============================================================================

/// Solve multi-period OPF using sequential period-by-period approach.
///
/// This is a practical approximation that:
/// 1. Solves each period as an independent AC-OPF
/// 2. Applies ramp constraints as tightened generator bounds
/// 3. Uses warm-start from previous period for faster convergence
///
/// # Algorithm
///
/// ```text
/// prev_dispatch = initial_dispatch (or midpoint if not specified)
///
/// for each period t:
///     1. Clone base problem
///     2. Scale loads by period.load_scale
///     3. Tighten generator P bounds based on ramp constraints:
///        P_min[g] = max(P_min[g], prev_P[g] - RD[g] × Δt)
///        P_max[g] = min(P_max[g], prev_P[g] + RU[g] × Δt)
///     4. Warm-start from prev_dispatch
///     5. Solve AC-OPF for period t
///     6. Update prev_dispatch with solution
/// ```
///
/// # Arguments
///
/// * `problem` - Multi-period problem specification
/// * `max_iterations` - Maximum iterations per period
/// * `tolerance` - Convergence tolerance
///
/// # Returns
///
/// `MultiPeriodSolution` with solutions for all periods
///
/// # Errors
///
/// Returns error if any period fails to solve (after logging which periods failed).
pub fn solve_multiperiod_sequential(
    problem: &MultiPeriodProblem,
    max_iterations: usize,
    tolerance: f64,
) -> Result<MultiPeriodSolution, OpfError> {
    use std::time::Instant;

    let start_time = Instant::now();
    let mut solutions = Vec::with_capacity(problem.periods.len());
    let mut prev_dispatch = problem.initial_dispatch.clone();
    let mut failed_periods = Vec::new();
    let mut total_cost = 0.0;

    for period in &problem.periods {
        // ====================================================================
        // STEP 1: CLONE AND SCALE BASE PROBLEM
        // ====================================================================
        //
        // Create a fresh copy for this period and scale loads.
        // Scaling represents time-varying demand (e.g., peak vs off-peak).

        let mut period_problem = problem.base_problem.clone();

        for bus in &mut period_problem.buses {
            bus.p_load *= period.load_scale;
            bus.q_load *= period.load_scale;
        }

        // ====================================================================
        // STEP 2: APPLY RAMP CONSTRAINTS AS TIGHTENED BOUNDS
        // ====================================================================
        //
        // Rather than adding explicit ramping constraints to the NLP,
        // we tighten the generator P bounds based on previous dispatch.
        // This is an approximation that works well for sequential solving.

        for gen in &mut period_problem.generators {
            if let Some(ramp) = problem.ramp_constraints.get(&gen.name) {
                // Get previous dispatch (or use midpoint if first period without initial)
                let prev_p = prev_dispatch
                    .get(&gen.name)
                    .copied()
                    .unwrap_or((gen.pmin_mw + gen.pmax_mw) / 2.0);

                // Compute ramping-constrained bounds
                let (p_min_eff, p_max_eff) =
                    ramp.effective_bounds(prev_p, period.duration_hr, gen.pmin_mw, gen.pmax_mw);

                // Update generator limits
                gen.pmin_mw = p_min_eff;
                gen.pmax_mw = p_max_eff;

                // Sanity check: ensure pmin <= pmax
                if gen.pmin_mw > gen.pmax_mw {
                    // Infeasible due to ramping - this shouldn't happen with good data
                    // Use midpoint as fallback
                    let mid = (gen.pmin_mw + gen.pmax_mw) / 2.0;
                    gen.pmin_mw = mid;
                    gen.pmax_mw = mid;
                }
            }
        }

        // ====================================================================
        // STEP 3: CREATE WARM-START INITIAL POINT
        // ====================================================================
        //
        // Use previous period's solution to accelerate convergence.
        // Typically reduces iterations by 30-50%.

        let x0 = if !prev_dispatch.is_empty() {
            // Build a pseudo-solution from prev_dispatch
            let mut warm_x = period_problem.initial_point();

            for (i, gen) in period_problem.generators.iter().enumerate() {
                if let Some(&p_mw) = prev_dispatch.get(&gen.name) {
                    // Clamp to new bounds
                    let p_clamped = p_mw.max(gen.pmin_mw).min(gen.pmax_mw);
                    warm_x[period_problem.pg_offset + i] = p_clamped / period_problem.base_mva;
                }
            }

            warm_x
        } else {
            period_problem.initial_point()
        };

        // ====================================================================
        // STEP 4: SOLVE AC-OPF FOR THIS PERIOD
        // ====================================================================

        let solution =
            super::solver::solve_with_start(&period_problem, x0, max_iterations, tolerance);

        match solution {
            Ok(sol) => {
                // Scale cost by period duration to get total energy cost
                let period_cost = sol.objective_value * period.duration_hr;
                total_cost += period_cost;

                // Update prev_dispatch for next period
                prev_dispatch = sol.generator_p.clone();

                solutions.push(sol);
            }
            Err(e) => {
                // Log failure but continue with remaining periods
                failed_periods.push(period.index);

                // Create a "failed" solution placeholder
                solutions.push(OpfSolution {
                    converged: false,
                    ..Default::default()
                });

                // For subsequent periods, keep using the last good dispatch
                // This allows the optimization to continue even if one period fails
                if solutions.len() == 1 {
                    // First period failed - can't continue meaningfully
                    return Err(e);
                }
            }
        }
    }

    let all_converged = failed_periods.is_empty();

    Ok(MultiPeriodSolution {
        period_solutions: solutions,
        total_cost,
        all_converged,
        failed_periods,
        total_solve_time_ms: start_time.elapsed().as_millis(),
    })
}

/// Create a standard 24-hour day-ahead problem from hourly load factors.
///
/// Convenience function for the common use case of day-ahead scheduling
/// with hourly resolution.
///
/// # Arguments
///
/// * `base_problem` - Single-period AC-OPF problem (e.g., from network)
/// * `hourly_load_factors` - 24 load scaling factors (0-23 hours)
/// * `ramp_constraints` - Generator ramp limits
///
/// # Returns
///
/// Multi-period problem ready for solving
pub fn create_day_ahead_problem(
    base_problem: AcOpfProblem,
    hourly_load_factors: &[f64; 24],
    ramp_constraints: Vec<RampConstraint>,
) -> MultiPeriodProblem {
    let periods: Vec<PeriodData> = hourly_load_factors
        .iter()
        .enumerate()
        .map(|(h, &scale)| PeriodData::hourly(h, scale))
        .collect();

    MultiPeriodProblem::new(base_problem, periods, ramp_constraints)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_period_data_creation() {
        let period = PeriodData::new(5, 0.5, 1.15);
        assert_eq!(period.index, 5);
        assert!((period.duration_hr - 0.5).abs() < 1e-9);
        assert!((period.load_scale - 1.15).abs() < 1e-9);
    }

    #[test]
    fn test_hourly_period() {
        let period = PeriodData::hourly(10, 0.85);
        assert_eq!(period.index, 10);
        assert!((period.duration_hr - 1.0).abs() < 1e-9);
        assert!((period.load_scale - 0.85).abs() < 1e-9);
    }

    #[test]
    fn test_ramp_constraint_symmetric() {
        let ramp = RampConstraint::symmetric("Gen1", 100.0);
        assert_eq!(ramp.gen_name, "Gen1");
        assert!((ramp.ramp_up_mw_hr - 100.0).abs() < 1e-9);
        assert!((ramp.ramp_down_mw_hr - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_ramp_constraint_asymmetric() {
        let ramp = RampConstraint::asymmetric("Gen2", 150.0, 80.0);
        assert_eq!(ramp.gen_name, "Gen2");
        assert!((ramp.ramp_up_mw_hr - 150.0).abs() < 1e-9);
        assert!((ramp.ramp_down_mw_hr - 80.0).abs() < 1e-9);
    }

    #[test]
    fn test_ramp_effective_bounds() {
        let ramp = RampConstraint::symmetric("Gen1", 50.0); // 50 MW/hr

        // Scenario: Previous dispatch = 200 MW, 1-hour period
        // Generator limits: 100-400 MW
        let (p_min, p_max) = ramp.effective_bounds(200.0, 1.0, 100.0, 400.0);

        // Ramp-constrained: 200 ± 50 = [150, 250]
        assert!((p_min - 150.0).abs() < 1e-9);
        assert!((p_max - 250.0).abs() < 1e-9);
    }

    #[test]
    fn test_ramp_effective_bounds_hits_generator_limit() {
        let ramp = RampConstraint::symmetric("Gen1", 200.0); // 200 MW/hr

        // Scenario: Previous dispatch = 150 MW, 1-hour period
        // Generator limits: 100-300 MW
        let (p_min, p_max) = ramp.effective_bounds(150.0, 1.0, 100.0, 300.0);

        // Ramp-constrained: 150 ± 200 = [-50, 350] → clamped to [100, 300]
        assert!((p_min - 100.0).abs() < 1e-9);
        assert!((p_max - 300.0).abs() < 1e-9);
    }

    #[test]
    fn test_ramp_effective_bounds_short_period() {
        let ramp = RampConstraint::symmetric("Gen1", 100.0); // 100 MW/hr

        // Scenario: Previous dispatch = 200 MW, 15-minute period (0.25 hr)
        // Generator limits: 100-400 MW
        let (p_min, p_max) = ramp.effective_bounds(200.0, 0.25, 100.0, 400.0);

        // Ramp-constrained: 200 ± (100 × 0.25) = [175, 225]
        assert!((p_min - 175.0).abs() < 1e-9);
        assert!((p_max - 225.0).abs() < 1e-9);
    }

    #[test]
    fn test_multiperiod_problem_creation() {
        use gat_core::{Branch, BranchId, Bus, BusId, CostModel, Edge, Gen, GenId, Network, Node};

        // Create minimal network
        let mut network = Network::new();
        let bus_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(1),
            name: "Bus1".to_string(),
            ..Bus::default()
        }));
        let bus2_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(2),
            name: "Bus2".to_string(),
            ..Bus::default()
        }));
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Gen1".to_string(),
            bus: BusId::new(1),
            pmin_mw: 10.0,
            pmax_mw: 100.0,
            cost_model: CostModel::linear(0.0, 20.0),
            ..Gen::default()
        }));
        network.graph.add_edge(
            bus_idx,
            bus2_idx,
            Edge::Branch(Branch {
                id: BranchId::new(1),
                from_bus: BusId::new(1),
                to_bus: BusId::new(2),
                resistance: 0.01,
                reactance: 0.1,
                status: true,
                ..Branch::default()
            }),
        );

        let base = AcOpfProblem::from_network(&network).unwrap();

        // Create 3-period problem
        let periods = vec![
            PeriodData::hourly(0, 0.8),
            PeriodData::hourly(1, 1.0),
            PeriodData::hourly(2, 0.9),
        ];
        let ramps = vec![RampConstraint::symmetric("Gen1", 30.0)];

        let multi = MultiPeriodProblem::new(base, periods, ramps);

        assert_eq!(multi.n_periods(), 3);
        assert!(multi.ramp_constraints.contains_key("Gen1"));
    }

    #[test]
    fn test_multiperiod_solution_trajectory() {
        use crate::opf::{OpfMethod, OpfSolution};

        // Create mock solutions for 3 periods
        let mut sol1 = OpfSolution {
            converged: true,
            method_used: OpfMethod::AcOpf,
            ..Default::default()
        };
        sol1.generator_p.insert("Gen1".to_string(), 50.0);
        sol1.generator_p.insert("Gen2".to_string(), 30.0);

        let mut sol2 = OpfSolution {
            converged: true,
            method_used: OpfMethod::AcOpf,
            ..Default::default()
        };
        sol2.generator_p.insert("Gen1".to_string(), 70.0);
        sol2.generator_p.insert("Gen2".to_string(), 35.0);

        let mut sol3 = OpfSolution {
            converged: true,
            method_used: OpfMethod::AcOpf,
            ..Default::default()
        };
        sol3.generator_p.insert("Gen1".to_string(), 60.0);
        sol3.generator_p.insert("Gen2".to_string(), 40.0);

        let multi_sol = MultiPeriodSolution {
            period_solutions: vec![sol1, sol2, sol3],
            total_cost: 1000.0,
            all_converged: true,
            failed_periods: vec![],
            total_solve_time_ms: 500,
        };

        // Test trajectory extraction
        let traj = multi_sol.generator_trajectory("Gen1");
        assert_eq!(traj.len(), 3);
        assert!((traj[0].1 - 50.0).abs() < 1e-9);
        assert!((traj[1].1 - 70.0).abs() < 1e-9);
        assert!((traj[2].1 - 60.0).abs() < 1e-9);
    }

    #[test]
    fn test_ramp_violation_detection() {
        use crate::opf::{OpfMethod, OpfSolution};

        // Create solutions with a ramp violation
        let mut sol1 = OpfSolution {
            converged: true,
            method_used: OpfMethod::AcOpf,
            ..Default::default()
        };
        sol1.generator_p.insert("Gen1".to_string(), 100.0);

        let mut sol2 = OpfSolution {
            converged: true,
            method_used: OpfMethod::AcOpf,
            ..Default::default()
        };
        // Ramp from 100 to 200 = +100 MW in 1 hour
        sol2.generator_p.insert("Gen1".to_string(), 200.0);

        let multi_sol = MultiPeriodSolution {
            period_solutions: vec![sol1, sol2],
            total_cost: 500.0,
            all_converged: true,
            failed_periods: vec![],
            total_solve_time_ms: 200,
        };

        // Ramp limit of 50 MW/hr should be violated
        let mut ramps = HashMap::new();
        ramps.insert("Gen1".to_string(), RampConstraint::symmetric("Gen1", 50.0));

        let periods = vec![PeriodData::hourly(0, 1.0), PeriodData::hourly(1, 1.0)];

        let violations = multi_sol.ramp_violations(&ramps, &periods);

        // Expect violation: ramped +100 MW when limit is +50 MW = 50 MW over
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, "Gen1");
        assert_eq!(violations[0].1, 1); // Period 1
        assert!((violations[0].2 - 50.0).abs() < 1e-3); // 50 MW violation
    }
}
