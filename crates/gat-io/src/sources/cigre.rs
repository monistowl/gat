/// CIGRE Medium Voltage (MV) test network builder
///
/// This module provides programmatic construction of the CIGRE 14-bus MV distribution
/// network, commonly used for state estimation and distribution system analysis
/// benchmarks. The network topology and parameters are based on CIGRE Task Force
/// C6.04.02 benchmark systems.
///
/// Reference: CIGRE Task Force C6.04.02, "Benchmark Systems for Network Integration
/// of Renewable and Distributed Energy Resources" (2014)
///
/// This network is used by the DSS² (Deep Statistical Solver for Distribution System
/// State Estimation) paper for WLS baseline comparisons.
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Kilovolts, Load, LoadId, Megavars, Megawatts,
    Network, Node, PerUnit, Radians,
};
use std::collections::HashMap;

/// CIGRE MV network configuration
#[derive(Debug, Clone)]
pub struct CigreMvConfig {
    /// Base voltage in kV (default: 20.0 kV for MV)
    pub base_kv: f64,
    /// System base MVA (default: 100.0)
    pub base_mva: f64,
    /// Load scaling factor (default: 1.0)
    pub load_scale: f64,
}

impl Default for CigreMvConfig {
    fn default() -> Self {
        Self {
            base_kv: 20.0,
            base_mva: 100.0,
            load_scale: 1.0,
        }
    }
}

/// Build the CIGRE 14-bus MV distribution network
///
/// The network consists of:
/// - Bus 0: Slack bus (HV/MV substation)
/// - Buses 1-13: Load buses with residential/commercial loads
/// - 13 distribution branches (overhead lines and cables)
///
/// Network is radial with a main feeder and lateral branches.
pub fn build_cigre_mv_network(config: &CigreMvConfig) -> Network {
    let mut network = Network::new();
    let mut bus_nodes: HashMap<usize, gat_core::NodeIndex> = HashMap::new();

    // CIGRE MV 14-bus topology based on CIGRE Task Force C6.04.02
    // Bus 0 is the slack (HV/MV substation), buses 1-13 are load buses
    //
    // Topology (radial feeder):
    //   0 (slack) -- 1 -- 2 -- 3 -- 4 -- 5 -- 6 -- 7
    //                     |         |
    //                     8 -- 9    10 -- 11 -- 12 -- 13

    // Add buses (14 buses: 0-13)
    for i in 0..14 {
        let bus_name = if i == 0 {
            "Slack (HV/MV Substation)".to_string()
        } else {
            format!("Bus {}", i)
        };

        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(i),
            name: bus_name,
            base_kv: Kilovolts(config.base_kv),
            voltage_pu: PerUnit(1.0),
            angle_rad: Radians(0.0),
            vmin_pu: Some(PerUnit(0.9)),
            vmax_pu: Some(PerUnit(1.1)),
            area_id: Some(1),
            zone_id: Some(1),
        }));
        bus_nodes.insert(i, node_idx);
    }

    // Add slack generator at bus 0
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "Grid Supply".to_string(),
        bus: BusId::new(0),
        active_power: Megawatts(0.0), // Will be solved by PF
        reactive_power: Megavars(0.0),
        pmin: Megawatts(-1000.0), // Can absorb or supply
        pmax: Megawatts(1000.0),
        qmin: Megavars(-500.0),
        qmax: Megavars(500.0),
        status: true,
        voltage_setpoint: Some(PerUnit(1.0)),
        ..Gen::default()
    }));

    // Load data for each bus (MW, Mvar) - typical residential/commercial distribution
    // Based on CIGRE MV benchmark: total load ~28 MW
    let loads = [
        // (bus_id, P_MW, Q_Mvar)
        (1, 2.0, 0.6),
        (2, 2.5, 0.8),
        (3, 2.0, 0.6),
        (4, 3.0, 1.0),
        (5, 2.5, 0.8),
        (6, 2.0, 0.6),
        (7, 3.0, 1.0),
        (8, 1.5, 0.5),
        (9, 2.0, 0.6),
        (10, 2.5, 0.8),
        (11, 2.0, 0.6),
        (12, 1.5, 0.5),
        (13, 1.5, 0.5),
    ];

    for (i, (bus_id, p_mw, q_mvar)) in loads.iter().enumerate() {
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(i),
            name: format!("Load at Bus {}", bus_id),
            bus: BusId::new(*bus_id),
            active_power: Megawatts(*p_mw * config.load_scale),
            reactive_power: Megavars(*q_mvar * config.load_scale),
        }));
    }

    // Branch data: (from_bus, to_bus, r_pu, x_pu, b_pu)
    // Impedances in per-unit on 100 MVA, 20 kV base
    // R and X values typical for MV overhead lines and cables
    let branches = [
        // Main feeder: 0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7
        (0, 1, 0.0005, 0.0050, 0.0001),
        (1, 2, 0.0008, 0.0080, 0.0001),
        (2, 3, 0.0010, 0.0100, 0.0002),
        (3, 4, 0.0012, 0.0120, 0.0002),
        (4, 5, 0.0010, 0.0100, 0.0002),
        (5, 6, 0.0008, 0.0080, 0.0001),
        (6, 7, 0.0006, 0.0060, 0.0001),
        // Lateral branch from bus 2: 2 -> 8 -> 9
        (2, 8, 0.0015, 0.0150, 0.0001),
        (8, 9, 0.0010, 0.0100, 0.0001),
        // Lateral branch from bus 4: 4 -> 10 -> 11 -> 12 -> 13
        (4, 10, 0.0012, 0.0120, 0.0002),
        (10, 11, 0.0010, 0.0100, 0.0001),
        (11, 12, 0.0008, 0.0080, 0.0001),
        (12, 13, 0.0006, 0.0060, 0.0001),
    ];

    for (i, (from, to, r, x, b)) in branches.iter().enumerate() {
        let from_idx = bus_nodes[from];
        let to_idx = bus_nodes[to];

        network.graph.add_edge(
            from_idx,
            to_idx,
            Edge::Branch(Branch {
                id: BranchId::new(i),
                name: format!("Line {}-{}", from, to),
                from_bus: BusId::new(*from),
                to_bus: BusId::new(*to),
                resistance: *r,
                reactance: *x,
                charging_b: PerUnit(*b),
                tap_ratio: 1.0,
                phase_shift: Radians(0.0),
                status: true,
                ..Branch::default()
            }),
        );
    }

    network
}

/// Measurement type for state estimation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeasurementType {
    /// Branch power flow (MW)
    Flow,
    /// Bus power injection (MW)
    Injection,
    /// Bus voltage angle (radians)
    Angle,
    /// Bus voltage magnitude (per-unit)
    Voltage,
}

impl MeasurementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MeasurementType::Flow => "flow",
            MeasurementType::Injection => "injection",
            MeasurementType::Angle => "angle",
            MeasurementType::Voltage => "voltage",
        }
    }
}

/// Single measurement for state estimation
#[derive(Debug, Clone)]
pub struct Measurement {
    pub measurement_type: MeasurementType,
    pub branch_id: Option<i64>,
    pub bus_id: Option<usize>,
    pub value: f64,
    pub weight: f64,
    pub label: Option<String>,
}

impl Measurement {
    /// Create a flow measurement
    pub fn flow(branch_id: i64, value: f64, weight: f64) -> Self {
        Self {
            measurement_type: MeasurementType::Flow,
            branch_id: Some(branch_id),
            bus_id: None,
            value,
            weight,
            label: Some(format!("Flow on branch {}", branch_id)),
        }
    }

    /// Create an injection measurement
    pub fn injection(bus_id: usize, value: f64, weight: f64) -> Self {
        Self {
            measurement_type: MeasurementType::Injection,
            branch_id: None,
            bus_id: Some(bus_id),
            value,
            weight,
            label: Some(format!("Injection at bus {}", bus_id)),
        }
    }

    /// Create a voltage angle measurement
    pub fn angle(bus_id: usize, value: f64, weight: f64) -> Self {
        Self {
            measurement_type: MeasurementType::Angle,
            branch_id: None,
            bus_id: Some(bus_id),
            value,
            weight,
            label: Some(format!("Angle at bus {}", bus_id)),
        }
    }

    /// Create a voltage magnitude measurement
    pub fn voltage(bus_id: usize, value: f64, weight: f64) -> Self {
        Self {
            measurement_type: MeasurementType::Voltage,
            branch_id: None,
            bus_id: Some(bus_id),
            value,
            weight,
            label: Some(format!("Voltage at bus {}", bus_id)),
        }
    }

    /// Add Gaussian noise to the measurement value using Box-Muller transform
    pub fn with_noise(mut self, std_dev: f64, rng: &mut impl rand::Rng) -> Self {
        // Box-Muller transform for Gaussian noise
        let u1: f64 = rand::Rng::gen::<f64>(rng).max(1e-10); // Avoid log(0)
        let u2: f64 = rand::Rng::gen::<f64>(rng);
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        self.value += z * std_dev;
        self
    }
}

/// Configuration for generating synthetic SE measurements
#[derive(Debug, Clone)]
pub struct MeasurementGeneratorConfig {
    /// Number of flow measurements to generate (0 = none)
    pub num_flow_measurements: usize,
    /// Number of injection measurements to generate (0 = none)
    pub num_injection_measurements: usize,
    /// Number of voltage measurements to generate (0 = none)
    pub num_voltage_measurements: usize,
    /// Standard deviation for measurement noise (0 = no noise)
    pub noise_std_dev: f64,
    /// Base weight for measurements (higher = more trusted)
    pub base_weight: f64,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for MeasurementGeneratorConfig {
    fn default() -> Self {
        Self {
            num_flow_measurements: 5,
            num_injection_measurements: 5,
            num_voltage_measurements: 3,
            noise_std_dev: 0.01,
            base_weight: 1.0,
            seed: Some(42),
        }
    }
}

/// Generate synthetic measurements from a power flow solution
///
/// This creates a measurement set suitable for WLS state estimation benchmarking.
/// True values are computed from the DC power flow solution, then optional noise
/// is added to simulate real measurement uncertainty.
///
/// # Arguments
/// * `network` - The power network (after PF solution)
/// * `bus_angles` - Map of bus_id -> voltage angle (radians) from PF
/// * `branch_flows` - Map of branch_id -> power flow (MW) from PF
/// * `config` - Measurement generation configuration
///
/// # Returns
/// Vector of measurements suitable for SE
pub fn generate_measurements(
    bus_angles: &HashMap<usize, f64>,
    branch_flows: &HashMap<i64, f64>,
    bus_injections: &HashMap<usize, f64>,
    config: &MeasurementGeneratorConfig,
) -> Vec<Measurement> {
    use rand::SeedableRng;

    let mut measurements = Vec::new();
    let mut rng = match config.seed {
        Some(seed) => rand::rngs::StdRng::seed_from_u64(seed),
        None => rand::rngs::StdRng::from_entropy(),
    };

    // Select branches for flow measurements
    let branch_ids: Vec<i64> = branch_flows.keys().cloned().collect();
    let num_flows = config.num_flow_measurements.min(branch_ids.len());

    for &branch_id in branch_ids.iter().take(num_flows) {
        if let Some(&flow) = branch_flows.get(&branch_id) {
            let m = Measurement::flow(branch_id, flow, config.base_weight);
            let m = if config.noise_std_dev > 0.0 {
                m.with_noise(config.noise_std_dev, &mut rng)
            } else {
                m
            };
            measurements.push(m);
        }
    }

    // Select buses for injection measurements (exclude slack bus 0)
    let injection_buses: Vec<usize> = bus_injections
        .keys()
        .filter(|&&b| b != 0)
        .cloned()
        .collect();
    let num_injections = config.num_injection_measurements.min(injection_buses.len());

    for &bus_id in injection_buses.iter().take(num_injections) {
        if let Some(&inj) = bus_injections.get(&bus_id) {
            let m = Measurement::injection(bus_id, inj, config.base_weight);
            let m = if config.noise_std_dev > 0.0 {
                m.with_noise(config.noise_std_dev, &mut rng)
            } else {
                m
            };
            measurements.push(m);
        }
    }

    // Select buses for voltage/angle measurements
    let angle_buses: Vec<usize> = bus_angles.keys().filter(|&&b| b != 0).cloned().collect();
    let num_voltages = config.num_voltage_measurements.min(angle_buses.len());

    for &bus_id in angle_buses.iter().take(num_voltages) {
        if let Some(&angle) = bus_angles.get(&bus_id) {
            let m = Measurement::angle(bus_id, angle, config.base_weight);
            let m = if config.noise_std_dev > 0.0 {
                m.with_noise(config.noise_std_dev * 0.1, &mut rng) // Angles have tighter tolerance
            } else {
                m
            };
            measurements.push(m);
        }
    }

    measurements
}

/// Write measurements to CSV format compatible with `gat se wls`
pub fn write_measurements_csv(
    measurements: &[Measurement],
    writer: &mut impl std::io::Write,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "measurement_type,branch_id,bus_id,value,weight,label"
    )?;

    for m in measurements {
        let branch_id = m.branch_id.map(|b| b.to_string()).unwrap_or_default();
        let bus_id = m.bus_id.map(|b| b.to_string()).unwrap_or_default();
        let label = m.label.as_deref().unwrap_or("");

        writeln!(
            writer,
            "{},{},{},{:.6},{:.6},\"{}\"",
            m.measurement_type.as_str(),
            branch_id,
            bus_id,
            m.value,
            m.weight,
            label
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_cigre_mv_network() {
        let config = CigreMvConfig::default();
        let network = build_cigre_mv_network(&config);

        let stats = network.stats();
        assert_eq!(stats.num_buses, 14, "Should have 14 buses");
        assert_eq!(stats.num_branches, 13, "Should have 13 branches");
        assert_eq!(stats.num_gens, 1, "Should have 1 generator (slack)");
        assert_eq!(stats.num_loads, 13, "Should have 13 loads");

        // Total load should be approximately 28 MW
        assert!(
            (stats.total_load_mw - 28.0).abs() < 0.1,
            "Total load should be ~28 MW, got {}",
            stats.total_load_mw
        );
    }

    #[test]
    fn test_measurement_generation() {
        use std::collections::HashMap;

        // Create fake PF solution data
        let mut bus_angles = HashMap::new();
        let mut branch_flows = HashMap::new();
        let mut bus_injections = HashMap::new();

        for i in 0..14 {
            bus_angles.insert(i, -0.01 * (i as f64)); // Decreasing angles
            if i > 0 {
                bus_injections.insert(i, -2.0); // Negative = load
            }
        }

        for i in 0..13 {
            branch_flows.insert(i as i64, 5.0 - 0.3 * (i as f64)); // Decreasing flow
        }

        let config = MeasurementGeneratorConfig {
            num_flow_measurements: 3,
            num_injection_measurements: 3,
            num_voltage_measurements: 2,
            noise_std_dev: 0.0, // No noise for deterministic test
            base_weight: 1.0,
            seed: Some(42),
        };

        let measurements =
            generate_measurements(&bus_angles, &branch_flows, &bus_injections, &config);

        assert_eq!(measurements.len(), 8, "Should have 8 measurements total");

        // Check measurement types
        let flows = measurements
            .iter()
            .filter(|m| m.measurement_type == MeasurementType::Flow)
            .count();
        let injections = measurements
            .iter()
            .filter(|m| m.measurement_type == MeasurementType::Injection)
            .count();
        let angles = measurements
            .iter()
            .filter(|m| m.measurement_type == MeasurementType::Angle)
            .count();

        assert_eq!(flows, 3);
        assert_eq!(injections, 3);
        assert_eq!(angles, 2);
    }

    #[test]
    fn test_measurements_csv_format() {
        let measurements = vec![
            Measurement::flow(0, 5.0, 1.0),
            Measurement::injection(1, -2.0, 1.0),
            Measurement::angle(2, -0.02, 1.0),
        ];

        let mut buffer = Vec::new();
        write_measurements_csv(&measurements, &mut buffer).unwrap();

        let csv_str = String::from_utf8(buffer).unwrap();
        assert!(csv_str.contains("measurement_type,branch_id,bus_id,value,weight,label"));
        assert!(csv_str.contains("flow,0,"));
        assert!(csv_str.contains("injection,,1,"));
        assert!(csv_str.contains("angle,,2,"));
    }
}
