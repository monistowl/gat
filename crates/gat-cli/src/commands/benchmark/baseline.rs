//! Baseline data loading for benchmark comparisons.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Load baseline objective values from a CSV file.
///
/// Expected format:
/// ```csv
/// case_name,objective
/// pglib_opf_case5_pjm,17551.89
/// pglib_opf_case14_ieee,8081.53
/// ```
pub fn load_baseline_objectives(path: &Path) -> Result<HashMap<String, f64>> {
    let mut map = HashMap::new();

    let mut reader = csv::Reader::from_path(path)
        .with_context(|| format!("opening baseline CSV: {}", path.display()))?;

    for result in reader.records() {
        let record = result.with_context(|| "reading baseline CSV record")?;

        let case_name = record
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("missing case_name column"))?
            .to_string();

        let objective: f64 = record
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("missing objective column"))?
            .parse()
            .with_context(|| format!("parsing objective for {}", case_name))?;

        map.insert(case_name, objective);
    }

    Ok(map)
}

/// Normalize case name for matching (strip extensions, lowercase)
pub fn normalize_case_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .trim_end_matches(".m")
        .trim_end_matches(".json")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_baseline_objectives() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "case_name,objective").unwrap();
        writeln!(file, "pglib_opf_case5_pjm,17551.89").unwrap();
        writeln!(file, "pglib_opf_case14_ieee,8081.53").unwrap();

        let map = load_baseline_objectives(file.path()).unwrap();

        assert_eq!(map.len(), 2);
        assert!((map["pglib_opf_case5_pjm"] - 17551.89).abs() < 0.01);
        assert!((map["pglib_opf_case14_ieee"] - 8081.53).abs() < 0.01);
    }

    #[test]
    fn test_normalize_case_name() {
        assert_eq!(
            normalize_case_name("PGLIB_OPF_CASE5_PJM.m"),
            "pglib_opf_case5_pjm"
        );
        assert_eq!(normalize_case_name("case14.json"), "case14");
        assert_eq!(normalize_case_name("  Case5  "), "case5");
    }
}
