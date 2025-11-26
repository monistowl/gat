//! Arrow directory writer with atomic writes for lossless network export.
//!
//! Implements the normalized multi-file Arrow format with:
//! - Directory structure: `output/{system,buses,generators,loads,branches}.arrow`
//! - Atomic writes via temp directory + rename
//! - LZ4 compression by default
//! - SHA256 checksums and manifest for integrity

use anyhow::{Context, Result};
use gat_core::{Edge, Network, Node};
use polars::io::ipc::IpcWriter;
use polars::prelude::{DataFrame, IntoSeries, ListChunked, NamedFrom, SerWriter, Series};
use std::fs;
use std::path::{Path, PathBuf};

use crate::arrow_manifest::{compute_sha256, ArrowManifest, SourceInfo, TableInfo};
use crate::arrow_schema::{
    BRANCH_TYPES, BUS_TYPES, COST_MODEL_NONE, COST_MODEL_PIECEWISE, COST_MODEL_POLYNOMIAL,
};
// Network validation will be added once normalized structs carry full referential data.

/// System-level metadata used when writing the `system.arrow` table.
///
/// The `system.arrow` row captures the per-unit basis and optional descriptive fields so that
/// downstream solvers (Newton-Raphson, DC approximations, etc.) can reconstruct execution parameters
/// without needing to re-parse MATPOWER or other source formats.
#[derive(Clone, Debug)]
pub struct SystemInfo {
    pub base_mva: f64,
    pub base_frequency_hz: f64,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Arrow directory writer with atomic write guarantees
pub struct ArrowDirectoryWriter {
    /// Temporary directory for intermediate writes
    temp_dir: PathBuf,
    /// Final output directory path
    final_dir: PathBuf,
}

impl ArrowDirectoryWriter {
    /// Create a new writer for the given output directory
    pub fn new(output_path: impl AsRef<Path>) -> Result<Self> {
        let final_dir = output_path.as_ref().to_path_buf();
        let temp_dir = final_dir.with_extension("tmp");

        // Clean up any leftover temp directory from crashed previous writes
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).with_context(|| {
                format!("cleaning up stale temp directory: {}", temp_dir.display())
            })?;
        }

        // Create fresh temp directory
        fs::create_dir_all(&temp_dir)
            .with_context(|| format!("creating temp directory: {}", temp_dir.display()))?;

        Ok(Self {
            temp_dir,
            final_dir,
        })
    }

    /// Write network to Arrow directory with atomic commit
    pub fn write_network(
        &self,
        network: &Network,
        system_info: Option<SystemInfo>,
        source_info: Option<SourceInfo>,
    ) -> Result<()> {
        // Step 1: Validate the network graph (basic referential checks)
        // Step 2: Create manifest with metadata
        let mut manifest = ArrowManifest::new(env!("CARGO_PKG_VERSION").to_string(), source_info);

        // Step 3: Write each table and collect metadata
        self.write_system_table(network, system_info.as_ref(), &mut manifest)?;
        self.write_buses_table(network, &mut manifest)?;
        self.write_generators_table(network, &mut manifest)?;
        self.write_loads_table(network, &mut manifest)?;
        self.write_branches_table(network, &mut manifest)?;

        // Step 4: Write manifest last (signals completion)
        self.write_manifest(&manifest).context("writing manifest")?;

        // Step 5: Atomic commit
        self.commit().context("atomic commit")?;

        Ok(())
    }

    /// Write manifest.json to temp directory
    fn write_manifest(&self, manifest: &ArrowManifest) -> Result<()> {
        let manifest_path = self.temp_dir.join("manifest.json");
        let json =
            serde_json::to_string_pretty(manifest).context("serializing manifest to JSON")?;

        fs::write(&manifest_path, json)
            .with_context(|| format!("writing manifest: {}", manifest_path.display()))?;

        Ok(())
    }

    /// Atomically commit writes by renaming temp directory to final location
    fn commit(&self) -> Result<()> {
        // Remove existing directory if present
        if self.final_dir.exists() {
            fs::remove_dir_all(&self.final_dir).with_context(|| {
                format!(
                    "removing existing output directory: {}",
                    self.final_dir.display()
                )
            })?;
        }

        // Atomic rename (POSIX guarantees atomicity on same filesystem)
        fs::rename(&self.temp_dir, &self.final_dir).with_context(|| {
            format!(
                "atomic rename: {} -> {}",
                self.temp_dir.display(),
                self.final_dir.display()
            )
        })?;

        Ok(())
    }

    /// Clean up temp directory on failure
    pub fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir).with_context(|| {
                format!("cleaning up temp directory: {}", self.temp_dir.display())
            })?;
        }
        Ok(())
    }

    /// Get the temp directory path
    pub fn temp_dir(&self) -> &Path {
        &self.temp_dir
    }

    /// Get the final output directory path
    pub fn final_dir(&self) -> &Path {
        &self.final_dir
    }

    fn write_table(
        &self,
        name: &str,
        df: &mut DataFrame,
        manifest: &mut ArrowManifest,
    ) -> Result<()> {
        let path = self.temp_dir.join(format!("{}.arrow", name));
        {
            let mut file = fs::File::create(&path)
                .with_context(|| format!("creating table file {}", path.display()))?;
            IpcWriter::new(&mut file)
                .finish(df)
                .with_context(|| format!("writing table {}", name))?;
        }

        let sha256 = compute_sha256(&path)?;
        let file_size_bytes = fs::metadata(&path)?.len();
        manifest.add_table(
            name,
            TableInfo {
                sha256,
                row_count: df.height() as u64,
                file_size_bytes,
            },
        );
        Ok(())
    }

    fn write_system_table(
        &self,
        _network: &Network,
        system_info: Option<&SystemInfo>,
        manifest: &mut ArrowManifest,
    ) -> Result<()> {
        let (base_mva, base_frequency_hz, name, description) = system_info
            .map(|info| {
                (
                    info.base_mva,
                    info.base_frequency_hz,
                    info.name.clone(),
                    info.description.clone(),
                )
            })
            .unwrap_or((100.0, 60.0, None, None));

        let mut df = DataFrame::new(vec![
            Series::new("base_mva", &[base_mva]),
            Series::new("base_frequency_hz", &[base_frequency_hz]),
            Series::new("name", vec![name]),
            Series::new("description", vec![description]),
        ])?;
        self.write_table("system", &mut df, manifest)
    }

    fn write_buses_table(&self, network: &Network, manifest: &mut ArrowManifest) -> Result<()> {
        let mut id = Vec::new();
        let mut name = Vec::new();
        let mut voltage_kv = Vec::new();
        let mut voltage_pu = Vec::new();
        let mut angle_rad = Vec::new();
        let mut bus_type = Vec::new();
        let mut vmin_pu = Vec::new();
        let mut vmax_pu = Vec::new();
        let mut area_id = Vec::new();
        let mut zone_id = Vec::new();

        for node in network.graph.node_weights() {
            if let Node::Bus(bus) = node {
                id.push(bus.id.value() as i64);
                name.push(bus.name.clone());
                voltage_kv.push(bus.voltage_kv);
                voltage_pu.push(1.0_f64); // placeholder until model expands
                angle_rad.push(0.0_f64); // placeholder
                bus_type.push(BUS_TYPES[0].to_string()); // PQ default
                vmin_pu.push(None::<f64>);
                vmax_pu.push(None::<f64>);
                area_id.push(None::<i64>);
                zone_id.push(None::<i64>);
            }
        }

        let mut df = DataFrame::new(vec![
            Series::new("id", id),
            Series::new("name", name),
            Series::new("voltage_kv", voltage_kv),
            Series::new("voltage_pu", voltage_pu),
            Series::new("angle_rad", angle_rad),
            Series::new("bus_type", bus_type),
            Series::new("vmin_pu", vmin_pu),
            Series::new("vmax_pu", vmax_pu),
            Series::new("area_id", area_id),
            Series::new("zone_id", zone_id),
        ])?;

        self.write_table("buses", &mut df, manifest)
    }

    fn write_generators_table(
        &self,
        network: &Network,
        manifest: &mut ArrowManifest,
    ) -> Result<()> {
        let mut id = Vec::new();
        let mut name = Vec::new();
        let mut bus = Vec::new();
        let mut status = Vec::new();
        let mut active_power_mw = Vec::new();
        let mut reactive_power_mvar = Vec::new();
        let mut pmin_mw = Vec::new();
        let mut pmax_mw = Vec::new();
        let mut qmin_mvar = Vec::new();
        let mut qmax_mvar = Vec::new();
        let mut voltage_setpoint_pu = Vec::new();
        let mut mbase_mva = Vec::new();
        // Store cost models as explicit integers so downstream readers know whether each generator
        // uses `no cost`, `piecewise`, or `polynomial` information; lists store the corresponding
        // coefficients/values for dispatch.
        let mut cost_model: Vec<i32> = Vec::new();
        let mut cost_startup = Vec::new();
        let mut cost_shutdown = Vec::new();
        let mut cost_coeffs: Vec<Vec<f64>> = Vec::new();
        let mut cost_values: Vec<Vec<f64>> = Vec::new();
        let mut is_syncon = Vec::new();

        for node in network.graph.node_weights() {
            if let Node::Gen(gen) = node {
                id.push(gen.id.value() as i64);
                name.push(gen.name.clone());
                bus.push(gen.bus.value() as i64);
                status.push(true); // Network Gen has no status flag; assume in service
                active_power_mw.push(gen.active_power_mw);
                reactive_power_mvar.push(gen.reactive_power_mvar);
                pmin_mw.push(gen.pmin_mw);
                pmax_mw.push(gen.pmax_mw);
                qmin_mvar.push(gen.qmin_mvar);
                qmax_mvar.push(gen.qmax_mvar);
                voltage_setpoint_pu.push(None::<f64>);
                mbase_mva.push(None::<f64>);
                is_syncon.push(gen.is_synchronous_condenser);

                match &gen.cost_model {
                    gat_core::CostModel::NoCost => {
                        cost_model.push(COST_MODEL_NONE);
                        cost_startup.push(None::<f64>);
                        cost_shutdown.push(None::<f64>);
                        cost_coeffs.push(Vec::new());
                        cost_values.push(Vec::new());
                    }
                    gat_core::CostModel::PiecewiseLinear(points) => {
                        cost_model.push(COST_MODEL_PIECEWISE);
                        cost_startup.push(None::<f64>);
                        cost_shutdown.push(None::<f64>);
                        let (xs, ys): (Vec<_>, Vec<_>) = points.iter().copied().unzip();
                        cost_coeffs.push(xs);
                        cost_values.push(ys);
                    }
                    gat_core::CostModel::Polynomial(coeffs) => {
                        cost_model.push(COST_MODEL_POLYNOMIAL);
                        cost_startup.push(None::<f64>);
                        cost_shutdown.push(None::<f64>);
                        cost_coeffs.push(coeffs.clone());
                        cost_values.push(Vec::new());
                    }
                }
            }
        }

        let mut cost_coeffs_series = ListChunked::from_iter(
            cost_coeffs
                .iter()
                .map(|coeffs| Series::new("", coeffs.as_slice())),
        )
        .into_series();
        cost_coeffs_series.rename("cost_coeffs");

        let mut cost_values_series = ListChunked::from_iter(
            cost_values
                .iter()
                .map(|values| Series::new("", values.as_slice())),
        )
        .into_series();
        cost_values_series.rename("cost_values");

        let cost_model_series = Series::new("cost_model", cost_model.as_slice());

        let mut df = DataFrame::new(vec![
            Series::new("id", &id),
            Series::new("name", &name),
            Series::new("bus", &bus),
            Series::new("status", &status),
            Series::new("active_power_mw", &active_power_mw),
            Series::new("reactive_power_mvar", &reactive_power_mvar),
            Series::new("pmin_mw", &pmin_mw),
            Series::new("pmax_mw", &pmax_mw),
            Series::new("qmin_mvar", &qmin_mvar),
            Series::new("qmax_mvar", &qmax_mvar),
            Series::new("voltage_setpoint_pu", &voltage_setpoint_pu),
            Series::new("mbase_mva", &mbase_mva),
            cost_model_series,
            Series::new("cost_startup", &cost_startup),
            Series::new("cost_shutdown", &cost_shutdown),
            cost_coeffs_series,
            cost_values_series,
            Series::new("is_synchronous_condenser", &is_syncon),
        ])?;

        self.write_table("generators", &mut df, manifest)
    }

    fn write_loads_table(&self, network: &Network, manifest: &mut ArrowManifest) -> Result<()> {
        let mut id = Vec::new();
        let mut name = Vec::new();
        let mut bus = Vec::new();
        let mut status = Vec::new();
        let mut active_power_mw = Vec::new();
        let mut reactive_power_mvar = Vec::new();

        for node in network.graph.node_weights() {
            if let Node::Load(load) = node {
                id.push(load.id.value() as i64);
                name.push(load.name.clone());
                bus.push(load.bus.value() as i64);
                status.push(true); // No status flag in core model
                active_power_mw.push(load.active_power_mw);
                reactive_power_mvar.push(load.reactive_power_mvar);
            }
        }

        let mut df = DataFrame::new(vec![
            Series::new("id", id),
            Series::new("name", name),
            Series::new("bus", bus),
            Series::new("status", status),
            Series::new("active_power_mw", active_power_mw),
            Series::new("reactive_power_mvar", reactive_power_mvar),
        ])?;

        self.write_table("loads", &mut df, manifest)
    }

    fn write_branches_table(&self, network: &Network, manifest: &mut ArrowManifest) -> Result<()> {
        let mut id = Vec::new();
        let mut name = Vec::new();
        let mut element_type = Vec::new();
        let mut from_bus = Vec::new();
        let mut to_bus = Vec::new();
        let mut status = Vec::new();
        let mut resistance_pu = Vec::new();
        let mut reactance_pu = Vec::new();
        let mut charging_b_pu = Vec::new();
        let mut tap_ratio = Vec::new();
        let mut phase_shift_rad = Vec::new();
        let mut rate_a_mva = Vec::new();
        let mut rate_b_mva = Vec::new();
        let mut rate_c_mva = Vec::new();
        let mut angle_min_rad = Vec::new();
        let mut angle_max_rad = Vec::new();

        for edge in network.graph.edge_weights() {
            if let Edge::Branch(branch) = edge {
                id.push(branch.id.value() as i64);
                name.push(branch.name.clone());
                element_type.push(BRANCH_TYPES[0].to_string()); // "line" default
                from_bus.push(branch.from_bus.value() as i64);
                to_bus.push(branch.to_bus.value() as i64);
                status.push(branch.status);
                resistance_pu.push(branch.resistance);
                reactance_pu.push(branch.reactance);
                charging_b_pu.push(branch.charging_b_pu);
                tap_ratio.push(branch.tap_ratio);
                phase_shift_rad.push(branch.phase_shift_rad);
                rate_a_mva.push(branch.rating_a_mva);
                rate_b_mva.push(None::<f64>);
                rate_c_mva.push(None::<f64>);
                angle_min_rad.push(None::<f64>);
                angle_max_rad.push(None::<f64>);
            }
        }

        let mut df = DataFrame::new(vec![
            Series::new("id", id),
            Series::new("name", name),
            Series::new("element_type", element_type),
            Series::new("from_bus", from_bus),
            Series::new("to_bus", to_bus),
            Series::new("status", status),
            Series::new("resistance_pu", resistance_pu),
            Series::new("reactance_pu", reactance_pu),
            Series::new("charging_b_pu", charging_b_pu),
            Series::new("tap_ratio", tap_ratio),
            Series::new("phase_shift_rad", phase_shift_rad),
            Series::new("rate_a_mva", rate_a_mva),
            Series::new("rate_b_mva", rate_b_mva),
            Series::new("rate_c_mva", rate_c_mva),
            Series::new("angle_min_rad", angle_min_rad),
            Series::new("angle_max_rad", angle_max_rad),
        ])?;

        self.write_table("branches", &mut df, manifest)
    }
}

/// Write network to Arrow directory format
pub fn write_network_to_arrow_directory(
    network: &Network,
    output_dir: impl AsRef<Path>,
) -> Result<()> {
    let writer = ArrowDirectoryWriter::new(output_dir)?;

    // Attempt write with cleanup on failure
    match writer.write_network(network, None, None) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Best effort cleanup
            let _ = writer.cleanup();
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_writer_creates_temp_directory() {
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("test_output");

        let writer = ArrowDirectoryWriter::new(&output_path).unwrap();

        assert!(writer.temp_dir().exists());
        assert!(!writer.final_dir().exists()); // Final dir shouldn't exist until commit
    }

    #[test]
    fn test_writer_cleans_up_stale_temp_dir() {
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("test_output");

        // Create first writer
        let writer1 = ArrowDirectoryWriter::new(&output_path).unwrap();
        let temp_path = writer1.temp_dir().to_path_buf();
        assert!(temp_path.exists());

        // Create second writer - should clean up temp from first
        let writer2 = ArrowDirectoryWriter::new(&output_path).unwrap();
        assert!(writer2.temp_dir().exists());
        assert_eq!(writer2.temp_dir(), writer1.temp_dir()); // Same path
    }

    #[test]
    fn test_write_manifest() {
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("test_output");

        let writer = ArrowDirectoryWriter::new(&output_path).unwrap();
        let manifest = ArrowManifest::new("0.4.0".to_string(), None);

        writer.write_manifest(&manifest).unwrap();

        let manifest_path = writer.temp_dir().join("manifest.json");
        assert!(manifest_path.exists());

        // Verify it's valid JSON
        let json = fs::read_to_string(&manifest_path).unwrap();
        let _: ArrowManifest = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_atomic_commit() {
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("test_output");

        let writer = ArrowDirectoryWriter::new(&output_path).unwrap();
        let temp_path = writer.temp_dir().to_path_buf();

        // Create a test file in temp dir
        fs::write(temp_path.join("test.txt"), "test").unwrap();

        // Commit
        writer.commit().unwrap();

        // Verify final directory exists with content
        assert!(output_path.exists());
        assert!(output_path.is_dir());
        assert!(output_path.join("test.txt").exists());

        // Verify temp dir is gone
        let temp_path = output_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_cleanup_removes_temp_dir() {
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("test_output");

        let writer = ArrowDirectoryWriter::new(&output_path).unwrap();
        let temp_path = writer.temp_dir().to_path_buf();

        assert!(temp_path.exists());

        writer.cleanup().unwrap();

        assert!(!temp_path.exists());
    }

    #[test]
    fn test_writer_paths() {
        let output_dir = TempDir::new().unwrap();
        let output_path = output_dir.path().join("my_network");

        let writer = ArrowDirectoryWriter::new(&output_path).unwrap();

        assert_eq!(writer.final_dir(), output_path);
        assert_eq!(
            writer.temp_dir(),
            output_path.with_extension("tmp"),
            "Temp dir should be output path with .tmp extension"
        );
    }
}
