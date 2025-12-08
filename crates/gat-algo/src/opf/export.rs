//! Implementation of SolutionExport trait for OpfSolution
//!
//! This module provides export functionality for OpfSolution to various formats.
//! The trait is defined in gat-io to avoid circular dependencies.

use super::types::OpfSolution;
use anyhow::{Context, Result};
use std::path::Path;

// We implement the trait directly on OpfSolution here
impl OpfSolution {
    /// Export to JSON format
    pub fn to_json(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .context("serializing OpfSolution to JSON")?;
        std::fs::write(path, json)
            .with_context(|| format!("writing JSON to {}", path.display()))?;
        Ok(())
    }

    /// Convert to JSON value (for streaming/stdout)
    pub fn to_json_value(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self).context("converting OpfSolution to JSON value")
    }

    /// Export to CSV format (requires csv feature)
    #[cfg(feature = "csv")]
    pub fn to_csv(&self, path: &Path) -> Result<()> {
        let mut wtr = csv::Writer::from_path(path)
            .with_context(|| format!("creating CSV writer for {}", path.display()))?;

        // Write header
        wtr.write_record(&["generator", "p_mw", "q_mvar"])
            .context("writing CSV header")?;

        // Write generator data (sorted for deterministic output)
        let mut gen_names: Vec<_> = self.generator_p.keys().collect();
        gen_names.sort();

        for name in gen_names {
            let p = self.generator_p.get(name).copied().unwrap_or(0.0);
            let q = self.generator_q.get(name).copied().unwrap_or(0.0);
            wtr.write_record(&[name, &p.to_string(), &q.to_string()])
                .context("writing CSV record")?;
        }

        wtr.flush().context("flushing CSV writer")?;
        Ok(())
    }

    /// Export to Parquet format using Arrow (requires desktop feature)
    #[cfg(all(feature = "desktop", feature = "polars-parquet"))]
    pub fn to_parquet(&self, path: &Path) -> Result<()> {
        use polars::prelude::*;

        // Collect generator data into vectors (sorted for deterministic output)
        let mut gen_names: Vec<_> = self.generator_p.keys().collect();
        gen_names.sort();

        let names: Vec<String> = gen_names.iter().map(|s| s.to_string()).collect();
        let p_values: Vec<f64> = gen_names
            .iter()
            .map(|n| self.generator_p.get(*n).copied().unwrap_or(0.0))
            .collect();
        let q_values: Vec<f64> = gen_names
            .iter()
            .map(|n| self.generator_q.get(*n).copied().unwrap_or(0.0))
            .collect();

        // Create DataFrame
        let df = DataFrame::new(vec![
            Series::new("generator".into(), names),
            Series::new("p_mw".into(), p_values),
            Series::new("q_mvar".into(), q_values),
        ])
        .context("creating DataFrame from OpfSolution")?;

        // Write to Parquet
        let mut file = std::fs::File::create(path)
            .with_context(|| format!("creating Parquet file at {}", path.display()))?;

        ParquetWriter::new(&mut file)
            .finish(&mut df.clone())
            .context("writing DataFrame to Parquet")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_solution() -> OpfSolution {
        let mut solution = OpfSolution::default();
        solution.converged = true;
        solution.method_used = super::super::types::OpfMethod::DcOpf;
        solution.iterations = 10;
        solution.solve_time_ms = 150;
        solution.objective_value = 12345.67;

        // Add some generator data
        solution.generator_p.insert("Gen1".to_string(), 100.0);
        solution.generator_p.insert("Gen2".to_string(), 150.0);
        solution.generator_q.insert("Gen1".to_string(), 20.0);
        solution.generator_q.insert("Gen2".to_string(), 30.0);

        // Add bus voltage data
        solution.bus_voltage_mag.insert("Bus1".to_string(), 1.02);
        solution.bus_voltage_mag.insert("Bus2".to_string(), 1.01);
        solution.bus_voltage_ang.insert("Bus1".to_string(), 0.0);
        solution.bus_voltage_ang.insert("Bus2".to_string(), -2.5);

        // Add branch flow data
        solution.branch_p_flow.insert("Branch1".to_string(), 75.0);
        solution.branch_q_flow.insert("Branch1".to_string(), 15.0);

        // Add LMP data
        solution.bus_lmp.insert("Bus1".to_string(), 25.5);
        solution.bus_lmp.insert("Bus2".to_string(), 27.3);

        solution.total_losses_mw = 2.5;

        solution
    }

    #[test]
    fn test_to_json_value() {
        let solution = create_test_solution();
        let result = solution.to_json_value();

        assert!(result.is_ok(), "to_json_value should succeed");

        let json = result.unwrap();
        assert!(json.is_object(), "JSON value should be an object");
        assert!(json.get("converged").is_some(), "JSON should have converged field");
        assert!(
            json.get("objective_value").is_some(),
            "JSON should have objective_value field"
        );
        assert!(
            json.get("generator_p").is_some(),
            "JSON should have generator_p field"
        );
    }

    #[test]
    fn test_to_json_file() {
        let solution = create_test_solution();
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("solution.json");

        let result = solution.to_json(&json_path);
        assert!(result.is_ok(), "to_json should succeed");

        // Verify file was created and contains valid JSON
        assert!(json_path.exists(), "JSON file should exist");
        let content = std::fs::read_to_string(&json_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(
            parsed.get("converged").is_some(),
            "Parsed JSON should have converged field"
        );
    }

    #[test]
    #[cfg(feature = "csv")]
    fn test_to_csv_file() {
        let solution = create_test_solution();
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("solution.csv");

        let result = solution.to_csv(&csv_path);
        assert!(result.is_ok(), "to_csv should succeed");

        // Verify file was created
        assert!(csv_path.exists(), "CSV file should exist");

        // Verify CSV has headers and data
        let content = std::fs::read_to_string(&csv_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.len() > 1, "CSV should have header and data rows");
        assert!(
            lines[0].contains("generator"),
            "CSV header should contain 'generator'"
        );
    }

    #[test]
    #[cfg(all(feature = "desktop", feature = "polars-parquet"))]
    fn test_to_parquet_file() {
        let solution = create_test_solution();
        let temp_dir = TempDir::new().unwrap();
        let parquet_path = temp_dir.path().join("solution.parquet");

        let result = solution.to_parquet(&parquet_path);
        assert!(result.is_ok(), "to_parquet should succeed");

        // Verify file was created
        assert!(parquet_path.exists(), "Parquet file should exist");
        assert!(
            parquet_path.metadata().unwrap().len() > 0,
            "Parquet file should not be empty"
        );
    }
}
