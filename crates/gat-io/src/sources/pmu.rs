//! PMU (Phasor Measurement Unit) data format importer
//!
//! This module provides types and loaders for synchrophasor data in IEEE C37.118
//! and related formats. PMU data provides high-precision synchronized measurements
//! of voltage and current phasors, frequency, and rate of change of frequency (ROCOF).
//!
//! # Supported Formats
//!
//! - **CSV**: Simple timestamped CSV with phasor columns
//! - **Parquet**: Columnar format for time-series data
//! - **JSON**: IEEE C37.118-like structured format
//!
//! # References
//!
//! - IEEE C37.118.1-2011: "Synchrophasor Measurements for Power Systems"
//! - SoCal 28-Bus Dataset: arXiv:2504.06588
//!
//! # Example
//!
//! ```no_run
//! use gat_io::sources::pmu::{load_pmu_csv, PmuFrame};
//!
//! let frames = load_pmu_csv("measurements.csv", None)?;
//! for frame in &frames {
//!     println!("Station {}: |V| = {:.4} p.u., ∠{:.2}°",
//!         frame.station_id,
//!         frame.voltage_mag_pu,
//!         frame.voltage_angle_deg);
//! }
//! ```

use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{Measurement, MeasurementType};

// ============================================================================
// PMU Data Types
// ============================================================================

/// PMU measurement frame - a single timestamped measurement from one PMU station.
///
/// Contains voltage phasor, current phasors, frequency, and quality indicators
/// following IEEE C37.118 conventions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmuFrame {
    /// Timestamp (microseconds since Unix epoch)
    pub timestamp_us: i64,

    /// PMU station identifier
    pub station_id: String,

    /// Bus ID this PMU is connected to (if known)
    pub bus_id: Option<usize>,

    /// Voltage magnitude in per-unit
    pub voltage_mag_pu: f64,

    /// Voltage angle in degrees
    pub voltage_angle_deg: f64,

    /// Current magnitude per phase (Amps or p.u.)
    /// Typically 3-phase: [Ia, Ib, Ic] or single-phase: [I]
    pub current_mags: Vec<f64>,

    /// Current angle per phase (degrees)
    pub current_angles: Vec<f64>,

    /// Frequency in Hz (nominal: 50 or 60)
    pub frequency_hz: f64,

    /// Rate of Change of Frequency (Hz/s)
    pub rocof_hz_s: f64,

    /// Data quality flags
    pub quality: PmuQuality,
}

impl Default for PmuFrame {
    fn default() -> Self {
        Self {
            timestamp_us: 0,
            station_id: String::new(),
            bus_id: None,
            voltage_mag_pu: 1.0,
            voltage_angle_deg: 0.0,
            current_mags: Vec::new(),
            current_angles: Vec::new(),
            frequency_hz: 60.0,
            rocof_hz_s: 0.0,
            quality: PmuQuality::default(),
        }
    }
}

impl PmuFrame {
    /// Create a new PMU frame with minimal data
    pub fn new(station_id: impl Into<String>, timestamp_us: i64) -> Self {
        Self {
            station_id: station_id.into(),
            timestamp_us,
            ..Default::default()
        }
    }

    /// Set voltage phasor
    pub fn with_voltage(mut self, mag_pu: f64, angle_deg: f64) -> Self {
        self.voltage_mag_pu = mag_pu;
        self.voltage_angle_deg = angle_deg;
        self
    }

    /// Set current phasor (single phase)
    pub fn with_current(mut self, mag: f64, angle_deg: f64) -> Self {
        self.current_mags = vec![mag];
        self.current_angles = vec![angle_deg];
        self
    }

    /// Set frequency measurements
    pub fn with_frequency(mut self, freq_hz: f64, rocof_hz_s: f64) -> Self {
        self.frequency_hz = freq_hz;
        self.rocof_hz_s = rocof_hz_s;
        self
    }

    /// Associate with a bus ID
    pub fn with_bus_id(mut self, bus_id: usize) -> Self {
        self.bus_id = Some(bus_id);
        self
    }
}

/// PMU data quality flags per IEEE C37.118
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PmuQuality {
    /// Data is valid and synchronized
    pub valid: bool,

    /// PMU is synchronized to UTC reference
    pub time_synchronized: bool,

    /// Data was interpolated (gap fill)
    pub interpolated: bool,

    /// Measurement is within accuracy limits
    pub accurate: bool,

    /// PMU clock error estimate (nanoseconds)
    pub clock_error_ns: Option<i64>,

    /// Total Vector Error (TVE) percentage, if available
    pub tve_percent: Option<f64>,
}

impl PmuQuality {
    /// Create quality flags indicating good data
    pub fn good() -> Self {
        Self {
            valid: true,
            time_synchronized: true,
            interpolated: false,
            accurate: true,
            clock_error_ns: Some(0),
            tve_percent: Some(0.0),
        }
    }

    /// Create quality flags indicating bad/missing data
    pub fn bad() -> Self {
        Self {
            valid: false,
            time_synchronized: false,
            interpolated: false,
            accurate: false,
            clock_error_ns: None,
            tve_percent: None,
        }
    }
}

/// Time-series of PMU measurements from multiple stations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PmuTimeSeries {
    /// Ordered list of PMU frames
    pub frames: Vec<PmuFrame>,

    /// Station metadata (station_id -> PmuStationInfo)
    pub stations: HashMap<String, PmuStationInfo>,

    /// Nominal system frequency (50 or 60 Hz)
    pub nominal_frequency_hz: f64,

    /// Sample rate in frames per second
    pub sample_rate_fps: f64,
}

impl PmuTimeSeries {
    /// Create empty time series
    pub fn new(nominal_frequency_hz: f64, sample_rate_fps: f64) -> Self {
        Self {
            frames: Vec::new(),
            stations: HashMap::new(),
            nominal_frequency_hz,
            sample_rate_fps,
        }
    }

    /// Add a frame
    pub fn push(&mut self, frame: PmuFrame) {
        self.frames.push(frame);
    }

    /// Get frames for a specific station
    pub fn frames_for_station<'a>(
        &'a self,
        station_id: &'a str,
    ) -> impl Iterator<Item = &'a PmuFrame> {
        self.frames
            .iter()
            .filter(move |f| f.station_id == station_id)
    }

    /// Get all unique timestamps
    pub fn timestamps(&self) -> Vec<i64> {
        let mut ts: Vec<i64> = self.frames.iter().map(|f| f.timestamp_us).collect();
        ts.sort();
        ts.dedup();
        ts
    }

    /// Get frames at a specific timestamp
    pub fn frames_at(&self, timestamp_us: i64) -> impl Iterator<Item = &PmuFrame> {
        self.frames
            .iter()
            .filter(move |f| f.timestamp_us == timestamp_us)
    }
}

/// Metadata about a PMU station
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PmuStationInfo {
    /// Station identifier
    pub station_id: String,

    /// Associated bus ID in the network model
    pub bus_id: Option<usize>,

    /// Station name/description
    pub name: Option<String>,

    /// Voltage transformer ratio
    pub vt_ratio: f64,

    /// Current transformer ratio
    pub ct_ratio: f64,

    /// Base voltage (kV)
    pub base_kv: f64,
}

// ============================================================================
// CSV Loader
// ============================================================================

/// CSV column mapping for PMU data
#[derive(Debug, Clone)]
pub struct PmuCsvConfig {
    /// Column name for timestamp (default: "timestamp_us")
    pub timestamp_col: String,
    /// Column name for station ID (default: "station_id")
    pub station_id_col: String,
    /// Column name for voltage magnitude (default: "voltage_mag_pu")
    pub voltage_mag_col: String,
    /// Column name for voltage angle (default: "voltage_angle_deg")
    pub voltage_angle_col: String,
    /// Column name for frequency (default: "frequency_hz")
    pub frequency_col: String,
    /// Column name for ROCOF (default: "rocof_hz_s")
    pub rocof_col: String,
    /// Whether voltage is in p.u. (true) or kV (false)
    pub voltage_is_pu: bool,
    /// Base kV for conversion if voltage_is_pu is false
    pub base_kv: f64,
}

impl Default for PmuCsvConfig {
    fn default() -> Self {
        Self {
            timestamp_col: "timestamp_us".to_string(),
            station_id_col: "station_id".to_string(),
            voltage_mag_col: "voltage_mag_pu".to_string(),
            voltage_angle_col: "voltage_angle_deg".to_string(),
            frequency_col: "frequency_hz".to_string(),
            rocof_col: "rocof_hz_s".to_string(),
            voltage_is_pu: true,
            base_kv: 1.0,
        }
    }
}

/// Load PMU data from CSV file
///
/// # Arguments
/// * `path` - Path to CSV file
/// * `config` - Optional column mapping configuration
///
/// # Returns
/// Vector of PMU frames in timestamp order
pub fn load_pmu_csv(path: impl AsRef<Path>, config: Option<PmuCsvConfig>) -> Result<Vec<PmuFrame>> {
    let config = config.unwrap_or_default();
    let file = std::fs::File::open(path.as_ref())
        .with_context(|| format!("Failed to open PMU CSV: {:?}", path.as_ref()))?;
    let reader = std::io::BufReader::new(file);

    let mut frames = Vec::new();
    let mut lines = reader.lines();

    // Parse header
    let header_line = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty CSV file"))??;
    let headers: Vec<&str> = header_line.split(',').map(|s| s.trim()).collect();

    // Find column indices
    let find_col =
        |name: &str| -> Option<usize> { headers.iter().position(|h| h.eq_ignore_ascii_case(name)) };

    let ts_idx = find_col(&config.timestamp_col);
    let station_idx = find_col(&config.station_id_col);
    let vmag_idx = find_col(&config.voltage_mag_col);
    let vang_idx = find_col(&config.voltage_angle_col);
    let freq_idx = find_col(&config.frequency_col);
    let rocof_idx = find_col(&config.rocof_col);

    // Parse data rows
    for (line_num, line_result) in lines.enumerate() {
        let line = line_result.with_context(|| format!("Error reading line {}", line_num + 2))?;
        let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

        let mut frame = PmuFrame::default();

        if let Some(idx) = ts_idx {
            if let Some(val) = cols.get(idx) {
                frame.timestamp_us = val.parse().unwrap_or(0);
            }
        }

        if let Some(idx) = station_idx {
            if let Some(val) = cols.get(idx) {
                frame.station_id = val.to_string();
            }
        }

        if let Some(idx) = vmag_idx {
            if let Some(val) = cols.get(idx) {
                let mut mag: f64 = val.parse().unwrap_or(1.0);
                if !config.voltage_is_pu && config.base_kv > 0.0 {
                    mag /= config.base_kv;
                }
                frame.voltage_mag_pu = mag;
            }
        }

        if let Some(idx) = vang_idx {
            if let Some(val) = cols.get(idx) {
                frame.voltage_angle_deg = val.parse().unwrap_or(0.0);
            }
        }

        if let Some(idx) = freq_idx {
            if let Some(val) = cols.get(idx) {
                frame.frequency_hz = val.parse().unwrap_or(60.0);
            }
        }

        if let Some(idx) = rocof_idx {
            if let Some(val) = cols.get(idx) {
                frame.rocof_hz_s = val.parse().unwrap_or(0.0);
            }
        }

        frame.quality = PmuQuality::good();
        frames.push(frame);
    }

    // Sort by timestamp
    frames.sort_by_key(|f| f.timestamp_us);

    Ok(frames)
}

// ============================================================================
// JSON Loader (IEEE C37.118-like format)
// ============================================================================

/// JSON format for PMU data (IEEE C37.118-like structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmuJsonDataset {
    /// Dataset metadata
    pub metadata: PmuDatasetMetadata,
    /// PMU station configurations
    pub stations: Vec<PmuStationInfo>,
    /// Measurement frames
    pub frames: Vec<PmuFrame>,
}

/// Dataset metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PmuDatasetMetadata {
    /// Dataset name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Source/attribution
    pub source: Option<String>,
    /// Nominal frequency (50 or 60)
    pub nominal_frequency_hz: f64,
    /// Sample rate (frames per second)
    pub sample_rate_fps: f64,
    /// Start timestamp (UTC)
    pub start_time_utc: Option<String>,
    /// End timestamp (UTC)
    pub end_time_utc: Option<String>,
}

/// Load PMU data from JSON file
pub fn load_pmu_json(path: impl AsRef<Path>) -> Result<PmuTimeSeries> {
    let file = std::fs::File::open(path.as_ref())
        .with_context(|| format!("Failed to open PMU JSON: {:?}", path.as_ref()))?;
    let reader = std::io::BufReader::new(file);

    let dataset: PmuJsonDataset =
        serde_json::from_reader(reader).context("Failed to parse PMU JSON")?;

    let mut ts = PmuTimeSeries::new(
        dataset.metadata.nominal_frequency_hz,
        dataset.metadata.sample_rate_fps,
    );

    for station in dataset.stations {
        ts.stations.insert(station.station_id.clone(), station);
    }

    ts.frames = dataset.frames;
    ts.frames.sort_by_key(|f| f.timestamp_us);

    Ok(ts)
}

// ============================================================================
// Conversion to State Estimation Measurements
// ============================================================================

/// Configuration for converting PMU frames to SE measurements
#[derive(Debug, Clone)]
pub struct PmuToSeConfig {
    /// Weight for voltage magnitude measurements
    pub voltage_mag_weight: f64,
    /// Weight for voltage angle measurements
    pub voltage_angle_weight: f64,
    /// Weight for current magnitude measurements
    pub current_mag_weight: f64,
    /// Weight for power injection measurements (derived)
    pub power_weight: f64,
    /// Use angle measurements (requires GPS sync)
    pub use_angles: bool,
}

impl Default for PmuToSeConfig {
    fn default() -> Self {
        Self {
            voltage_mag_weight: 100.0, // PMU voltage is high precision
            voltage_angle_weight: 50.0,
            current_mag_weight: 50.0,
            power_weight: 10.0,
            use_angles: true,
        }
    }
}

/// Convert PMU frames at a single timestamp to SE measurement set
///
/// # Arguments
/// * `frames` - PMU frames at the same timestamp
/// * `station_to_bus` - Mapping from station_id to bus index
/// * `config` - Conversion configuration
///
/// # Returns
/// Vector of Measurement objects for state estimation
pub fn pmu_frames_to_measurements(
    frames: &[PmuFrame],
    station_to_bus: &HashMap<String, usize>,
    config: &PmuToSeConfig,
) -> Vec<Measurement> {
    let mut measurements = Vec::new();

    for frame in frames {
        // Skip invalid frames
        if !frame.quality.valid {
            continue;
        }

        // Get bus ID
        let bus_id = frame
            .bus_id
            .or_else(|| station_to_bus.get(&frame.station_id).copied());

        if let Some(bus) = bus_id {
            // Voltage magnitude measurement
            measurements.push(Measurement {
                measurement_type: MeasurementType::Voltage,
                branch_id: None,
                bus_id: Some(bus),
                value: frame.voltage_mag_pu,
                weight: config.voltage_mag_weight,
                label: Some(format!("PMU V_mag @ bus {}", bus)),
            });

            // Voltage angle measurement (if enabled)
            if config.use_angles && frame.quality.time_synchronized {
                measurements.push(Measurement {
                    measurement_type: MeasurementType::Angle,
                    branch_id: None,
                    bus_id: Some(bus),
                    value: frame.voltage_angle_deg.to_radians(),
                    weight: config.voltage_angle_weight,
                    label: Some(format!("PMU V_ang @ bus {}", bus)),
                });
            }
        }
    }

    measurements
}

/// Convert a full PMU time series to a vector of measurement snapshots
///
/// Each snapshot contains measurements from all PMUs at that timestamp.
pub fn pmu_series_to_measurement_snapshots(
    series: &PmuTimeSeries,
    config: &PmuToSeConfig,
) -> Vec<(i64, Vec<Measurement>)> {
    let station_to_bus: HashMap<String, usize> = series
        .stations
        .iter()
        .filter_map(|(id, info)| info.bus_id.map(|b| (id.clone(), b)))
        .collect();

    let mut snapshots = Vec::new();

    for ts in series.timestamps() {
        let frames: Vec<&PmuFrame> = series.frames_at(ts).collect();
        let frame_refs: Vec<PmuFrame> = frames.iter().map(|f| (*f).clone()).collect();
        let measurements = pmu_frames_to_measurements(&frame_refs, &station_to_bus, config);

        if !measurements.is_empty() {
            snapshots.push((ts, measurements));
        }
    }

    snapshots
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmu_frame_builder() {
        let frame = PmuFrame::new("PMU_001", 1699900000_000000)
            .with_voltage(1.02, -5.3)
            .with_current(150.0, -35.3)
            .with_frequency(59.98, -0.02)
            .with_bus_id(5);

        assert_eq!(frame.station_id, "PMU_001");
        assert_eq!(frame.timestamp_us, 1699900000_000000);
        assert!((frame.voltage_mag_pu - 1.02).abs() < 1e-10);
        assert!((frame.voltage_angle_deg - (-5.3)).abs() < 1e-10);
        assert_eq!(frame.bus_id, Some(5));
    }

    #[test]
    fn test_pmu_quality_good() {
        let q = PmuQuality::good();
        assert!(q.valid);
        assert!(q.time_synchronized);
        assert!(!q.interpolated);
        assert!(q.accurate);
    }

    #[test]
    fn test_pmu_time_series() {
        let mut series = PmuTimeSeries::new(60.0, 30.0);

        series.push(
            PmuFrame::new("PMU_A", 1000)
                .with_voltage(1.01, 0.0)
                .with_bus_id(1),
        );
        series.push(
            PmuFrame::new("PMU_B", 1000)
                .with_voltage(0.99, -2.5)
                .with_bus_id(2),
        );
        series.push(
            PmuFrame::new("PMU_A", 2000)
                .with_voltage(1.02, 0.1)
                .with_bus_id(1),
        );

        let timestamps = series.timestamps();
        assert_eq!(timestamps.len(), 2);
        assert_eq!(timestamps[0], 1000);
        assert_eq!(timestamps[1], 2000);

        let at_1000: Vec<_> = series.frames_at(1000).collect();
        assert_eq!(at_1000.len(), 2);
    }

    #[test]
    fn test_pmu_to_measurements() {
        let frames = vec![
            PmuFrame::new("PMU_001", 1000)
                .with_voltage(1.02, -5.0)
                .with_bus_id(1),
            PmuFrame::new("PMU_002", 1000)
                .with_voltage(0.98, -7.5)
                .with_bus_id(2),
        ];

        // Add quality flags
        let frames: Vec<PmuFrame> = frames
            .into_iter()
            .map(|mut f| {
                f.quality = PmuQuality::good();
                f
            })
            .collect();

        let station_to_bus = HashMap::new(); // Not needed since bus_id is set
        let config = PmuToSeConfig::default();

        let measurements = pmu_frames_to_measurements(&frames, &station_to_bus, &config);

        // Should have 2 voltage mags + 2 voltage angles = 4 measurements
        assert_eq!(measurements.len(), 4);

        // Check voltage magnitude measurement
        let vmag_1 = measurements
            .iter()
            .find(|m| m.bus_id == Some(1) && matches!(m.measurement_type, MeasurementType::Voltage))
            .unwrap();
        assert!((vmag_1.value - 1.02).abs() < 1e-10);
    }
}
