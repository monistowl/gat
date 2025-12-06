use crate::arena::ArenaContext;
use anyhow::{anyhow, Result};
use gat_core::{BusId, Network, Node, NodeIndex};
use petgraph::visit::EdgeRef;
use rayon::prelude::*;
use std::collections::HashSet;

/// Represents a single outage scenario (which generators/lines are offline)
#[derive(Debug, Clone)]
pub struct OutageScenario {
    /// Generator node indices that are offline
    pub offline_generators: HashSet<NodeIndex>,
    /// Branch edge indices that are offline
    pub offline_branches: HashSet<usize>,
    /// Demand scaling factor (0.8 = 80% of nominal demand)
    pub demand_scale: f64,
    /// Scenario probability (should sum to 1.0 across all scenarios)
    pub probability: f64,
}

impl OutageScenario {
    /// Create a new scenario with all equipment online
    pub fn baseline() -> Self {
        Self {
            offline_generators: HashSet::new(),
            offline_branches: HashSet::new(),
            demand_scale: 1.0,
            probability: 0.0,
        }
    }

    /// Check if scenario is feasible (has available supply)
    pub fn has_capacity(&self, network: &Network, total_demand: f64) -> bool {
        let mut available_capacity = 0.0;

        // Iterate through all nodes to find generators
        for node_idx in network.graph.node_indices() {
            if let Some(Node::Gen(gen)) = network.graph.node_weight(node_idx) {
                if !self.offline_generators.contains(&node_idx) {
                    available_capacity += gen.active_power.value();
                }
            }
        }

        available_capacity >= total_demand * self.demand_scale
    }
}

/// Outage scenario generator with statistical failure rates
pub struct OutageGenerator {
    /// Generator failure rate (per year)
    pub gen_failure_rate: f64,
    /// Branch failure rate (per year)
    pub branch_failure_rate: f64,
    /// Demand variation range (0.8 to 1.2 = ±20%)
    pub demand_range: (f64, f64),
    /// Random seed for reproducibility
    pub seed: u64,
}

impl OutageGenerator {
    /// Create new scenario generator with default rates
    pub fn new() -> Self {
        Self {
            gen_failure_rate: 0.05,    // 5% failure rate per year
            branch_failure_rate: 0.02, // 2% failure rate per year
            demand_range: (0.8, 1.2),
            seed: 42,
        }
    }

    /// Generate N random outage scenarios
    pub fn generate_scenarios(
        &self,
        network: &Network,
        num_scenarios: usize,
    ) -> Vec<OutageScenario> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut scenarios = Vec::with_capacity(num_scenarios);
        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        let mut rng_state = hasher.finish();

        // Collect all generator node indices
        let gen_nodes: Vec<NodeIndex> = network
            .graph
            .node_indices()
            .filter(|&idx| matches!(network.graph.node_weight(idx), Some(Node::Gen(_))))
            .collect();

        // Collect all branch edge indices
        let branch_count = network.graph.edge_count();

        for _ in 0..num_scenarios {
            // Simple LCG random number generator
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let _rand_f64 = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;

            let mut offline_generators = HashSet::new();
            for &gen_idx in &gen_nodes {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let r = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;
                if r < self.gen_failure_rate {
                    offline_generators.insert(gen_idx);
                }
            }

            let mut offline_branches = HashSet::new();
            for idx in 0..branch_count {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                let r = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;
                if r < self.branch_failure_rate {
                    offline_branches.insert(idx);
                }
            }

            // Demand scaling
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let demand_r = ((rng_state >> 16) & 0x7fff) as f64 / 32768.0;
            let demand_scale =
                self.demand_range.0 + (self.demand_range.1 - self.demand_range.0) * demand_r;

            scenarios.push(OutageScenario {
                offline_generators,
                offline_branches,
                demand_scale,
                probability: 1.0 / num_scenarios as f64,
            });
        }

        scenarios
    }
}

impl Default for OutageGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// LOLE/EUE computation results
#[derive(Debug, Clone)]
pub struct ReliabilityMetrics {
    /// Loss of Load Expectation (hours per year)
    pub lole: f64,
    /// Energy Unserved (MWh per year)
    pub eue: f64,
    /// Number of scenarios analyzed
    pub scenarios_analyzed: usize,
    /// Number of scenarios with shortfall
    pub scenarios_with_shortfall: usize,
    /// Average shortfall when it occurs (MW)
    pub average_shortfall: f64,
}

/// Monte Carlo LOLE/EUE calculator
pub struct MonteCarlo {
    /// Scenario generator
    pub scenario_gen: OutageGenerator,
    /// Number of scenarios to run
    pub num_scenarios: usize,
    /// Hours per year (365.25 days * 24 hours)
    pub hours_per_year: f64,
}

impl MonteCarlo {
    /// Create new Monte Carlo analyzer
    pub fn new(num_scenarios: usize) -> Self {
        Self {
            scenario_gen: OutageGenerator::new(),
            num_scenarios,
            hours_per_year: 365.25 * 24.0,
        }
    }

    /// Compute LOLE and EUE for a network
    pub fn compute_reliability(&self, network: &Network) -> Result<ReliabilityMetrics> {
        use std::collections::{HashMap, HashSet};

        // Build lookup caches once (reused for all scenarios)
        let node_count = network.graph.node_count();
        let mut bus_id_to_node: HashMap<BusId, NodeIndex> = HashMap::with_capacity(node_count);
        let mut load_buses: HashSet<BusId> = HashSet::with_capacity(node_count);
        let mut total_demand = 0.0;

        for node_idx in network.graph.node_indices() {
            match network.graph.node_weight(node_idx) {
                Some(Node::Bus(bus)) => {
                    bus_id_to_node.insert(bus.id, node_idx);
                }
                Some(Node::Load(load)) => {
                    total_demand += load.active_power.value();
                    load_buses.insert(load.bus);
                }
                _ => {}
            }
        }

        if total_demand <= 0.0 {
            return Err(anyhow!("Network has no load"));
        }

        // Generate scenarios
        let scenarios = self
            .scenario_gen
            .generate_scenarios(network, self.num_scenarios);

        // Each parallel task gets its own arena context
        let results: Result<Vec<(f64, f64, bool)>> = scenarios
            .par_iter()
            .map_init(
                || ArenaContext::new(),
                |ctx, scenario| {
                    let available_gen = self.calculate_deliverable_generation_arena(
                        network,
                        scenario,
                        &bus_id_to_node,
                        &load_buses,
                        ctx,
                    )?;

                    let demand = total_demand * scenario.demand_scale;
                    let has_shortfall = available_gen < demand;
                    let shortfall = if has_shortfall {
                        demand - available_gen
                    } else {
                        0.0
                    };

                    ctx.reset(); // O(1) reset for next scenario

                    Ok((
                        scenario.probability,
                        shortfall * scenario.probability,
                        has_shortfall,
                    ))
                },
            )
            .collect();

        let results = results?;

        // Aggregate parallel results
        let (shortfall_hours, total_shortfall_mwh, scenarios_with_shortfall) = results.iter().fold(
            (0.0, 0.0, 0usize),
            |(sh, ts, count), (prob, shortfall_mwh, has_shortfall)| {
                if *has_shortfall {
                    (sh + prob, ts + shortfall_mwh, count + 1)
                } else {
                    (sh, ts, count)
                }
            },
        );

        // Convert shortfall hours to annual basis
        let lole = shortfall_hours * self.hours_per_year;
        let eue = total_shortfall_mwh * self.hours_per_year;

        let average_shortfall = if scenarios_with_shortfall > 0 {
            total_shortfall_mwh * self.hours_per_year / scenarios_with_shortfall as f64
        } else {
            0.0
        };

        Ok(ReliabilityMetrics {
            lole,
            eue,
            scenarios_analyzed: self.num_scenarios,
            scenarios_with_shortfall,
            average_shortfall,
        })
    }

    /// Calculate generation available to serve load considering branch connectivity
    ///
    /// This function determines which generators can actually reach the load nodes
    /// through available (online) branches. If critical branches are offline,
    /// some generators may be isolated and unable to contribute to supply.
    ///
    /// The algorithm works by:
    /// 1. Using pre-built map from BusId to NodeIndex for bus nodes (passed in)
    /// 2. Determining bus-level connectivity through online branches
    /// 3. Checking if each generator's bus can reach any load's bus
    #[allow(dead_code)]
    fn calculate_deliverable_generation(
        &self,
        network: &Network,
        scenario: &OutageScenario,
        bus_id_to_node: &std::collections::HashMap<BusId, NodeIndex>,
        load_buses: &std::collections::HashSet<BusId>,
    ) -> Result<f64> {
        use std::collections::{HashMap, HashSet, VecDeque};

        if load_buses.is_empty() {
            return Ok(0.0);
        }

        // Step 3: Find all online generators and their buses
        let mut gen_bus_capacity: Vec<(BusId, f64)> = Vec::new();
        for node_idx in network.graph.node_indices() {
            if let Some(Node::Gen(gen)) = network.graph.node_weight(node_idx) {
                if !scenario.offline_generators.contains(&node_idx) {
                    gen_bus_capacity.push((gen.bus, gen.active_power.value()));
                }
            }
        }

        if gen_bus_capacity.is_empty() {
            return Ok(0.0);
        }

        // Step 4: Build bus connectivity through online branches
        // We need to find which buses can reach which other buses
        let mut bus_reachable: HashMap<BusId, HashSet<BusId>> = HashMap::new();

        // Get all unique generator buses as starting points
        let gen_buses: HashSet<BusId> = gen_bus_capacity.iter().map(|(b, _)| *b).collect();

        for &start_bus in &gen_buses {
            // Verify the bus exists in the network
            if !bus_id_to_node.contains_key(&start_bus) {
                continue;
            }

            let mut visited_buses: HashSet<BusId> = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(start_bus);
            visited_buses.insert(start_bus);

            while let Some(current_bus) = queue.pop_front() {
                let Some(&current_node) = bus_id_to_node.get(&current_bus) else {
                    continue;
                };

                // Explore neighbors through online branches using adjacency iteration
                // This is O(degree) instead of O(edges), reducing overall complexity
                // from O(buses * edges) to O(buses * avg_degree)
                for edge_ref in network.graph.edges(current_node) {
                    let edge_idx = edge_ref.id();

                    // Skip offline branches
                    if scenario.offline_branches.contains(&edge_idx.index()) {
                        continue;
                    }

                    // Get the neighbor node (edge_ref.target() for directed graphs,
                    // but we also need to check source for undirected traversal)
                    let neighbor = edge_ref.target();
                    let neighbor = if neighbor == current_node {
                        edge_ref.source()
                    } else {
                        neighbor
                    };

                    // Get the neighbor's bus ID if it's a bus node
                    if let Some(Node::Bus(neighbor_bus)) = network.graph.node_weight(neighbor) {
                        if !visited_buses.contains(&neighbor_bus.id) {
                            visited_buses.insert(neighbor_bus.id);
                            queue.push_back(neighbor_bus.id);
                        }
                    }
                }
            }

            bus_reachable.insert(start_bus, visited_buses);
        }

        // Step 5: Calculate deliverable generation
        // A generator is deliverable if its bus can reach any load bus
        let mut total_deliverable = 0.0;

        for (gen_bus, capacity) in gen_bus_capacity {
            if let Some(reachable_buses) = bus_reachable.get(&gen_bus) {
                if reachable_buses.iter().any(|b| load_buses.contains(b)) {
                    total_deliverable += capacity;
                }
            }
        }

        Ok(total_deliverable)
    }

    /// Arena-allocated version of calculate_deliverable_generation.
    ///
    /// Uses arena-backed collections for temporary allocations,
    /// enabling O(1) bulk deallocation between scenarios.
    fn calculate_deliverable_generation_arena(
        &self,
        network: &Network,
        scenario: &OutageScenario,
        bus_id_to_node: &std::collections::HashMap<BusId, NodeIndex>,
        load_buses: &std::collections::HashSet<BusId>,
        ctx: &ArenaContext,
    ) -> Result<f64> {
        if load_buses.is_empty() {
            return Ok(0.0);
        }

        // Step 3: Find all online generators and their buses
        let mut gen_bus_capacity = ctx.alloc_vec::<(BusId, f64)>();
        for node_idx in network.graph.node_indices() {
            if let Some(Node::Gen(gen)) = network.graph.node_weight(node_idx) {
                if !scenario.offline_generators.contains(&node_idx) {
                    gen_bus_capacity.push((gen.bus, gen.active_power.value()));
                }
            }
        }

        if gen_bus_capacity.is_empty() {
            return Ok(0.0);
        }

        // Step 4: Build bus connectivity through online branches
        let mut bus_reachable =
            ctx.alloc_hashmap::<BusId, hashbrown::HashSet<BusId, _, &bumpalo::Bump>>();

        // Get all unique generator buses as starting points
        let mut gen_buses = ctx.alloc_hashset::<BusId>();
        for (b, _) in gen_bus_capacity.iter() {
            gen_buses.insert(*b);
        }

        for &start_bus in gen_buses.iter() {
            if !bus_id_to_node.contains_key(&start_bus) {
                continue;
            }

            let mut visited_buses = ctx.alloc_hashset::<BusId>();
            let mut queue = ctx.alloc_vec::<BusId>();
            queue.push(start_bus);
            visited_buses.insert(start_bus);

            while let Some(current_bus) = queue.pop() {
                let Some(&current_node) = bus_id_to_node.get(&current_bus) else {
                    continue;
                };

                for edge_ref in network.graph.edges(current_node) {
                    let edge_idx = edge_ref.id();

                    if scenario.offline_branches.contains(&edge_idx.index()) {
                        continue;
                    }

                    let neighbor = edge_ref.target();
                    let neighbor = if neighbor == current_node {
                        edge_ref.source()
                    } else {
                        neighbor
                    };

                    if let Some(Node::Bus(neighbor_bus)) = network.graph.node_weight(neighbor) {
                        if !visited_buses.contains(&neighbor_bus.id) {
                            visited_buses.insert(neighbor_bus.id);
                            queue.push(neighbor_bus.id);
                        }
                    }
                }
            }

            bus_reachable.insert(start_bus, visited_buses);
        }

        // Step 5: Calculate deliverable generation
        let mut total_deliverable = 0.0;

        for (gen_bus, capacity) in gen_bus_capacity.iter() {
            if let Some(reachable_buses) = bus_reachable.get(gen_bus) {
                if reachable_buses.iter().any(|b| load_buses.contains(b)) {
                    total_deliverable += capacity;
                }
            }
        }

        Ok(total_deliverable)
    }

    /// Compute LOLE for multiple networks (e.g., different scenarios)
    pub fn compute_multiple(&self, networks: &[Network]) -> Result<Vec<ReliabilityMetrics>> {
        let mut results = Vec::with_capacity(networks.len());
        for network in networks {
            results.push(self.compute_reliability(network)?);
        }
        Ok(results)
    }
}

impl Default for MonteCarlo {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Deliverability Score configuration
///
/// Composite reliability metric combining multiple failure modes into a 0-100 score.
/// Formula: DeliverabilityScore = 100 * [1 - w_lole * (LOLE/LOLE_max)
///                                         - w_voltage * (violations/max_violations)
///                                         - w_thermal * (overloads/max_overloads)]
#[derive(Debug, Clone)]
pub struct DeliverabilityScoreConfig {
    /// Weight for LOLE component (0.0-1.0)
    pub weight_lole: f64,
    /// Weight for voltage violations component (0.0-1.0)
    pub weight_voltage: f64,
    /// Weight for thermal overloads component (0.0-1.0)
    pub weight_thermal: f64,
    /// Maximum acceptable LOLE (hours/year) for scoring
    pub lole_max: f64,
    /// Maximum acceptable voltage violations for scoring
    pub max_violations: f64,
    /// Maximum acceptable thermal overloads for scoring
    pub max_overloads: f64,
}

impl DeliverabilityScoreConfig {
    /// Create new configuration with default values
    pub fn new() -> Self {
        Self {
            weight_lole: 1.0,
            weight_voltage: 0.0,
            weight_thermal: 0.0,
            lole_max: 3.0, // NERC benchmark: ~0.5-3 hrs/year
            max_violations: 10.0,
            max_overloads: 5.0,
        }
    }

    /// Builder: Set LOLE weight
    pub fn with_weight_lole(mut self, weight: f64) -> Self {
        self.weight_lole = weight;
        self
    }

    /// Builder: Set voltage violations weight
    pub fn with_weight_voltage(mut self, weight: f64) -> Self {
        self.weight_voltage = weight;
        self
    }

    /// Builder: Set thermal overloads weight
    pub fn with_weight_thermal(mut self, weight: f64) -> Self {
        self.weight_thermal = weight;
        self
    }

    /// Builder: Set maximum LOLE threshold
    pub fn with_lole_max(mut self, lole_max: f64) -> Self {
        self.lole_max = lole_max;
        self
    }

    /// Validate that weights sum to something reasonable
    pub fn validate(&self) -> Result<()> {
        let total_weight = self.weight_lole + self.weight_voltage + self.weight_thermal;
        if total_weight <= 0.0 {
            return Err(anyhow!("Deliverability score: total weight must be > 0"));
        }
        if self.lole_max <= 0.0 {
            return Err(anyhow!("Deliverability score: lole_max must be > 0"));
        }
        Ok(())
    }
}

impl Default for DeliverabilityScoreConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Deliverability Score: composite reliability metric (0-100)
#[derive(Debug, Clone)]
pub struct DeliverabilityScore {
    /// Score value 0-100 (higher = more reliable)
    pub score: f64,
    /// LOLE contribution to score reduction (0.0-1.0)
    pub lole_factor: f64,
    /// Voltage violations contribution to score reduction (0.0-1.0)
    pub voltage_factor: f64,
    /// Thermal overloads contribution to score reduction (0.0-1.0)
    pub thermal_factor: f64,
    /// Underlying reliability metrics
    pub metrics: ReliabilityMetrics,
}

impl DeliverabilityScore {
    /// Compute Deliverability Score from reliability metrics
    ///
    /// Currently only supports LOLE-based scoring (Phase 3.11).
    /// Voltage violations and thermal overloads will be integrated in Task 13.
    pub fn from_metrics(
        metrics: ReliabilityMetrics,
        config: &DeliverabilityScoreConfig,
    ) -> Result<Self> {
        config.validate()?;

        // Normalized LOLE factor (0.0 = perfect, 1.0+ = exceeded max)
        let lole_factor = (metrics.lole / config.lole_max).min(1.0);

        // Voltage and thermal factors currently 0 (no OPF integration yet)
        let voltage_factor = 0.0;
        let thermal_factor = 0.0;

        // Calculate weighted score reduction
        let total_weight = config.weight_lole + config.weight_voltage + config.weight_thermal;
        let weighted_reduction = (config.weight_lole * lole_factor
            + config.weight_voltage * voltage_factor
            + config.weight_thermal * thermal_factor)
            / total_weight;

        let score = 100.0 * (1.0 - weighted_reduction);

        Ok(Self {
            score: score.clamp(0.0, 100.0), // Clamp to 0-100
            lole_factor,
            voltage_factor,
            thermal_factor,
            metrics,
        })
    }

    /// Determine reliability status based on score
    pub fn status(&self) -> &'static str {
        match self.score {
            90.0..=100.0 => "Excellent",
            80.0..90.0 => "Good",
            70.0..80.0 => "Fair",
            60.0..70.0 => "Poor",
            _ => "Critical",
        }
    }

    /// Check if score meets minimum threshold
    pub fn meets_threshold(&self, min_score: f64) -> bool {
        self.score >= min_score
    }
}
