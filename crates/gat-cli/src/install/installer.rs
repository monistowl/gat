//! Component installation logic

use crate::install::{
    build_download_url, detect_arch, detect_os, ensure_gat_dirs, fetch_latest_release, Component,
    GatDirs,
};
use anyhow::{anyhow, Result};
use std::process::Command;

/// Install or upgrade a component
pub fn install_component(component: Component) -> Result<()> {
    let gat_dirs = ensure_gat_dirs()?;

    println!("Installing {} v0.3.1...", component);

    // Try to fetch pre-built binary
    let version = match fetch_latest_release("monistowl", "gat") {
        Ok(v) => v,
        Err(e) => {
            println!("⚠ Failed to fetch latest version: {}", e);
            println!("  Attempting source build...");
            return build_from_source(component, &gat_dirs);
        }
    };

    let os = detect_os()?;
    let arch = detect_arch()?;

    let download_url = build_download_url("monistowl", "gat", component, &version, &os, &arch);

    match download_and_extract(&download_url, &gat_dirs, component) {
        Ok(_) => {
            println!("✓ {} installed to {}", component, gat_dirs.bin.display());
            Ok(())
        }
        Err(e) => {
            println!("⚠ Binary download failed: {}", e);
            println!("  Attempting source build...");
            build_from_source(component, &gat_dirs)
        }
    }
}

/// Download and extract component binary
fn download_and_extract(url: &str, gat_dirs: &GatDirs, component: Component) -> Result<()> {
    let tmpdir = std::env::temp_dir().join(format!("gat-install-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&tmpdir)?;

    println!("Downloading from {}", url);

    let tarball = tmpdir.join("component.tar.gz");
    let status = Command::new("curl")
        .args(["-fL", url, "-o"])
        .arg(&tarball)
        .status()?;

    if !status.success() {
        std::fs::remove_dir_all(&tmpdir)?;
        return Err(anyhow!("Download failed"));
    }

    // Extract
    let extract_status = Command::new("tar")
        .args(["-xzf"])
        .arg(&tarball)
        .arg("-C")
        .arg(&tmpdir)
        .status()?;

    if !extract_status.success() {
        std::fs::remove_dir_all(&tmpdir)?;
        return Err(anyhow!("Failed to extract tarball"));
    }

    // Move binary to ~/.gat/bin/
    let binary_name = component.binary_name();
    let src_binary = tmpdir.join(binary_name);
    let dest_binary = gat_dirs.bin.join(binary_name);

    if !src_binary.exists() {
        std::fs::remove_dir_all(&tmpdir)?;
        return Err(anyhow!(
            "Binary not found in extracted archive: {}",
            binary_name
        ));
    }

    std::fs::copy(&src_binary, &dest_binary)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest_binary, std::fs::Permissions::from_mode(0o755))?;
    }

    // Cleanup
    std::fs::remove_dir_all(&tmpdir)?;

    Ok(())
}

/// Fallback: build component from source
fn build_from_source(component: Component, gat_dirs: &GatDirs) -> Result<()> {
    println!("Building {} from source...", component);

    let crate_name = match component {
        Component::Tui => "gat-tui",
        Component::Gui => "gat-gui",
        Component::Solvers => "gat-algo", // Solvers are part of gat-algo
    };

    // Find the root directory (go up from current executable)
    let exe_path = std::env::current_exe()?;
    let exe_parent = exe_path
        .parent()
        .ok_or_else(|| anyhow!("Cannot determine executable directory"))?
        .parent()
        .ok_or_else(|| anyhow!("Cannot determine root directory"))?;
    let root_dir = exe_parent.to_path_buf();

    let status = Command::new("cargo")
        .args(["build", "-p", crate_name, "--release"])
        .current_dir(&root_dir)
        .status()?;

    if !status.success() {
        return Err(anyhow!("Build failed for {}", crate_name));
    }

    // Copy binary from target/release to ~/.gat/bin/
    let binary_name = component.binary_name();
    let src = root_dir.join(format!("target/release/{}", binary_name));
    let dest = gat_dirs.bin.join(binary_name);

    if src.exists() {
        std::fs::copy(&src, &dest)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
        }
    } else {
        return Err(anyhow!("Binary not found at {}", src.display()));
    }

    println!("✓ {} built from source and installed", component);
    Ok(())
}

/// Upgrade all installed components
pub fn upgrade_all(gat_home: &std::path::Path) -> Result<()> {
    let gat_bin = gat_home.join("bin/gat");

    for component in Component::all() {
        if component.is_installed(&gat_bin) {
            println!("Upgrading {}...", component);
            install_component(*component)?;
        }
    }

    println!("✓ All components upgraded");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_binary_name_from_component() {
        // Simple smoke test that component names are usable
        let tui_name = Component::Tui.binary_name();
        assert!(!tui_name.is_empty());
        assert!(tui_name.starts_with("gat-"));
    }
}
