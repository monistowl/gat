//! Arrow/Parquet schema definitions for GAT data tables
//!
//! This crate defines the serializable schema types used throughout GAT
//! for power system data interchange and storage.

use serde::{Deserialize, Serialize};

/// Schema definition for bus/node data in power systems
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BusSchema {
    /// Bus voltage level in kV
    pub voltage_kv: f64,
    /// Minimum voltage limit in per-unit
    pub vmin_pu: f64,
    /// Maximum voltage limit in per-unit
    pub vmax_pu: f64,
    /// Bus type: 1=PQ, 2=PV, 3=slack, 4=isolated
    pub bus_type: i32,
}

/// Schema definition for generator data
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GenSchema {
    /// Minimum active power output in MW
    pub pmin_mw: f64,
    /// Maximum active power output in MW
    pub pmax_mw: f64,
    /// Minimum reactive power output in MVAr
    pub qmin_mvar: f64,
    /// Maximum reactive power output in MVAr
    pub qmax_mvar: f64,
    /// Cost coefficient c0 (constant term)
    pub cost_c0: f64,
    /// Cost coefficient c1 (linear term)
    pub cost_c1: f64,
    /// Cost coefficient c2 (quadratic term)
    pub cost_c2: f64,
}

/// Schema definition for branch/line data
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BranchSchema {
    /// Resistance in per-unit
    pub resistance_pu: f64,
    /// Reactance in per-unit
    pub reactance_pu: f64,
    /// Total line charging susceptance in per-unit
    pub charging_b_pu: f64,
    /// Thermal rating in MVA
    pub rating_mva: f64,
    /// Tap ratio for transformers
    pub tap_ratio: f64,
    /// Phase shift in radians for transformers
    pub phase_shift_rad: f64,
}

/// Schema definition for load data
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LoadSchema {
    /// Active power demand in MW
    pub active_power_mw: f64,
    /// Reactive power demand in MVAr
    pub reactive_power_mvar: f64,
}

pub mod dist {
    /// Placeholder for future Arrow schema definitions for distribution tables.
    pub fn node_table() -> &'static str {
        "dist_nodes schema placeholder"
    }
}

pub mod derms {
    /// Placeholder for future Arrow schema definitions for DER asset tables.
    pub fn asset_table() -> &'static str {
        "der_assets schema placeholder"
    }
}

pub mod adms {
    /// Placeholder for future Arrow schema definitions for reliability tables.
    pub fn reliability_table() -> &'static str {
        "reliability schema placeholder"
    }
}
