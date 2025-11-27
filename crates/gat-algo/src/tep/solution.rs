//! TEP solution data structures
//!
//! Defines the output from solving TEP problems.

use super::CandidateId;
use std::collections::HashMap;
use std::time::Duration;

/// Decision to build a candidate line
#[derive(Debug, Clone)]
pub struct LineBuildDecision {
    /// Candidate line ID
    pub candidate_id: CandidateId,
    /// Candidate name
    pub name: String,
    /// Number of circuits to build (0 = don't build)
    pub circuits_to_build: usize,
    /// Investment cost for this decision
    pub investment_cost: f64,
}

impl LineBuildDecision {
    /// Check if this line should be built
    pub fn is_built(&self) -> bool {
        self.circuits_to_build > 0
    }
}

/// Complete solution to a TEP problem
#[derive(Debug, Clone)]
pub struct TepSolution {
    /// Whether the solver found an optimal solution
    pub optimal: bool,
    /// Total objective value (investment + operating cost)
    pub total_cost: f64,
    /// Investment cost component
    pub investment_cost: f64,
    /// Operating cost component
    pub operating_cost: f64,
    /// Build decisions for each candidate
    pub build_decisions: Vec<LineBuildDecision>,
    /// Generator dispatch (name -> MW)
    pub generator_dispatch: HashMap<String, f64>,
    /// Bus voltage angles (name -> radians)
    pub bus_angles: HashMap<String, f64>,
    /// Branch flows for existing lines (name -> MW)
    pub existing_branch_flows: HashMap<String, f64>,
    /// Branch flows for built candidate lines (candidate_id -> MW)
    pub candidate_flows: HashMap<CandidateId, f64>,
    /// Number of iterations (MILP solver)
    pub iterations: usize,
    /// Solve time
    pub solve_time: Duration,
    /// MIP gap (relative optimality gap, if applicable)
    pub mip_gap: Option<f64>,
    /// Solver status message
    pub status_message: String,
}

impl TepSolution {
    /// Create a new empty solution
    pub fn new() -> Self {
        Self {
            optimal: false,
            total_cost: 0.0,
            investment_cost: 0.0,
            operating_cost: 0.0,
            build_decisions: Vec::new(),
            generator_dispatch: HashMap::new(),
            bus_angles: HashMap::new(),
            existing_branch_flows: HashMap::new(),
            candidate_flows: HashMap::new(),
            iterations: 0,
            solve_time: Duration::ZERO,
            mip_gap: None,
            status_message: String::new(),
        }
    }

    /// Get number of lines built
    pub fn lines_built(&self) -> usize {
        self.build_decisions.iter().filter(|d| d.is_built()).count()
    }

    /// Get total circuits built (accounting for parallel circuits)
    pub fn total_circuits_built(&self) -> usize {
        self.build_decisions
            .iter()
            .map(|d| d.circuits_to_build)
            .sum()
    }

    /// Get the names of built lines
    pub fn built_line_names(&self) -> Vec<&str> {
        self.build_decisions
            .iter()
            .filter(|d| d.is_built())
            .map(|d| d.name.as_str())
            .collect()
    }

    /// Get total generation dispatched
    pub fn total_generation_mw(&self) -> f64 {
        self.generator_dispatch.values().sum()
    }

    /// Format a human-readable summary
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("TEP Solution Summary\n{}\n", "=".repeat(40)));
        s.push_str(&format!(
            "Status: {}\n",
            if self.optimal {
                "Optimal"
            } else {
                "Suboptimal/Infeasible"
            }
        ));
        s.push_str(&format!("Total Cost: ${:.2}\n", self.total_cost));
        s.push_str(&format!("  Investment: ${:.2}\n", self.investment_cost));
        s.push_str(&format!("  Operating: ${:.2}\n", self.operating_cost));
        s.push_str(&format!(
            "Lines Built: {} ({} circuits)\n",
            self.lines_built(),
            self.total_circuits_built()
        ));
        s.push_str(&format!(
            "Total Generation: {:.2} MW\n",
            self.total_generation_mw()
        ));
        s.push_str(&format!("Solve Time: {:.2?}\n", self.solve_time));
        if let Some(gap) = self.mip_gap {
            s.push_str(&format!("MIP Gap: {:.4}%\n", gap * 100.0));
        }

        if !self.build_decisions.is_empty() {
            s.push_str("\nBuild Decisions:\n");
            for decision in &self.build_decisions {
                if decision.is_built() {
                    s.push_str(&format!(
                        "  [BUILD] {} (x{}) - ${:.2}\n",
                        decision.name, decision.circuits_to_build, decision.investment_cost
                    ));
                } else {
                    s.push_str(&format!("  [SKIP]  {}\n", decision.name));
                }
            }
        }

        s
    }
}

impl Default for TepSolution {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solution_summary() {
        let mut solution = TepSolution::new();
        solution.optimal = true;
        solution.total_cost = 5_000_000.0;
        solution.investment_cost = 3_000_000.0;
        solution.operating_cost = 2_000_000.0;

        solution.build_decisions.push(LineBuildDecision {
            candidate_id: CandidateId::new(1),
            name: "Line 1-2".to_string(),
            circuits_to_build: 1,
            investment_cost: 1_500_000.0,
        });

        solution.build_decisions.push(LineBuildDecision {
            candidate_id: CandidateId::new(2),
            name: "Line 2-3".to_string(),
            circuits_to_build: 2,
            investment_cost: 1_500_000.0,
        });

        solution.build_decisions.push(LineBuildDecision {
            candidate_id: CandidateId::new(3),
            name: "Line 1-3".to_string(),
            circuits_to_build: 0,
            investment_cost: 0.0,
        });

        assert_eq!(solution.lines_built(), 2);
        assert_eq!(solution.total_circuits_built(), 3);

        let summary = solution.summary();
        assert!(summary.contains("Lines Built: 2"));
        assert!(summary.contains("[BUILD] Line 1-2"));
        assert!(summary.contains("[SKIP]  Line 1-3"));
    }

    #[test]
    fn test_built_line_names() {
        let mut solution = TepSolution::new();

        solution.build_decisions.push(LineBuildDecision {
            candidate_id: CandidateId::new(1),
            name: "Alpha".to_string(),
            circuits_to_build: 1,
            investment_cost: 1e6,
        });

        solution.build_decisions.push(LineBuildDecision {
            candidate_id: CandidateId::new(2),
            name: "Beta".to_string(),
            circuits_to_build: 0,
            investment_cost: 0.0,
        });

        solution.build_decisions.push(LineBuildDecision {
            candidate_id: CandidateId::new(3),
            name: "Gamma".to_string(),
            circuits_to_build: 2,
            investment_cost: 2e6,
        });

        let names = solution.built_line_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Alpha"));
        assert!(names.contains(&"Gamma"));
        assert!(!names.contains(&"Beta"));
    }
}
