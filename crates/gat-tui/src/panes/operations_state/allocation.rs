//! Allocation analysis state management
//!
//! Handles allocation results, metrics calculations, and navigation.

use crate::components::ListWidget;

use super::types::AllocationResult;

/// State for allocation analysis operations
#[derive(Clone, Debug)]
pub struct AllocationState {
    /// Allocation results by node
    results: Vec<AllocationResult>,
    /// Currently selected result index
    selected: usize,
    /// List widget for UI rendering
    pub allocation_list: ListWidget,
}

impl Default for AllocationState {
    fn default() -> Self {
        Self::new()
    }
}

impl AllocationState {
    /// Create a new AllocationState with default sample data
    pub fn new() -> Self {
        let results = vec![
            AllocationResult {
                node_id: "NODE_A".into(),
                rents: 1250.5,
                contribution: 45.2,
                allocation_factor: 0.85,
                revenue_adequacy: 95.3,
                cost_recovery: 98.2,
                surplus_deficit: 152.40,
            },
            AllocationResult {
                node_id: "NODE_B".into(),
                rents: 890.3,
                contribution: 32.1,
                allocation_factor: 0.72,
                revenue_adequacy: 87.6,
                cost_recovery: 91.5,
                surplus_deficit: 65.20,
            },
            AllocationResult {
                node_id: "NODE_C".into(),
                rents: 675.8,
                contribution: 28.5,
                allocation_factor: 0.68,
                revenue_adequacy: 82.1,
                cost_recovery: 85.3,
                surplus_deficit: -45.30,
            },
        ];

        let mut allocation_list = ListWidget::new("operations_allocation");
        for result in &results {
            allocation_list.add_item(result.display_line(), result.node_id.clone());
        }

        Self {
            results,
            selected: 0,
            allocation_list,
        }
    }

    /// Create an empty AllocationState
    pub fn empty() -> Self {
        Self {
            results: Vec::new(),
            selected: 0,
            allocation_list: ListWidget::new("operations_allocation"),
        }
    }

    /// Get all allocation results
    pub fn results(&self) -> &[AllocationResult] {
        &self.results
    }

    /// Get the number of allocation results
    pub fn count(&self) -> usize {
        self.results.len()
    }

    /// Select the next allocation result
    pub fn select_next(&mut self) {
        if self.selected < self.results.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Select the previous allocation result
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Get the currently selected allocation result
    pub fn selected(&self) -> Option<&AllocationResult> {
        self.results.get(self.selected)
    }

    /// Get the selected index
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Add a new allocation result to the front of the list
    pub fn add(&mut self, result: AllocationResult) {
        self.allocation_list
            .add_item(result.display_line(), result.node_id.clone());
        self.results.insert(0, result);
    }

    /// Get formatted details for the selected allocation
    pub fn get_details(&self) -> String {
        self.selected()
            .map(|r| r.details())
            .unwrap_or_else(|| "No allocation results selected".into())
    }

    // ============================================================================
    // Aggregate metrics
    // ============================================================================

    /// Calculate total rents across all nodes
    pub fn total_rents(&self) -> f64 {
        self.results.iter().map(|r| r.rents).sum()
    }

    /// Calculate total contributions across all nodes
    pub fn total_contributions(&self) -> f64 {
        self.results.iter().map(|r| r.contribution).sum()
    }

    /// Calculate average allocation factor
    pub fn avg_allocation_factor(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.results.iter().map(|r| r.allocation_factor).sum();
        sum / self.results.len() as f64
    }

    /// Calculate average revenue adequacy percentage
    pub fn avg_revenue_adequacy(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.results.iter().map(|r| r.revenue_adequacy).sum();
        sum / self.results.len() as f64
    }

    /// Calculate average cost recovery percentage
    pub fn avg_cost_recovery(&self) -> f64 {
        if self.results.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.results.iter().map(|r| r.cost_recovery).sum();
        sum / self.results.len() as f64
    }

    /// Calculate total surplus/deficit across all nodes
    pub fn total_surplus_deficit(&self) -> f64 {
        self.results.iter().map(|r| r.surplus_deficit).sum()
    }

    /// Get a formatted summary of all allocation metrics
    pub fn get_summary(&self) -> String {
        format!(
            "Total Rents: ${:.2}\nTotal Contributions: ${:.2}\nAvg Allocation Factor: {:.2}\nAvg Revenue Adequacy: {:.1}%\nAvg Cost Recovery: {:.1}%\nTotal Surplus/Deficit: ${:.2}",
            self.total_rents(),
            self.total_contributions(),
            self.avg_allocation_factor(),
            self.avg_revenue_adequacy(),
            self.avg_cost_recovery(),
            self.total_surplus_deficit(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_state_init() {
        let state = AllocationState::new();
        assert_eq!(state.count(), 3);
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_allocation_state_empty() {
        let state = AllocationState::empty();
        assert_eq!(state.count(), 0);
        assert!(state.selected().is_none());
    }

    #[test]
    fn test_allocation_selection() {
        let mut state = AllocationState::new();
        assert_eq!(state.selected_index(), 0);

        state.select_next();
        assert_eq!(state.selected_index(), 1);

        state.select_next();
        assert_eq!(state.selected_index(), 2);

        state.select_next(); // Should not exceed bounds
        assert_eq!(state.selected_index(), 2);

        state.select_prev();
        assert_eq!(state.selected_index(), 1);
    }

    #[test]
    fn test_selected_allocation() {
        let state = AllocationState::new();
        let result = state.selected().unwrap();
        assert_eq!(result.node_id, "NODE_A");
        assert_eq!(result.rents, 1250.5);
    }

    #[test]
    fn test_allocation_details() {
        let state = AllocationState::new();
        let details = state.get_details();
        assert!(details.contains("NODE_A"));
        assert!(details.contains("1250.50"));
        assert!(details.contains("95.3"));
    }

    #[test]
    fn test_add_allocation() {
        let mut state = AllocationState::new();
        let initial_count = state.count();

        let new_result = AllocationResult {
            node_id: "NODE_D".into(),
            rents: 500.0,
            contribution: 20.0,
            allocation_factor: 0.65,
            revenue_adequacy: 80.0,
            cost_recovery: 85.0,
            surplus_deficit: 25.0,
        };

        state.add(new_result);
        assert_eq!(state.count(), initial_count + 1);
        // New result should be at the front
        assert_eq!(state.results()[0].node_id, "NODE_D");
    }

    #[test]
    fn test_total_rents() {
        let state = AllocationState::new();
        let total = state.total_rents();
        // 1250.5 + 890.3 + 675.8 = 2816.6
        assert!(total > 2800.0);
        assert!(total < 2820.0);
    }

    #[test]
    fn test_total_contributions() {
        let state = AllocationState::new();
        let total = state.total_contributions();
        // 45.2 + 32.1 + 28.5 = 105.8
        assert!(total > 105.0);
        assert!(total < 110.0);
    }

    #[test]
    fn test_avg_allocation_factor() {
        let state = AllocationState::new();
        let avg = state.avg_allocation_factor();
        // (0.85 + 0.72 + 0.68) / 3 = 0.75
        assert!(avg > 0.70);
        assert!(avg < 0.80);
    }

    #[test]
    fn test_avg_revenue_adequacy() {
        let state = AllocationState::new();
        let avg = state.avg_revenue_adequacy();
        // (95.3 + 87.6 + 82.1) / 3 = 88.33
        assert!(avg > 85.0);
        assert!(avg < 92.0);
    }

    #[test]
    fn test_avg_cost_recovery() {
        let state = AllocationState::new();
        let avg = state.avg_cost_recovery();
        // (98.2 + 91.5 + 85.3) / 3 = 91.67
        assert!(avg > 88.0);
        assert!(avg < 95.0);
    }

    #[test]
    fn test_total_surplus_deficit() {
        let state = AllocationState::new();
        let total = state.total_surplus_deficit();
        // 152.40 + 65.20 - 45.30 = 172.30
        assert!(total > 170.0);
        assert!(total < 175.0);
    }

    #[test]
    fn test_allocation_summary() {
        let state = AllocationState::new();
        let summary = state.get_summary();
        assert!(summary.contains("Total Rents:"));
        assert!(summary.contains("Total Contributions:"));
        assert!(summary.contains("Avg Allocation Factor:"));
        assert!(summary.contains("Avg Revenue Adequacy:"));
        assert!(summary.contains("Avg Cost Recovery:"));
        assert!(summary.contains("Total Surplus/Deficit:"));
    }

    #[test]
    fn test_empty_metrics() {
        let state = AllocationState::empty();
        assert_eq!(state.avg_allocation_factor(), 0.0);
        assert_eq!(state.avg_revenue_adequacy(), 0.0);
        assert_eq!(state.avg_cost_recovery(), 0.0);
        assert_eq!(state.total_rents(), 0.0);
    }
}
