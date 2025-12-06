use crate::{arena::ArenaContext, OutageScenario, ReliabilityMetrics};
use anyhow::{anyhow, Result};
use gat_core::{Network, Node, NodeIndex};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

/// Area identifier for multi-area systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AreaId(pub usize);

/// Inter-area transmission corridor
#[derive(Debug, Clone)]
pub struct Corridor {
    /// Corridor identifier
    pub id: usize,
    /// Area A (export area)
    pub area_a: AreaId,
    /// Area B (import area)
    pub area_b: AreaId,
    /// Maximum power transfer from A to B (MW)
    pub capacity_mw: f64,
    /// Failure rate (probability per year)
    pub failure_rate: f64,
}

impl Corridor {
    /// Create new corridor
    pub fn new(id: usize, area_a: AreaId, area_b: AreaId, capacity_mw: f64) -> Self {
        Self {
            id,
            area_a,
            area_b,
            capacity_mw,
            failure_rate: 0.01, // 1% failure rate by default
        }
    }

    /// Set failure rate
    pub fn with_failure_rate(mut self, rate: f64) -> Self {
        self.failure_rate = rate;
        self
    }

    /// Check if corridor is online in scenario
    pub fn is_online(&self, offline_corridors: &HashSet<usize>) -> bool {
        !offline_corridors.contains(&self.id)
    }
}

/// Multi-area system configuration
#[derive(Debug)]
pub struct MultiAreaSystem {
    /// Area networks (keyed by AreaId)
    pub areas: HashMap<AreaId, Network>,
    /// Inter-area corridors
    pub corridors: Vec<Corridor>,
    /// Map from area to its node indices within the network
    pub area_node_map: HashMap<AreaId, Vec<NodeIndex>>,
}

impl MultiAreaSystem {
    /// Create new multi-area system
    pub fn new() -> Self {
        Self {
            areas: HashMap::new(),
            corridors: Vec::new(),
            area_node_map: HashMap::new(),
        }
    }

    /// Add area to system
    pub fn add_area(&mut self, area_id: AreaId, network: Network) -> Result<()> {
        if self.areas.contains_key(&area_id) {
            return Err(anyhow!("Area {:?} already exists", area_id));
        }
        self.areas.insert(area_id, network);
        self.area_node_map.insert(area_id, Vec::new());
        Ok(())
    }

    /// Add corridor between areas
    pub fn add_corridor(&mut self, corridor: Corridor) -> Result<()> {
        if !self.areas.contains_key(&corridor.area_a) {
            return Err(anyhow!("Area {:?} does not exist", corridor.area_a));
        }
        if !self.areas.contains_key(&corridor.area_b) {
            return Err(anyhow!("Area {:?} does not exist", corridor.area_b));
        }
        self.corridors.push(corridor);
        Ok(())
    }

    /// Get number of areas
    pub fn num_areas(&self) -> usize {
        self.areas.len()
    }

    /// Get number of corridors
    pub fn num_corridors(&self) -> usize {
        self.corridors.len()
    }

    /// Validate system has at least 2 areas
    pub fn validate(&self) -> Result<()> {
        if self.areas.len() < 2 {
            return Err(anyhow!("Multi-area system must have at least 2 areas"));
        }
        Ok(())
    }
}

impl Default for MultiAreaSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-area outage scenario with corridor failures
#[derive(Debug, Clone)]
pub struct MultiAreaOutageScenario {
    /// Area outage scenarios (per-area)
    pub area_scenarios: HashMap<AreaId, OutageScenario>,
    /// Offline corridors
    pub offline_corridors: HashSet<usize>,
    /// Scenario probability
    pub probability: f64,
}

impl MultiAreaOutageScenario {
    /// Create new multi-area scenario
    pub fn new(probability: f64) -> Self {
        Self {
            area_scenarios: HashMap::new(),
            offline_corridors: HashSet::new(),
            probability,
        }
    }

    /// Set area scenario
    pub fn set_area(&mut self, area_id: AreaId, scenario: OutageScenario) {
        self.area_scenarios.insert(area_id, scenario);
    }

    /// Mark corridor as offline
    pub fn mark_corridor_offline(&mut self, corridor_id: usize) {
        self.offline_corridors.insert(corridor_id);
    }

    /// Check if scenario is feasible without inter-area support
    pub fn is_feasible_standalone(&self, system: &MultiAreaSystem) -> bool {
        for (area_id, scenario) in &self.area_scenarios {
            if let Some(network) = system.areas.get(area_id) {
                // Calculate area demand and supply
                let mut area_demand = 0.0;
                let mut area_supply = 0.0;

                for node_idx in network.graph.node_indices() {
                    match network.graph.node_weight(node_idx) {
                        Some(Node::Load(load)) => area_demand += load.active_power.value(),
                        Some(Node::Gen(gen)) => {
                            if !scenario.offline_generators.contains(&node_idx) {
                                area_supply += gen.active_power.value();
                            }
                        }
                        _ => {}
                    }
                }

                // Check if area can supply its own demand
                let required_demand = area_demand * scenario.demand_scale;
                if area_supply < required_demand {
                    return false;
                }
            }
        }
        true
    }
}

/// Multi-area zone-to-zone LOLE metrics
#[derive(Debug, Clone)]
pub struct AreaLoleMetrics {
    /// LOLE for each area (hours/year)
    pub area_lole: HashMap<AreaId, f64>,
    /// EUE for each area (MWh/year)
    pub area_eue: HashMap<AreaId, f64>,
    /// Zone-to-zone LOLE (area perspective)
    pub zone_to_zone_lole: HashMap<AreaId, f64>,
    /// Total scenarios analyzed
    pub scenarios_analyzed: usize,
    /// Number of scenarios with any shortfall
    pub scenarios_with_shortfall: usize,
    /// Corridor utilization (percentage of capacity)
    pub corridor_utilization: HashMap<usize, f64>,
}

impl AreaLoleMetrics {
    /// Create new area LOLE metrics
    pub fn new(num_scenarios: usize) -> Self {
        Self {
            area_lole: HashMap::new(),
            area_eue: HashMap::new(),
            zone_to_zone_lole: HashMap::new(),
            scenarios_analyzed: num_scenarios,
            scenarios_with_shortfall: 0,
            corridor_utilization: HashMap::new(),
        }
    }
}

/// Per-scenario result for parallel aggregation
#[derive(Debug, Clone, Default)]
struct ScenarioResult {
    /// Whether this scenario has any shortfall
    has_shortfall: bool,
    /// Per-area shortfall contributions: (LOLE contribution, EUE contribution)
    area_shortfalls: HashMap<AreaId, (f64, f64)>,
    /// Per-corridor utilization for this scenario
    corridor_utils: HashMap<usize, f64>,
}

impl ScenarioResult {
    /// Create new empty scenario result
    fn new() -> Self {
        Self::default()
    }

    /// Add area shortfall contribution
    fn add_area_shortfall(&mut self, area_id: AreaId, lole: f64, eue: f64) {
        self.has_shortfall = true;
        self.area_shortfalls.insert(area_id, (lole, eue));
    }

    /// Add corridor utilization
    fn add_corridor_utilization(&mut self, corridor_id: usize, util: f64) {
        self.corridor_utils.insert(corridor_id, util);
    }
}

/// Multi-area Monte Carlo analyzer with zone-to-zone reliability
pub struct MultiAreaMonteCarlo {
    /// Number of scenarios to run
    pub num_scenarios: usize,
    /// Hours per year for LOLE calculation
    pub hours_per_year: f64,
}

impl MultiAreaMonteCarlo {
    /// Create new multi-area Monte Carlo analyzer
    pub fn new(num_scenarios: usize) -> Self {
        Self {
            num_scenarios,
            hours_per_year: 365.25 * 24.0,
        }
    }

    /// Generate multi-area outage scenarios
    pub fn generate_multiarea_scenarios(
        &self,
        system: &MultiAreaSystem,
        seed: u64,
    ) -> Result<Vec<MultiAreaOutageScenario>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut scenarios = Vec::with_capacity(self.num_scenarios);
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let mut rng_state = hasher.finish();

        for _ in 0..self.num_scenarios {
            // Generate scenarios for each area
            let mut scenario = MultiAreaOutageScenario::new(1.0 / self.num_scenarios as f64);

            for (area_id, network) in &system.areas {
                // Collect generator and branch indices for this area
                let gen_nodes: Vec<NodeIndex> = network
                    .graph
                    .node_indices()
                    .filter(|&idx| matches!(network.graph.node_weight(idx), Some(Node::Gen(_))))
                    .collect();

                let branch_count = network.graph.edge_count();

                // Generate outages for this area
                let mut offline_generators = HashSet::new();
                for &gen_idx in &gen_nodes {
                    rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                    let r = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;
                    if r < 0.05 {
                        // 5% generator failure rate
                        offline_generators.insert(gen_idx);
                    }
                }

                let mut offline_branches = HashSet::new();
                for idx in 0..branch_count {
                    rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                    let r = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;
                    if r < 0.02 {
                        // 2% branch failure rate
                        offline_branches.insert(idx);
                    }
                }

                // Demand scaling (0.8 to 1.2)
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let demand_r = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;
                let demand_scale = 0.8 + (1.2 - 0.8) * demand_r;

                let area_scenario = OutageScenario {
                    offline_generators,
                    offline_branches,
                    demand_scale,
                    probability: 1.0 / self.num_scenarios as f64,
                };

                scenario.set_area(*area_id, area_scenario);
            }

            // Generate corridor failures
            for corridor in &system.corridors {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let r = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;
                if r < corridor.failure_rate {
                    scenario.mark_corridor_offline(corridor.id);
                }
            }

            scenarios.push(scenario);
        }

        Ok(scenarios)
    }

    /// Compute multi-area zone-to-zone LOLE
    pub fn compute_multiarea_reliability(
        &self,
        system: &MultiAreaSystem,
    ) -> Result<AreaLoleMetrics> {
        system.validate()?;

        // Generate scenarios
        let scenarios = self.generate_multiarea_scenarios(system, 42)?;

        let mut metrics = AreaLoleMetrics::new(self.num_scenarios);

        // Initialize area metrics
        for area_id in system.areas.keys() {
            metrics.area_lole.insert(*area_id, 0.0);
            metrics.area_eue.insert(*area_id, 0.0);
            metrics.zone_to_zone_lole.insert(*area_id, 0.0);
        }

        // Initialize corridor utilization tracking
        for corridor in &system.corridors {
            metrics.corridor_utilization.insert(corridor.id, 0.0);
        }

        // Analyze each scenario
        for scenario in &scenarios {
            let mut any_shortfall = false;

            // Per-area analysis
            for (area_id, network) in &system.areas {
                if let Some(area_scenario) = scenario.area_scenarios.get(area_id) {
                    // Calculate area demand and supply
                    let mut area_demand = 0.0;
                    let mut area_supply = 0.0;

                    for node_idx in network.graph.node_indices() {
                        match network.graph.node_weight(node_idx) {
                            Some(Node::Load(load)) => area_demand += load.active_power.value(),
                            Some(Node::Gen(gen)) => {
                                if !area_scenario.offline_generators.contains(&node_idx) {
                                    area_supply += gen.active_power.value();
                                }
                            }
                            _ => {}
                        }
                    }

                    let required_demand = area_demand * area_scenario.demand_scale;

                    // Calculate available inter-area support (through online corridors)
                    let mut available_import = 0.0;
                    for corridor in &system.corridors {
                        if (corridor.area_a == *area_id || corridor.area_b == *area_id)
                            && corridor.is_online(&scenario.offline_corridors)
                        {
                            // Assume 50% of corridor capacity is available for import
                            available_import += corridor.capacity_mw * 0.5;
                        }
                    }

                    let total_available = area_supply + available_import;

                    // Check for shortfall
                    if total_available < required_demand {
                        let shortfall = required_demand - total_available;
                        let lole_contribution = area_scenario.probability;
                        let eue_contribution = shortfall * area_scenario.probability;

                        // Update area LOLE and EUE
                        *metrics.area_lole.get_mut(area_id).unwrap() += lole_contribution;
                        *metrics.area_eue.get_mut(area_id).unwrap() += eue_contribution;
                        *metrics.zone_to_zone_lole.get_mut(area_id).unwrap() += lole_contribution;

                        any_shortfall = true;
                    }
                }
            }

            if any_shortfall {
                metrics.scenarios_with_shortfall += 1;
            }

            // Update corridor utilization (assumed average 60% in normal scenarios)
            for corridor in &system.corridors {
                if corridor.is_online(&scenario.offline_corridors) {
                    *metrics
                        .corridor_utilization
                        .entry(corridor.id)
                        .or_insert(0.0) += 60.0;
                }
            }
        }

        // Convert LOLE from probability to hours/year
        for lole in metrics.area_lole.values_mut() {
            *lole *= self.hours_per_year;
        }

        // Convert EUE from probability-weighted MWh to annual MWh
        for eue in metrics.area_eue.values_mut() {
            *eue *= self.hours_per_year;
        }

        // Convert zone-to-zone LOLE similarly
        for z2z_lole in metrics.zone_to_zone_lole.values_mut() {
            *z2z_lole *= self.hours_per_year;
        }

        // Average corridor utilization across scenarios
        for util in metrics.corridor_utilization.values_mut() {
            *util /= self.num_scenarios as f64;
        }

        Ok(metrics)
    }

    /// Compute multi-area zone-to-zone LOLE using parallel arena-based evaluation
    ///
    /// This version uses `rayon::par_iter().map_init()` with arena allocation
    /// to reduce allocator pressure in the scenario evaluation hot loop.
    pub fn compute_multiarea_reliability_parallel(
        &self,
        system: &MultiAreaSystem,
    ) -> Result<AreaLoleMetrics> {
        system.validate()?;

        // Generate scenarios (this is serial but cheap)
        let scenarios = self.generate_multiarea_scenarios(system, 42)?;

        // Parallel scenario evaluation with arena allocation
        let scenario_results: Vec<ScenarioResult> = scenarios
            .par_iter()
            .map_init(ArenaContext::new, |ctx, scenario| {
                let result = self.evaluate_scenario_with_arena(system, scenario, ctx);
                ctx.reset(); // O(1) bulk deallocation
                result
            })
            .collect();

        // Reduce results into metrics
        self.reduce_scenario_results(system, scenario_results)
    }

    /// Evaluate a single multi-area scenario using arena-backed collections
    fn evaluate_scenario_with_arena(
        &self,
        system: &MultiAreaSystem,
        scenario: &MultiAreaOutageScenario,
        ctx: &ArenaContext,
    ) -> ScenarioResult {
        let mut result = ScenarioResult::new();

        // Use arena-backed HashSet for corridor online checks
        let mut offline_corridors = ctx.alloc_hashset::<usize>();
        for &id in &scenario.offline_corridors {
            offline_corridors.insert(id);
        }

        // Per-area analysis
        for (area_id, network) in &system.areas {
            if let Some(area_scenario) = scenario.area_scenarios.get(area_id) {
                // Calculate area demand and supply
                let mut area_demand = 0.0;
                let mut area_supply = 0.0;

                for node_idx in network.graph.node_indices() {
                    match network.graph.node_weight(node_idx) {
                        Some(Node::Load(load)) => area_demand += load.active_power.value(),
                        Some(Node::Gen(gen)) => {
                            if !area_scenario.offline_generators.contains(&node_idx) {
                                area_supply += gen.active_power.value();
                            }
                        }
                        _ => {}
                    }
                }

                let required_demand = area_demand * area_scenario.demand_scale;

                // Calculate available inter-area support (through online corridors)
                let mut available_import = 0.0;
                for corridor in &system.corridors {
                    if (corridor.area_a == *area_id || corridor.area_b == *area_id)
                        && !offline_corridors.contains(&corridor.id)
                    {
                        // Assume 50% of corridor capacity is available for import
                        available_import += corridor.capacity_mw * 0.5;
                    }
                }

                let total_available = area_supply + available_import;

                // Check for shortfall
                if total_available < required_demand {
                    let shortfall = required_demand - total_available;
                    let lole_contribution = area_scenario.probability;
                    let eue_contribution = shortfall * area_scenario.probability;

                    result.add_area_shortfall(*area_id, lole_contribution, eue_contribution);
                }
            }
        }

        // Track corridor utilization (assumed average 60% when online)
        for corridor in &system.corridors {
            if !offline_corridors.contains(&corridor.id) {
                result.add_corridor_utilization(corridor.id, 60.0);
            }
        }

        result
    }

    /// Reduce per-scenario results into final metrics
    fn reduce_scenario_results(
        &self,
        system: &MultiAreaSystem,
        results: Vec<ScenarioResult>,
    ) -> Result<AreaLoleMetrics> {
        let mut metrics = AreaLoleMetrics::new(self.num_scenarios);

        // Initialize area metrics
        for area_id in system.areas.keys() {
            metrics.area_lole.insert(*area_id, 0.0);
            metrics.area_eue.insert(*area_id, 0.0);
            metrics.zone_to_zone_lole.insert(*area_id, 0.0);
        }

        // Initialize corridor utilization tracking
        for corridor in &system.corridors {
            metrics.corridor_utilization.insert(corridor.id, 0.0);
        }

        // Aggregate results
        for result in results {
            if result.has_shortfall {
                metrics.scenarios_with_shortfall += 1;
            }

            for (area_id, (lole, eue)) in &result.area_shortfalls {
                *metrics.area_lole.get_mut(area_id).unwrap() += lole;
                *metrics.area_eue.get_mut(area_id).unwrap() += eue;
                *metrics.zone_to_zone_lole.get_mut(area_id).unwrap() += lole;
            }

            for (corridor_id, util) in &result.corridor_utils {
                *metrics.corridor_utilization.get_mut(corridor_id).unwrap() += util;
            }
        }

        // Convert LOLE from probability to hours/year
        for lole in metrics.area_lole.values_mut() {
            *lole *= self.hours_per_year;
        }

        // Convert EUE from probability-weighted MWh to annual MWh
        for eue in metrics.area_eue.values_mut() {
            *eue *= self.hours_per_year;
        }

        // Convert zone-to-zone LOLE similarly
        for z2z_lole in metrics.zone_to_zone_lole.values_mut() {
            *z2z_lole *= self.hours_per_year;
        }

        // Average corridor utilization across scenarios
        for util in metrics.corridor_utilization.values_mut() {
            *util /= self.num_scenarios as f64;
        }

        Ok(metrics)
    }

    /// Get per-area reliability as individual metrics
    pub fn compute_area_reliability(
        &self,
        system: &MultiAreaSystem,
        area_id: AreaId,
    ) -> Result<ReliabilityMetrics> {
        let multiarea_metrics = self.compute_multiarea_reliability(system)?;

        let lole = *multiarea_metrics
            .area_lole
            .get(&area_id)
            .ok_or_else(|| anyhow!("Area {:?} not found", area_id))?;
        let eue = *multiarea_metrics
            .area_eue
            .get(&area_id)
            .ok_or_else(|| anyhow!("Area {:?} not found", area_id))?;

        Ok(ReliabilityMetrics {
            lole,
            eue,
            scenarios_analyzed: self.num_scenarios,
            scenarios_with_shortfall: multiarea_metrics.scenarios_with_shortfall,
            average_shortfall: if multiarea_metrics.scenarios_with_shortfall > 0 {
                eue / multiarea_metrics.scenarios_with_shortfall as f64
            } else {
                0.0
            },
        })
    }
}

impl Default for MultiAreaMonteCarlo {
    fn default() -> Self {
        Self::new(1000)
    }
}
