use anyhow::Result;
use gat_viz;

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

    #[test]
    fn launch_returns_summary() {
        let summary = launch(None).unwrap();
        assert!(summary.contains("visualized"));
    }
}
