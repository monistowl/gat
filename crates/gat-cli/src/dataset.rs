use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

fn repo_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_data/datasets")
}

pub fn fetch_rts_gmlc(out: &Path, tag: Option<&str>) -> Result<()> {
    let src_dir = repo_data_dir().join("rts-gmlc");
    if !src_dir.exists() {
        return Err(anyhow!("RTS-GMLC dataset not staged in repo"));
    }
    fs::create_dir_all(out)?;
    for file in &["grid.matpower", "timeseries.csv"] {
        let src = src_dir.join(file);
        let dst = out.join(file);
        fs::copy(&src, &dst)
            .with_context(|| format!("copying {} to {}", src.display(), dst.display()))?;
    }
    if let Some(tag) = tag {
        println!("RTS-GMLC tag {tag} staged to {}", out.display());
    }
    println!("RTS-GMLC dataset ready at {}", out.display());
    Ok(())
}

pub fn list_hiren() -> Result<Vec<String>> {
    let mut cases = Vec::new();
    let dir = repo_data_dir().join("hiren");
    for entry in fs::read_dir(dir)? {
        let ent = entry?;
        if let Some(name) = ent.path().file_stem().and_then(|s| s.to_str()) {
            cases.push(name.to_string());
        }
    }
    cases.sort();
    Ok(cases)
}

pub fn fetch_hiren(case: &str, out: &Path) -> Result<()> {
    let src = repo_data_dir()
        .join("hiren")
        .join(format!("{case}.matpower"));
    if !src.exists() {
        return Err(anyhow!("HIREN case {} not found", case));
    }
    fs::create_dir_all(out)?;
    let dst = out.join(src.file_name().unwrap());
    fs::copy(&src, &dst)?;
    println!("HIREN case {} copied to {}", case, dst.display());
    Ok(())
}

pub fn import_dsgrid(out: &Path) -> Result<()> {
    let src = repo_data_dir().join("dsgrid/demand.parquet");
    if !src.exists() {
        return Err(anyhow!("dsgrid fixture missing"));
    }
    fs::create_dir_all(out.parent().unwrap_or(out))?;
    fs::copy(&src, out)?;
    println!("dsgrid data copied to {}", out.display());
    Ok(())
}

pub fn fetch_sup3rcc(out: &Path) -> Result<()> {
    let src = repo_data_dir().join("sup3rcc/weather.parquet");
    fs::copy(&src, out)?;
    println!("Sup3rCC resource data copied to {}", out.display());
    Ok(())
}

pub fn sample_sup3rcc_grid(_grid: &Path, out: &Path) -> Result<()> {
    let src = repo_data_dir().join("sup3rcc/weather.parquet");
    fs::copy(&src, out)?;
    println!("Sup3r sample data written to {}", out.display());
    Ok(())
}

pub fn import_pras(src: &Path, out: &Path) -> Result<()> {
    let source = if src.is_dir() {
        src.join("pras.csv")
    } else {
        src.to_path_buf()
    };
    if !source.exists() {
        return Err(anyhow!("PRAS source {} missing", source.display()));
    }
    fs::copy(&source, out)?;
    println!("PRAS data copied to {}", out.display());
    Ok(())
}
