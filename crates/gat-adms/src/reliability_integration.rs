use anyhow::{Result, anyhow};
use gat_algo::{
    DeliverabilityScore, DeliverabilityScoreConfig, MonteCarlo, MultiAreaMonteCarlo,
    MultiAreaSystem, AreaId,
};
use gat_core::Network;

/// FLISR operation with reliability impact tracking
#[derive(Debug, Clone)]
pub struct FlisrRestoration {
    /// Operation identifier
    pub operation_id: usize,
    /// Faulted component identifier
    pub faulted_component: String,
    /// Time to detect fault (minutes)
    pub detection_time: f64,
    /// Time to isolate fault (minutes)
    pub isolation_time: f64,
    /// Time to restore service via alternate path (minutes)
    pub restoration_time: f64,
    /// Total downtime (detection + isolation + restoration)
    pub total_duration: f64,
    /// Load restored (MW)
    pub load_restored: f64,
    /// LOLE before FLISR (hours/year)
    pub lole_before: f64,
    /// LOLE after FLISR restoration (hours/year)
    pub lole_after: f64,
    /// LOLE reduction as percentage
    pub lole_reduction_pct: f64,
}

impl FlisrRestoration {
    /// Create new FLISR restoration record
    pub fn new(
        operation_id: usize,
        faulted_component: String,
        detection_time: f64,
        isolation_time: f64,
        restoration_time: f64,
        load_restored: f64,
    ) -> Self {
        let total_duration = detection_time + isolation_time + restoration_time;
        Self {
            operation_id,
            faulted_component,
            detection_time,
            isolation_time,
            restoration_time,
            total_duration,
            load_restored,
            lole_before: 0.0,
            lole_after: 0.0,
            lole_reduction_pct: 0.0,
        }
    }

    /// Set LOLE metrics before and after restoration
    pub fn set_lole_metrics(&mut self, before: f64, after: f64) {
        self.lole_before = before;
        self.lole_after = after;
        let diff = (before - after).max(0.0);
        self.lole_reduction_pct = if before > 0.0 { (diff / before) * 100.0 } else { 0.0 };
    }

    /// Effectiveness: reduction in LOLE as fraction
    pub fn effectiveness(&self) -> f64 {
        if self.lole_before > 0.0 {
            ((self.lole_before - self.lole_after) / self.lole_before).max(0.0).min(1.0)
        } else {
            0.0
        }
    }

    /// Check if FLISR was effective (restored >50% of LOLE)
    pub fn was_effective(&self) -> bool {
        self.effectiveness() > 0.5
    }
}

/// VVO configuration with reliability constraints
#[derive(Debug, Clone)]
pub struct ReliabilityAwareVvo {
    /// Minimum acceptable deliverability score (0-100)
    pub min_deliverability_score: f64,
    /// Weight for loss minimization (vs. reliability)
    pub loss_weight: f64,
    /// Weight for voltage margin maintenance
    pub voltage_weight: f64,
    /// Current operating mode: aggressive (loss focus) or conservative (reliability focus)
    pub aggressive_mode: bool,
}

impl ReliabilityAwareVvo {
    /// Create new VVO configuration
    pub fn new() -> Self {
        Self {
            min_deliverability_score: 80.0,  // Target "Good" reliability
            loss_weight: 0.6,
            voltage_weight: 0.4,
            aggressive_mode: false,
        }
    }

    /// Set minimum deliverability threshold
    pub fn with_min_score(mut self, min_score: f64) -> Self {
        self.min_deliverability_score = min_score.max(0.0).min(100.0);
        self
    }

    /// Enable aggressive mode (prioritize loss minimization)
    pub fn with_aggressive_mode(mut self, aggressive: bool) -> Self {
        self.aggressive_mode = aggressive;
        self
    }

    /// Check if VVO operation is allowed given reliability score
    pub fn check_reliability_constraint(&self, deliverability_score: &DeliverabilityScore) -> bool {
        deliverability_score.score >= self.min_deliverability_score
    }

    /// Compute objective function weighting losses vs. reliability
    pub fn compute_objective_weight(&self, current_score: f64) -> f64 {
        if current_score < self.min_deliverability_score {
            // Score is below threshold: heavily penalize loss minimization
            0.1  // Favor reliability (weight losses only 10%)
        } else if current_score < self.min_deliverability_score + 10.0 {
            // Score is near threshold: balanced approach
            0.5
        } else {
            // Score is healthy: focus on loss minimization
            if self.aggressive_mode { 0.8 } else { 0.6 }
        }
    }
}

impl Default for ReliabilityAwareVvo {
    fn default() -> Self {
        Self::new()
    }
}

/// Outage maintenance scheduling with multi-area coordination
#[derive(Debug, Clone)]
pub struct MaintenanceSchedule {
    /// Scheduled maintenance windows (area, day, duration_hours)
    pub maintenance_windows: Vec<(AreaId, u32, f64)>,
    /// System LOLE without maintenance (baseline)
    pub baseline_lole: f64,
    /// Peak LOLE during worst scheduled maintenance window
    pub peak_lole_during_maintenance: f64,
    /// Annual EUE reduction from optimized scheduling
    pub eue_reduction_pct: f64,
}

impl MaintenanceSchedule {
    /// Create new maintenance schedule
    pub fn new(baseline_lole: f64) -> Self {
        Self {
            maintenance_windows: Vec::new(),
            baseline_lole,
            peak_lole_during_maintenance: baseline_lole,
            eue_reduction_pct: 0.0,
        }
    }

    /// Add maintenance window
    pub fn add_window(&mut self, area: AreaId, day: u32, duration_hours: f64) -> Result<()> {
        if day > 365 {
            return Err(anyhow!("Day must be <= 365"));
        }
        if duration_hours <= 0.0 || duration_hours > 24.0 {
            return Err(anyhow!("Duration must be > 0 and <= 24 hours"));
        }
        self.maintenance_windows.push((area, day, duration_hours));
        Ok(())
    }

    /// Check if schedule maintains constraints (no two neighbors on same day)
    pub fn validate_multiarea_coordination(&self, system: &MultiAreaSystem) -> Result<()> {
        // Get day -> areas mapping
        let mut days_by_area: std::collections::HashMap<u32, Vec<AreaId>> =
            std::collections::HashMap::new();

        for (area, day, _) in &self.maintenance_windows {
            days_by_area.entry(*day).or_insert_with(Vec::new).push(*area);
        }

        // Check if multiple areas on same day
        for (day, areas) in days_by_area.iter() {
            if areas.len() > 1 {
                // Multiple areas on same day - check if they're neighbors
                for i in 0..areas.len() {
                    for j in (i + 1)..areas.len() {
                        let area_a = areas[i];
                        let area_b = areas[j];

                        // Check if areas are connected via corridor
                        let is_neighbor = system.corridors.iter().any(|c| {
                            (c.area_a == area_a && c.area_b == area_b)
                                || (c.area_a == area_b && c.area_b == area_a)
                        });

                        if is_neighbor {
                            return Err(anyhow!(
                                "Multi-area coordination violation: areas {:?} and {:?} both scheduled for day {}",
                                area_a, area_b, day
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Estimate maximum LOLE during maintenance period
    pub fn estimate_peak_lole(&mut self, system: &MultiAreaSystem) -> Result<()> {
        let mc = MultiAreaMonteCarlo::new(500);
        let baseline_metrics = mc.compute_multiarea_reliability(system)?;

        // Calculate baseline LOLE
        let baseline_total_lole: f64 = baseline_metrics.area_lole.values().sum();
        self.baseline_lole = baseline_total_lole;

        // Simple heuristic: peak LOLE = baseline + (avg area LOLE for each maintenance window)
        let num_areas = system.num_areas() as f64;
        let avg_area_lole = if num_areas > 0.0 {
            baseline_total_lole / num_areas
        } else {
            0.0
        };

        // For each maintenance window, add impact
        let mut peak_lole = baseline_total_lole;
        for (_area, _day, _hours) in &self.maintenance_windows {
            // Simple linear approximation: maintenance adds ~20% to area LOLE
            peak_lole += avg_area_lole * 0.2;
        }

        self.peak_lole_during_maintenance = peak_lole;

        // EUE reduction: well-scheduled maintenance reduces congestion-related outages
        // Assume 5% EUE reduction per well-coordinated window
        let reduction_per_window = 5.0 * self.maintenance_windows.len() as f64;
        self.eue_reduction_pct = reduction_per_window.min(15.0);  // Cap at 15%

        Ok(())
    }

    /// Check if peak LOLE stays within acceptable threshold
    pub fn meets_reliability_threshold(&self, threshold_lole: f64) -> bool {
        self.peak_lole_during_maintenance <= threshold_lole
    }
}

/// ADMS workflow orchestration with reliability integration
pub struct ReliabilityOrchestrator {
    /// Deliverability score config
    pub deliverability_config: DeliverabilityScoreConfig,
    /// VVO reliability-aware settings
    pub vvo_config: ReliabilityAwareVvo,
    /// FLISR restoration history
    pub flisr_operations: Vec<FlisrRestoration>,
    /// Maintenance schedules
    pub maintenance_schedule: Option<MaintenanceSchedule>,
}

impl ReliabilityOrchestrator {
    /// Create new orchestrator
    pub fn new() -> Self {
        Self {
            deliverability_config: DeliverabilityScoreConfig::new(),
            vvo_config: ReliabilityAwareVvo::new(),
            flisr_operations: Vec::new(),
            maintenance_schedule: None,
        }
    }

    /// Evaluate system reliability with current state
    pub fn evaluate_reliability(&self, network: &Network) -> Result<DeliverabilityScore> {
        let mc = MonteCarlo::new(500);
        let metrics = mc.compute_reliability(network)?;
        DeliverabilityScore::from_metrics(metrics, &self.deliverability_config)
    }

    /// Execute FLISR operation and track reliability impact
    pub fn execute_flisr_operation(
        &mut self,
        network_pre_fault: &Network,
        network_post_restoration: &Network,
        faulted_component: String,
        load_restored: f64,
    ) -> Result<FlisrRestoration> {
        // Compute reliability metrics before and after
        let mc = MonteCarlo::new(300);
        let metrics_pre = mc.compute_reliability(network_pre_fault)?;
        let metrics_post = mc.compute_reliability(network_post_restoration)?;

        let op_id = self.flisr_operations.len();
        let mut operation = FlisrRestoration::new(op_id, faulted_component, 2.0, 5.0, 8.0, load_restored);
        operation.set_lole_metrics(metrics_pre.lole, metrics_post.lole);

        self.flisr_operations.push(operation.clone());
        Ok(operation)
    }

    /// Evaluate if VVO operation respects reliability constraints
    pub fn check_vvo_reliability(&self, network: &Network) -> Result<bool> {
        let score = self.evaluate_reliability(network)?;
        Ok(self.vvo_config.check_reliability_constraint(&score))
    }

    /// Plan maintenance with multi-area coordination
    pub fn plan_maintenance(&mut self, system: &MultiAreaSystem) -> Result<MaintenanceSchedule> {
        let mc = MultiAreaMonteCarlo::new(500);
        let metrics = mc.compute_multiarea_reliability(system)?;

        let baseline_lole: f64 = metrics.area_lole.values().sum();
        let mut schedule = MaintenanceSchedule::new(baseline_lole);

        // Simple greedy scheduling: pick days with lowest baseline LOLE
        let mut scheduled_areas = std::collections::HashSet::new();
        for (area_id, _) in metrics.area_lole.iter() {
            if !scheduled_areas.contains(area_id) {
                // Schedule maintenance on low-demand day (day 150 = May 30, typically lower)
                schedule.add_window(*area_id, 150, 8.0)?;
                scheduled_areas.insert(*area_id);
            }
        }

        // Validate multi-area coordination
        schedule.validate_multiarea_coordination(system)?;

        // Estimate peak LOLE during maintenance
        schedule.estimate_peak_lole(system)?;

        self.maintenance_schedule = Some(schedule.clone());
        Ok(schedule)
    }

    /// Get FLISR effectiveness statistics
    pub fn flisr_effectiveness_stats(&self) -> (f64, usize) {
        if self.flisr_operations.is_empty() {
            return (0.0, 0);
        }
        let total_effectiveness: f64 = self.flisr_operations.iter().map(|op| op.effectiveness()).sum();
        let avg_effectiveness = total_effectiveness / self.flisr_operations.len() as f64;
        let effective_ops = self.flisr_operations.iter().filter(|op| op.was_effective()).count();
        (avg_effectiveness, effective_ops)
    }
}

impl Default for ReliabilityOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}
