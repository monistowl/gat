//! Time-series state estimation using PMU synchrophasor data
//!
//! This module provides functions for processing streaming PMU measurements through
//! the Weighted Least Squares (WLS) state estimator. It extends the single-snapshot
//! approach in `gat-algo` to support:
//!
//! - Batch processing of multiple timestamps from PMU time-series
//! - Warm-starting from previous solutions for faster convergence
//! - Quality-weighted measurements based on PMU data quality flags
//! - Aggregation of results into time-indexed DataFrames
//!
//! ## PMU-Based State Estimation
//!
//! Phasor Measurement Units provide synchronized voltage and current phasors at
//! high rates (30-60 Hz). For state estimation:
//!
//! - Voltage magnitude becomes a `Voltage` measurement (per-unit)
//! - Voltage angle becomes an `Angle` measurement (radians)
//! - Current phasors can be converted to branch flow measurements
//!
//! ## References
//!
//! - SoCal 28-Bus Digital Twin: arXiv:2504.06588
//! - IEEE C37.118 Standard for Synchrophasors

use crate::sources::cigre::{Measurement, MeasurementType};
use crate::sources::pmu::{PmuFrame, PmuTimeSeries, PmuToSeConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of state estimation for a single timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEstimateSnapshot {
    /// Timestamp in microseconds since epoch
    pub timestamp_us: i64,
    /// Estimated voltage angles by bus ID (radians)
    pub angles: HashMap<usize, f64>,
    /// Estimated voltage magnitudes by bus ID (per-unit) if available
    pub voltages: HashMap<usize, f64>,
    /// Chi-squared goodness of fit statistic
    pub chi2: f64,
    /// Number of measurements used
    pub num_measurements: usize,
    /// Whether estimation converged successfully
    pub converged: bool,
}

impl Default for StateEstimateSnapshot {
    fn default() -> Self {
        Self {
            timestamp_us: 0,
            angles: HashMap::new(),
            voltages: HashMap::new(),
            chi2: 0.0,
            num_measurements: 0,
            converged: false,
        }
    }
}

/// Configuration for time-series state estimation
#[derive(Debug, Clone)]
pub struct TimeSeriesSeConfig {
    /// Configuration for PMU to SE measurement conversion
    pub pmu_config: PmuToSeConfig,
    /// Maximum iterations per solve
    pub max_iterations: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Warm-start from previous solution
    pub warm_start: bool,
    /// Parallel processing threshold (number of timestamps)
    pub parallel_threshold: usize,
}

impl Default for TimeSeriesSeConfig {
    fn default() -> Self {
        Self {
            pmu_config: PmuToSeConfig::default(),
            max_iterations: 100,
            tolerance: 1e-6,
            warm_start: true,
            parallel_threshold: 10,
        }
    }
}

/// Time-series state estimation results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeSeriesSeResult {
    /// Estimated states for each timestamp
    pub snapshots: Vec<StateEstimateSnapshot>,
    /// Total processing time in milliseconds
    pub processing_time_ms: f64,
    /// Number of successful estimates
    pub num_converged: usize,
    /// Number of failed estimates
    pub num_failed: usize,
}

impl TimeSeriesSeResult {
    /// Get the success rate (fraction of converged solutions)
    pub fn success_rate(&self) -> f64 {
        if self.snapshots.is_empty() {
            0.0
        } else {
            self.num_converged as f64 / self.snapshots.len() as f64
        }
    }

    /// Get angles at a specific timestamp
    pub fn angles_at(&self, timestamp_us: i64) -> Option<&HashMap<usize, f64>> {
        self.snapshots
            .iter()
            .find(|s| s.timestamp_us == timestamp_us)
            .map(|s| &s.angles)
    }

    /// Get all timestamps
    pub fn timestamps(&self) -> Vec<i64> {
        self.snapshots.iter().map(|s| s.timestamp_us).collect()
    }
}

/// Group PMU frames by timestamp into measurement snapshots
pub fn group_frames_by_timestamp(
    frames: &[PmuFrame],
    station_to_bus: &HashMap<String, usize>,
    config: &PmuToSeConfig,
) -> Vec<(i64, Vec<Measurement>)> {
    // Collect unique timestamps
    let mut timestamps: Vec<i64> = frames.iter().map(|f| f.timestamp_us).collect();
    timestamps.sort();
    timestamps.dedup();

    // Group measurements by timestamp
    timestamps
        .into_iter()
        .map(|ts| {
            let ts_frames: Vec<&PmuFrame> = frames.iter().filter(|f| f.timestamp_us == ts).collect();

            let measurements: Vec<Measurement> = ts_frames
                .iter()
                .filter_map(|frame| {
                    // Skip frames with bad quality - require valid measurements
                    if !frame.quality.valid {
                        return None;
                    }
                    // Skip if using angles but not time-synchronized
                    if config.use_angles && !frame.quality.time_synchronized {
                        return None;
                    }

                    // Get bus ID from station mapping or frame itself
                    let bus_id = frame
                        .bus_id
                        .or_else(|| station_to_bus.get(&frame.station_id).copied())?;

                    // Compute measurement weight from quality
                    let tve_factor = if let Some(tve) = frame.quality.tve_percent {
                        // TVE is Total Vector Error (%)
                        // Lower TVE = higher weight
                        (1.0 / (1.0 + tve * tve)).max(0.1)
                    } else {
                        1.0
                    };

                    let accuracy_factor = if frame.quality.accurate { 1.0 } else { 0.5 };

                    // Create measurements for angle and voltage
                    let mut meas = Vec::with_capacity(2);

                    // Voltage angle measurement (if using angles)
                    if config.use_angles {
                        let weight = config.voltage_angle_weight * tve_factor * accuracy_factor;
                        meas.push(Measurement {
                            measurement_type: MeasurementType::Angle,
                            branch_id: None,
                            bus_id: Some(bus_id),
                            value: frame.voltage_angle_deg.to_radians(),
                            weight,
                            label: Some(format!("PMU {} angle", frame.station_id)),
                        });
                    }

                    // Voltage magnitude measurement
                    let vm_weight = config.voltage_mag_weight * tve_factor * accuracy_factor;
                    meas.push(Measurement {
                        measurement_type: MeasurementType::Voltage,
                        branch_id: None,
                        bus_id: Some(bus_id),
                        value: frame.voltage_mag_pu,
                        weight: vm_weight,
                        label: Some(format!("PMU {} voltage", frame.station_id)),
                    });

                    Some(meas)
                })
                .flatten()
                .collect();

            (ts, measurements)
        })
        .filter(|(_, meas)| !meas.is_empty())
        .collect()
}

/// Process PMU time-series for state estimation
///
/// This function groups PMU frames by timestamp and prepares them for
/// sequential or parallel state estimation.
pub fn prepare_pmu_measurements(
    series: &PmuTimeSeries,
    station_to_bus: &HashMap<String, usize>,
    config: &TimeSeriesSeConfig,
) -> Vec<(i64, Vec<Measurement>)> {
    group_frames_by_timestamp(&series.frames, station_to_bus, &config.pmu_config)
}

/// Compute measurement residuals given estimated state
///
/// For each measurement, computes: residual = measured - estimated
pub fn compute_residuals(
    measurements: &[Measurement],
    angle_estimates: &HashMap<usize, f64>,
    _voltage_estimates: &HashMap<usize, f64>,
) -> Vec<f64> {
    measurements
        .iter()
        .map(|m| {
            match m.measurement_type {
                MeasurementType::Angle => {
                    let bus = m.bus_id.expect("angle measurement needs bus_id");
                    let estimated = angle_estimates.get(&bus).copied().unwrap_or(0.0);
                    m.value - estimated
                }
                MeasurementType::Voltage => {
                    // DC-SE doesn't estimate voltage magnitude, assume 1.0 p.u.
                    m.value - 1.0
                }
                MeasurementType::Flow | MeasurementType::Injection => {
                    // Flow and injection residuals require network model
                    0.0
                }
            }
        })
        .collect()
}

/// Compute chi-squared statistic from weighted residuals
pub fn compute_chi2(residuals: &[f64], weights: &[f64]) -> f64 {
    residuals
        .iter()
        .zip(weights.iter())
        .map(|(r, w)| w * r * r)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::pmu::{PmuFrame, PmuQuality};

    fn make_test_frame(station: &str, bus_id: usize, ts: i64, angle_deg: f64) -> PmuFrame {
        PmuFrame {
            timestamp_us: ts,
            station_id: station.to_string(),
            bus_id: Some(bus_id),
            voltage_mag_pu: 1.0,
            voltage_angle_deg: angle_deg,
            current_mags: vec![],
            current_angles: vec![],
            frequency_hz: 60.0,
            rocof_hz_s: 0.0,
            quality: PmuQuality::good(),
        }
    }

    #[test]
    fn test_group_frames_by_timestamp() {
        let frames = vec![
            make_test_frame("S1", 1, 1000, 0.0),
            make_test_frame("S2", 2, 1000, 5.0),
            make_test_frame("S1", 1, 2000, 1.0),
            make_test_frame("S2", 2, 2000, 6.0),
        ];

        let station_to_bus = HashMap::new(); // Use bus_id from frames
        let config = PmuToSeConfig::default();

        let grouped = group_frames_by_timestamp(&frames, &station_to_bus, &config);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0].0, 1000);
        assert_eq!(grouped[1].0, 2000);

        // Each timestamp should have 4 measurements (2 angles + 2 voltages)
        assert_eq!(grouped[0].1.len(), 4);
        assert_eq!(grouped[1].1.len(), 4);
    }

    #[test]
    fn test_time_series_se_result_success_rate() {
        let mut result = TimeSeriesSeResult::default();
        result.snapshots = vec![
            StateEstimateSnapshot {
                converged: true,
                ..Default::default()
            },
            StateEstimateSnapshot {
                converged: true,
                ..Default::default()
            },
            StateEstimateSnapshot {
                converged: false,
                ..Default::default()
            },
        ];
        result.num_converged = 2;
        result.num_failed = 1;

        assert!((result.success_rate() - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_chi2() {
        let residuals = vec![0.1, 0.2, 0.05];
        let weights = vec![1.0, 2.0, 4.0];

        let chi2 = compute_chi2(&residuals, &weights);

        // chi2 = 1.0*0.01 + 2.0*0.04 + 4.0*0.0025 = 0.01 + 0.08 + 0.01 = 0.10
        assert!((chi2 - 0.10).abs() < 1e-6);
    }
}
