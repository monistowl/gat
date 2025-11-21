use anyhow::Result;

pub fn launch(output: Option<&str>) -> Result<String> {
    if let Some(path) = output {
        let _ = std::fs::write(path, "stub visualization"); // placeholder
    }
    let summary = gat_viz::visualize_data();
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use anyhow::Result;
    use tempfile::tempdir;

    #[test]
    fn launch_returns_summary() {
        let summary = launch(None).unwrap();
        assert!(summary.contains("visualized"));
    }

    #[test]
    fn launch_writes_output_file() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("visualization.txt");
        let summary = launch(Some(path.to_str().unwrap()))?;
        assert!(summary.contains("visualized"));
        let contents = fs::read_to_string(&path)?;
        assert!(contents.contains("stub visualization"));
        Ok(())
    }
}
