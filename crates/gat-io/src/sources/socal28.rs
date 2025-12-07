//! SoCal 28-Bus Distribution Network Digital Twin
//!
//! This module provides a programmatic construction of a distribution network
//! representative of the SoCal 28-Bus system described in:
//!
//! > Xie, Y., Werner, L., Chen, K., Le, T.-L., Ortega, C., & Low, S. (2025).
//! > "A Digital Twin of an Electrical Distribution Grid: SoCal 28-Bus Dataset"
//! > arXiv:2504.06588
//!
//! The real SoCal 28-Bus dataset is available via the Caltech API at
//! <https://socal28bus.caltech.edu> with data from <https://github.com/caltech-netlab/digital-twin-dataset>.
//!
//! ## Network Characteristics
//!
//! The SoCal 28-Bus system is a real-world distribution grid featuring:
//! - 28 buses across primary and secondary voltage levels
//! - Dense PMU deployment (all injection buses monitored)
//! - Diverse generation: solar PV, fuel cells, natural gas, utility interconnection
//! - Diverse loads: EV charging, data centers, central cooling, office buildings
//! - Three-phase and single-phase sections
//!
//! ## Voltage Levels
//!
//! - Primary: 16.5 kV / 2.4 kV at substation
//! - Secondary: 277/480 V at most meter locations
//!
//! ## PMU Data Types
//!
//! The dataset provides three measurement types:
//! 1. **Magnitude** - RMS current and voltage at 1-second intervals
//! 2. **Synchro-phasor** - Complex phasors at 10-second intervals
//! 3. **Synchro-waveform** - Raw point-on-wave at 2.5 kHz (1-second windows)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use gat_io::sources::socal28::{build_socal28_network, SoCal28Config};
//!
//! let config = SoCal28Config::default();
//! let network = build_socal28_network(&config);
//!
//! // Get PMU station mappings
//! let stations = config.pmu_stations();
//! ```

use crate::sources::pmu::{PmuFrame, PmuQuality, PmuStationInfo, PmuTimeSeries};
use gat_core::{
    Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Kilovolts, Load, LoadId, Megavars, Megawatts,
    Network, Node, PerUnit, Radians,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the SoCal 28-Bus network
#[derive(Debug, Clone)]
pub struct SoCal28Config {
    /// Primary (medium voltage) level in kV
    pub primary_kv: f64,
    /// Secondary (low voltage) level in kV
    pub secondary_kv: f64,
    /// System base MVA
    pub base_mva: f64,
    /// Load scaling factor (default: 1.0)
    pub load_scale: f64,
    /// Include DER (solar, fuel cells) as negative loads or generators
    pub der_as_generators: bool,
}

impl Default for SoCal28Config {
    fn default() -> Self {
        Self {
            primary_kv: 16.5,   // MV primary
            secondary_kv: 0.48, // 480V secondary
            base_mva: 10.0,     // Distribution system base
            load_scale: 1.0,
            der_as_generators: true,
        }
    }
}

impl SoCal28Config {
    /// Get PMU station information for the network
    ///
    /// Returns a vector of station metadata matching the PMU deployment
    /// in the SoCal 28-Bus dataset.
    pub fn pmu_stations(&self) -> Vec<PmuStationInfo> {
        vec![
            PmuStationInfo {
                station_id: "SUB".to_string(),
                name: Some("Main Substation".to_string()),
                bus_id: Some(0),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.primary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_1".to_string(),
                name: Some("Solar PV Array".to_string()),
                bus_id: Some(5),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_2".to_string(),
                name: Some("Data Center".to_string()),
                bus_id: Some(8),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_3".to_string(),
                name: Some("EV Charging Station".to_string()),
                bus_id: Some(12),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_4".to_string(),
                name: Some("Central Cooling Plant".to_string()),
                bus_id: Some(15),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_5".to_string(),
                name: Some("Office Building A".to_string()),
                bus_id: Some(18),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_6".to_string(),
                name: Some("Fuel Cell Generator".to_string()),
                bus_id: Some(21),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_7".to_string(),
                name: Some("Natural Gas Generator".to_string()),
                bus_id: Some(24),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
            PmuStationInfo {
                station_id: "egauge_8".to_string(),
                name: Some("Office Building B".to_string()),
                bus_id: Some(27),
                vt_ratio: 1.0,
                ct_ratio: 1.0,
                base_kv: self.secondary_kv,
            },
        ]
    }

    /// Get mapping from station ID to bus ID
    pub fn station_to_bus_map(&self) -> HashMap<String, usize> {
        self.pmu_stations()
            .into_iter()
            .filter_map(|s| s.bus_id.map(|b| (s.station_id, b)))
            .collect()
    }
}

/// Bus type classification for the SoCal 28-Bus network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoCal28BusType {
    /// Substation connection point
    Substation,
    /// Primary (MV) feeder bus
    PrimaryFeeder,
    /// Distribution transformer (MV/LV)
    Transformer,
    /// Secondary (LV) load bus
    SecondaryLoad,
    /// Generation bus (solar, fuel cell, NG)
    Generation,
}

/// Build the SoCal 28-Bus distribution network
///
/// The network topology is based on a typical Southern California distribution
/// system with:
/// - One main substation (bus 0)
/// - Three primary feeders branching out
/// - Distribution transformers stepping down to secondary
/// - Mixed load and DER at secondary level
///
/// ```text
///                          [0] Substation
///                              |
///              +---------------+---------------+
///              |               |               |
///            [1]             [2]             [3]
///           Feeder1         Feeder2         Feeder3
///              |               |               |
///        +-----+-----+   +-----+-----+   +-----+-----+
///        |     |     |   |     |     |   |     |     |
///       [4]  [5]   [6]  [7]  [8]   [9]  [10] [11]  [12]
///        |   (PV)   |    |  (DC)   |    |    |   (EV)
///        |         |    |         |    |    |     |
///      [13] [14] [15] [16] [17] [18] [19] [20] [21]
///       |    |  (CC)  |    |  (OA)  |    |   (FC)
///       |    |        |    |        |    |     |
///     [22] [23]     [24] [25]     [26] [27]
///            |      (NG)              (OB)
///          [...]
/// ```
///
/// Legend:
/// - PV: Solar PV Array
/// - DC: Data Center
/// - EV: EV Charging Station
/// - CC: Central Cooling
/// - OA: Office Building A
/// - FC: Fuel Cell
/// - NG: Natural Gas Generator
/// - OB: Office Building B
pub fn build_socal28_network(config: &SoCal28Config) -> Network {
    let mut network = Network::new();
    let mut bus_nodes: HashMap<usize, gat_core::NodeIndex> = HashMap::new();

    // Bus definitions for SoCal 28-Bus
    // Format: (bus_id, name, voltage_kv, bus_type)
    let buses: Vec<(usize, &str, f64, SoCal28BusType)> = vec![
        // Substation
        (
            0,
            "Main Substation",
            config.primary_kv,
            SoCal28BusType::Substation,
        ),
        // Primary feeder buses
        (
            1,
            "Feeder 1 Head",
            config.primary_kv,
            SoCal28BusType::PrimaryFeeder,
        ),
        (
            2,
            "Feeder 2 Head",
            config.primary_kv,
            SoCal28BusType::PrimaryFeeder,
        ),
        (
            3,
            "Feeder 3 Head",
            config.primary_kv,
            SoCal28BusType::PrimaryFeeder,
        ),
        // Feeder 1 secondary
        (
            4,
            "F1 Xfmr A",
            config.secondary_kv,
            SoCal28BusType::Transformer,
        ),
        (
            5,
            "Solar PV Site",
            config.secondary_kv,
            SoCal28BusType::Generation,
        ),
        (
            6,
            "F1 Xfmr B",
            config.secondary_kv,
            SoCal28BusType::Transformer,
        ),
        // Feeder 2 secondary
        (
            7,
            "F2 Xfmr A",
            config.secondary_kv,
            SoCal28BusType::Transformer,
        ),
        (
            8,
            "Data Center",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            9,
            "F2 Xfmr B",
            config.secondary_kv,
            SoCal28BusType::Transformer,
        ),
        // Feeder 3 secondary
        (
            10,
            "F3 Xfmr A",
            config.secondary_kv,
            SoCal28BusType::Transformer,
        ),
        (
            11,
            "F3 Xfmr B",
            config.secondary_kv,
            SoCal28BusType::Transformer,
        ),
        (
            12,
            "EV Charging",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        // Further secondary
        (
            13,
            "Res Load 1",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            14,
            "Res Load 2",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            15,
            "Central Cooling",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            16,
            "Res Load 3",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            17,
            "Res Load 4",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            18,
            "Office A",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            19,
            "Res Load 5",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            20,
            "Res Load 6",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            21,
            "Fuel Cell Site",
            config.secondary_kv,
            SoCal28BusType::Generation,
        ),
        (
            22,
            "Res Load 7",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            23,
            "Res Load 8",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            24,
            "NG Generator",
            config.secondary_kv,
            SoCal28BusType::Generation,
        ),
        (
            25,
            "Res Load 9",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            26,
            "Res Load 10",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
        (
            27,
            "Office B",
            config.secondary_kv,
            SoCal28BusType::SecondaryLoad,
        ),
    ];

    // Add buses to network
    for (bus_id, name, base_kv, _bus_type) in &buses {
        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(*bus_id),
            name: name.to_string(),
            base_kv: Kilovolts(*base_kv),
            voltage_pu: PerUnit(1.0),
            angle_rad: Radians(0.0),
            vmin_pu: Some(PerUnit(0.95)),
            vmax_pu: Some(PerUnit(1.05)),
            area_id: Some(1),
            zone_id: Some(1),
        }));
        bus_nodes.insert(*bus_id, node_idx);
    }

    // Add slack generator at substation (utility interconnection)
    network.graph.add_node(Node::Gen(Gen {
        id: GenId::new(0),
        name: "Utility Interconnection".to_string(),
        bus: BusId::new(0),
        active_power: Megawatts(0.0),
        reactive_power: Megavars(0.0),
        pmin: Megawatts(-10.0), // Can export to grid
        pmax: Megawatts(10.0),
        qmin: Megavars(-5.0),
        qmax: Megavars(5.0),
        status: true,
        voltage_setpoint: Some(PerUnit(1.0)),
        ..Gen::default()
    }));

    // Add DER generators if configured
    if config.der_as_generators {
        // Solar PV at bus 5
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(1),
            name: "Solar PV Array".to_string(),
            bus: BusId::new(5),
            active_power: Megawatts(0.5 * config.load_scale),
            reactive_power: Megavars(0.0),
            pmin: Megawatts(0.0),
            pmax: Megawatts(1.0),
            qmin: Megavars(-0.3),
            qmax: Megavars(0.3),
            status: true,
            voltage_setpoint: None,
            ..Gen::default()
        }));

        // Fuel Cell at bus 21
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(2),
            name: "Fuel Cell".to_string(),
            bus: BusId::new(21),
            active_power: Megawatts(0.3 * config.load_scale),
            reactive_power: Megavars(0.0),
            pmin: Megawatts(0.1),
            pmax: Megawatts(0.5),
            qmin: Megavars(-0.2),
            qmax: Megavars(0.2),
            status: true,
            voltage_setpoint: None,
            ..Gen::default()
        }));

        // Natural Gas Generator at bus 24
        network.graph.add_node(Node::Gen(Gen {
            id: GenId::new(3),
            name: "Natural Gas Generator".to_string(),
            bus: BusId::new(24),
            active_power: Megawatts(0.4 * config.load_scale),
            reactive_power: Megavars(0.1),
            pmin: Megawatts(0.2),
            pmax: Megawatts(1.0),
            qmin: Megavars(-0.5),
            qmax: Megavars(0.5),
            status: true,
            voltage_setpoint: None,
            ..Gen::default()
        }));
    }

    // Load data: (bus_id, P_MW, Q_Mvar, name)
    // Representative distribution loads scaled for 10 MVA base
    let loads: Vec<(usize, f64, f64, &str)> = vec![
        (8, 1.5, 0.5, "Data Center"),  // High constant power
        (12, 0.8, 0.2, "EV Charging"), // Variable
        (13, 0.15, 0.05, "Residential 1"),
        (14, 0.12, 0.04, "Residential 2"),
        (15, 0.6, 0.3, "Central Cooling"), // High reactive
        (16, 0.18, 0.06, "Residential 3"),
        (17, 0.14, 0.04, "Residential 4"),
        (18, 0.45, 0.15, "Office A"),
        (19, 0.16, 0.05, "Residential 5"),
        (20, 0.13, 0.04, "Residential 6"),
        (22, 0.11, 0.03, "Residential 7"),
        (23, 0.17, 0.05, "Residential 8"),
        (25, 0.14, 0.04, "Residential 9"),
        (26, 0.12, 0.04, "Residential 10"),
        (27, 0.35, 0.12, "Office B"),
    ];

    for (i, (bus_id, p_mw, q_mvar, name)) in loads.iter().enumerate() {
        network.graph.add_node(Node::Load(Load {
            id: LoadId::new(i),
            name: name.to_string(),
            bus: BusId::new(*bus_id),
            active_power: Megawatts(*p_mw * config.load_scale),
            reactive_power: Megavars(*q_mvar * config.load_scale),
        }));
    }

    // Branch data: (from_bus, to_bus, r_pu, x_pu, b_pu, is_transformer)
    // Impedances in per-unit on configured base
    // MV lines have lower impedance, transformers have higher X/R
    let branches: Vec<(usize, usize, f64, f64, f64, bool)> = vec![
        // Substation to feeders (primary lines)
        (0, 1, 0.001, 0.010, 0.002, false),
        (0, 2, 0.001, 0.012, 0.002, false),
        (0, 3, 0.001, 0.011, 0.002, false),
        // Feeder 1 branches
        (1, 4, 0.002, 0.020, 0.001, true), // Transformer
        (1, 5, 0.002, 0.018, 0.001, true), // Transformer to PV
        (1, 6, 0.002, 0.022, 0.001, true), // Transformer
        (4, 13, 0.010, 0.030, 0.0005, false),
        (4, 14, 0.012, 0.035, 0.0005, false),
        (6, 15, 0.008, 0.025, 0.0005, false),
        // Feeder 2 branches
        (2, 7, 0.002, 0.021, 0.001, true),
        (2, 8, 0.002, 0.019, 0.001, true), // Transformer to DC
        (2, 9, 0.002, 0.020, 0.001, true),
        (7, 16, 0.011, 0.032, 0.0005, false),
        (7, 17, 0.013, 0.038, 0.0005, false),
        (9, 18, 0.009, 0.028, 0.0005, false),
        // Feeder 3 branches
        (3, 10, 0.002, 0.020, 0.001, true),
        (3, 11, 0.002, 0.022, 0.001, true),
        (3, 12, 0.002, 0.018, 0.001, true), // Transformer to EV
        (10, 19, 0.010, 0.030, 0.0005, false),
        (10, 20, 0.014, 0.040, 0.0005, false),
        (11, 21, 0.008, 0.024, 0.0005, false), // To fuel cell
        (11, 22, 0.012, 0.036, 0.0005, false),
        (11, 23, 0.015, 0.045, 0.0005, false),
        (12, 24, 0.009, 0.027, 0.0005, false), // To NG gen
        (12, 25, 0.011, 0.033, 0.0005, false),
        (12, 26, 0.013, 0.039, 0.0005, false),
        (12, 27, 0.010, 0.030, 0.0005, false),
    ];

    for (i, (from, to, r, x, b, is_xfmr)) in branches.iter().enumerate() {
        let from_idx = bus_nodes[from];
        let to_idx = bus_nodes[to];

        // Set tap ratio for transformers
        let tap = if *is_xfmr { 1.0 } else { 1.0 };

        network.graph.add_edge(
            from_idx,
            to_idx,
            Edge::Branch(Branch {
                id: BranchId::new(i),
                name: format!("{} {}-{}", if *is_xfmr { "Xfmr" } else { "Line" }, from, to),
                from_bus: BusId::new(*from),
                to_bus: BusId::new(*to),
                resistance: *r,
                reactance: *x,
                charging_b: PerUnit(*b),
                tap_ratio: tap,
                phase_shift: Radians(0.0),
                status: true,
                ..Branch::default()
            }),
        );
    }

    network
}

/// Metadata for the SoCal 28-Bus dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoCal28Metadata {
    /// Dataset name
    pub name: String,
    /// Number of buses
    pub num_buses: usize,
    /// Number of PMU sensors
    pub num_pmu_sensors: usize,
    /// Primary voltage level (kV)
    pub primary_kv: f64,
    /// Secondary voltage level (kV)
    pub secondary_kv: f64,
    /// Data start timestamp (if loaded from file)
    pub start_time: Option<i64>,
    /// Data end timestamp (if loaded from file)
    pub end_time: Option<i64>,
    /// Paper reference
    pub reference: String,
    /// Data source URL
    pub data_url: String,
}

impl Default for SoCal28Metadata {
    fn default() -> Self {
        Self {
            name: "SoCal 28-Bus Distribution Network".to_string(),
            num_buses: 28,
            num_pmu_sensors: 9,
            primary_kv: 16.5,
            secondary_kv: 0.48,
            start_time: None,
            end_time: None,
            reference: "arXiv:2504.06588".to_string(),
            data_url: "https://github.com/caltech-netlab/digital-twin-dataset".to_string(),
        }
    }
}

/// Generate synthetic PMU time-series data for the SoCal 28-Bus network
///
/// This creates representative PMU data for testing and development when
/// real data from the Caltech API is not available.
///
/// # Arguments
/// * `config` - Network configuration
/// * `duration_seconds` - Duration of the time series in seconds
/// * `sample_rate_hz` - Sample rate (typically 1.0 for phasor data)
/// * `seed` - Random seed for reproducibility
///
/// # Returns
/// A `PmuTimeSeries` with synthetic measurements
pub fn generate_synthetic_pmu_data(
    config: &SoCal28Config,
    duration_seconds: u64,
    sample_rate_hz: f64,
    seed: u64,
) -> PmuTimeSeries {
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let mut rng = StdRng::seed_from_u64(seed);
    let stations = config.pmu_stations();
    let num_samples = (duration_seconds as f64 * sample_rate_hz) as usize;

    let mut frames = Vec::with_capacity(num_samples * stations.len());

    // Base voltage and angle for each station
    let base_angles: Vec<f64> = stations
        .iter()
        .enumerate()
        .map(|(i, _)| -2.0 * (i as f64)) // Slight angle droop down the feeder
        .collect();

    for sample_idx in 0..num_samples {
        let timestamp_us = (sample_idx as i64) * (1_000_000.0 / sample_rate_hz) as i64;

        for (station_idx, station) in stations.iter().enumerate() {
            // Add small random variations
            let angle_noise = (rand::Rng::gen::<f64>(&mut rng) - 0.5) * 0.2;
            let voltage_noise = (rand::Rng::gen::<f64>(&mut rng) - 0.5) * 0.01;

            // Simulate slow voltage variations (load changes)
            let time_factor = (sample_idx as f64 * 0.001).sin() * 0.02;

            let voltage_mag = 1.0 + voltage_noise + time_factor;
            let voltage_angle = base_angles[station_idx] + angle_noise;

            frames.push(PmuFrame {
                timestamp_us,
                station_id: station.station_id.clone(),
                bus_id: station.bus_id,
                voltage_mag_pu: voltage_mag,
                voltage_angle_deg: voltage_angle,
                current_mags: vec![],
                current_angles: vec![],
                frequency_hz: 60.0 + (rand::Rng::gen::<f64>(&mut rng) - 0.5) * 0.01,
                rocof_hz_s: (rand::Rng::gen::<f64>(&mut rng) - 0.5) * 0.1,
                quality: PmuQuality::good(),
            });
        }
    }

    // Build station map
    let mut station_map = HashMap::new();
    for station in &stations {
        station_map.insert(station.station_id.clone(), station.clone());
    }

    PmuTimeSeries {
        frames,
        stations: station_map,
        nominal_frequency_hz: 60.0,
        sample_rate_fps: sample_rate_hz,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_socal28_network() {
        let config = SoCal28Config::default();
        let network = build_socal28_network(&config);

        // Should have 28 buses
        let bus_count = network
            .graph
            .node_weights()
            .filter(|n| matches!(n, Node::Bus(_)))
            .count();
        assert_eq!(bus_count, 28);

        // Should have 4 generators (1 slack + 3 DER)
        let gen_count = network
            .graph
            .node_weights()
            .filter(|n| matches!(n, Node::Gen(_)))
            .count();
        assert_eq!(gen_count, 4);

        // Should have 15 loads
        let load_count = network
            .graph
            .node_weights()
            .filter(|n| matches!(n, Node::Load(_)))
            .count();
        assert_eq!(load_count, 15);

        // Should have 27 branches (tree topology for 28 buses)
        let branch_count = network
            .graph
            .edge_weights()
            .filter(|e| matches!(e, Edge::Branch(_)))
            .count();
        assert_eq!(branch_count, 27);
    }

    #[test]
    fn test_pmu_stations() {
        let config = SoCal28Config::default();
        let stations = config.pmu_stations();

        // Should have 9 PMU stations
        assert_eq!(stations.len(), 9);

        // First station should be substation
        assert_eq!(stations[0].station_id, "SUB");
        assert_eq!(stations[0].bus_id, Some(0));

        // All stations should have bus IDs
        for station in &stations {
            assert!(station.bus_id.is_some());
        }
    }

    #[test]
    fn test_station_to_bus_map() {
        let config = SoCal28Config::default();
        let map = config.station_to_bus_map();

        assert_eq!(map.get("SUB"), Some(&0));
        assert_eq!(map.get("egauge_1"), Some(&5));
        assert_eq!(map.get("egauge_2"), Some(&8));
        assert_eq!(map.len(), 9);
    }

    #[test]
    fn test_generate_synthetic_pmu_data() {
        let config = SoCal28Config::default();
        let series = generate_synthetic_pmu_data(&config, 10, 1.0, 42);

        // Should have 10 seconds * 1 Hz * 9 stations = 90 frames
        assert_eq!(series.frames.len(), 90);

        // First frame should be at timestamp 0
        assert_eq!(series.frames[0].timestamp_us, 0);

        // Voltage should be near 1.0 p.u.
        for frame in &series.frames {
            assert!((frame.voltage_mag_pu - 1.0).abs() < 0.1);
            assert!(frame.quality.valid);
        }
    }

    #[test]
    fn test_metadata() {
        let meta = SoCal28Metadata::default();

        assert_eq!(meta.num_buses, 28);
        assert_eq!(meta.num_pmu_sensors, 9);
        assert!(meta.reference.contains("2504.06588"));
    }
}
