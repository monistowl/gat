//! GitHub API client for fetching releases

use crate::install::Component;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

/// GitHub release response (subset of fields we need)
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Fetch latest release info from GitHub API using curl (without shell).
///
/// This function invokes curl directly to avoid shell command injection.
/// JSON parsing is done with serde instead of piping to jq.
pub fn fetch_latest_release(repo_owner: &str, repo_name: &str) -> Result<String> {
    let api_url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        repo_owner, repo_name
    );

    // Invoke curl directly without shell to prevent command injection
    let output = std::process::Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "Accept: application/vnd.github.v3+json",
            &api_url,
        ])
        .output()
        .context("Failed to run curl")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to fetch releases from GitHub: {}", stderr));
    }

    // Parse JSON with serde instead of piping to jq
    let release: GitHubRelease =
        serde_json::from_slice(&output.stdout).context("Failed to parse GitHub release JSON")?;

    if release.tag_name.is_empty() {
        return Err(anyhow!("No tag_name in GitHub release response"));
    }

    Ok(release.tag_name)
}

/// Build download URL for a release component
pub fn build_download_url(
    repo_owner: &str,
    repo_name: &str,
    component: Component,
    version: &str,
    os: &str,
    arch: &str,
) -> String {
    let artifact_name = format!(
        "{}-{}-{}-{}.tar.gz",
        component.artifact_prefix(),
        version,
        os,
        arch
    );
    format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        repo_owner, repo_name, version, artifact_name
    )
}

/// Detect operating system
pub fn detect_os() -> Result<String> {
    let output = std::process::Command::new("uname").arg("-s").output()?;

    let os = String::from_utf8(output.stdout)?.trim().to_lowercase();

    match os.as_str() {
        "linux" => Ok("linux".to_string()),
        "darwin" => Ok("macos".to_string()),
        _ => Err(anyhow!("Unsupported OS: {}", os)),
    }
}

/// Detect CPU architecture
pub fn detect_arch() -> Result<String> {
    let output = std::process::Command::new("uname").arg("-m").output()?;

    let arch = String::from_utf8(output.stdout)?.trim().to_string();

    match arch.as_str() {
        "x86_64" | "amd64" => Ok("x86_64".to_string()),
        "arm64" | "aarch64" => Ok("arm64".to_string()),
        _ => Err(anyhow!("Unsupported architecture: {}", arch)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_download_url() {
        let url = build_download_url(
            "monistowl",
            "gat",
            Component::Tui,
            "v0.3.1",
            "linux",
            "x86_64",
        );
        assert!(url.contains("gat-tui-v0.3.1-linux-x86_64.tar.gz"));
        assert!(url.contains("github.com/monistowl/gat"));
    }

    #[test]
    fn test_build_download_url_solvers() {
        let url = build_download_url(
            "monistowl",
            "gat",
            Component::Solvers,
            "v0.3.1",
            "macos",
            "arm64",
        );
        assert!(url.contains("gat-solvers-v0.3.1-macos-arm64.tar.gz"));
    }

    #[test]
    fn test_detect_os() {
        let os = detect_os().unwrap();
        assert!(os == "linux" || os == "macos");
    }

    #[test]
    fn test_detect_arch() {
        let arch = detect_arch().unwrap();
        assert!(arch == "x86_64" || arch == "arm64");
    }
}
