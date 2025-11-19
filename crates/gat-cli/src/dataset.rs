use std::env;
use std::fs::{self, File};
use std::io::{self, copy};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use zip::ZipArchive;

/// Return the repo-mounted folder that hosts the checked-in dataset fixtures (used by tests).
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

/// Copy a PRAS export (CSV or directory) into `out`.
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

/// Metadata for a curated public dataset entry.
/// This mirrors catalog systems such as Open Power System Data (doi:10.1016/j.energy.2017.07.039)
struct PublicDataset {
    id: &'static str,
    description: &'static str,
    url: &'static str,
    filename: &'static str,
    license: &'static str,
    tags: &'static [&'static str],
    extract: bool,
}

pub struct PublicDatasetFilter {
    pub tag: Option<String>,
    pub query: Option<String>,
}

impl Default for PublicDatasetFilter {
    fn default() -> Self {
        Self {
            tag: None,
            query: None,
        }
    }
}

const PUBLIC_DATASETS: &[PublicDataset] = &[
    PublicDataset {
        id: "opsd-time-series-2020",
        description: "Open Power System Data 60 minute single-index time series (Oct 2020 snapshot).",
        url: "https://data.open-power-system-data.org/time_series/2020-10-06/time_series_60min_singleindex.csv",
        filename: "time_series_60min_singleindex.csv",
        license: "CC-BY-SA 4.0",
        tags: &["time-series", "open-data", "europe"],
        extract: false,
    },
    PublicDataset {
        id: "airtravel",
        description: "US air travel passenger counts (classic CSV used for demos).",
        url: "https://people.sc.fsu.edu/~jburkardt/data/csv/airtravel.csv",
        filename: "airtravel.csv",
        license: "Public domain",
        tags: &["time-series", "tutorial"],
        extract: false,
    },
];

fn find_public_dataset(id: &str) -> Option<&'static PublicDataset> {
    PUBLIC_DATASETS
        .iter()
        .find(|dataset| dataset.id.eq_ignore_ascii_case(id))
}

fn dataset_matches_filter(dataset: &PublicDataset, filter: &PublicDatasetFilter) -> bool {
    if let Some(tag) = &filter.tag {
        let tag = tag.to_lowercase();
        if !dataset
            .tags
            .iter()
            .any(|t| t.eq_ignore_ascii_case(tag.as_str()))
        {
            return false;
        }
    }
    if let Some(query) = &filter.query {
        let query_lc = query.to_lowercase();
        if !dataset.id.to_lowercase().contains(&query_lc)
            && !dataset.description.to_lowercase().contains(&query_lc)
        {
            return false;
        }
    }
    true
}

/// Print the catalog contents filtered by tags/queries so users can inspect before downloading.
pub fn list_public_datasets(filter: &PublicDatasetFilter) -> Result<()> {
    let matches: Vec<_> = PUBLIC_DATASETS
        .iter()
        .filter(|dataset| dataset_matches_filter(dataset, filter))
        .collect();
    if matches.is_empty() {
        println!("No datasets matched the provided filters.");
        return Ok(());
    }
    for dataset in matches {
        println!("{id} - {desc}", id = dataset.id, desc = dataset.description);
        println!("  source : {}", dataset.url);
        println!("  license: {}", dataset.license);
        if !dataset.tags.is_empty() {
            println!("  tags   : {}", dataset.tags.join(", "));
        }
        println!();
    }
    Ok(())
}

/// Print a detailed description (source URL, tags, license) for one catalog entry.
/// This helps novices map dataset IDs to real data before fetching.
pub fn describe_public_dataset(id: &str) -> Result<()> {
    let dataset = find_public_dataset(id).ok_or_else(|| {
        let available = PUBLIC_DATASETS
            .iter()
            .map(|d| d.id)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow!(
            "Unknown dataset \"{id}\". Available ids: {available}",
            id = id,
            available = available,
        )
    })?;
    println!("Dataset: {}", dataset.id);
    println!("Description: {}", dataset.description);
    println!("Source URL: {}", dataset.url);
    println!("License: {}", dataset.license);
    println!("Filename: {}", dataset.filename);
    if !dataset.tags.is_empty() {
        println!("Tags: {}", dataset.tags.join(", "));
    }
    println!("Extract archive automatically: {}", dataset.extract);
    Ok(())
}

/// Choose a cache directory for downloaded data (prefers `~/.cache/gat/datasets`).
pub fn default_public_dataset_dir() -> PathBuf {
    if let Ok(override_dir) = env::var("GAT_PUBLIC_DATASET_DIR") {
        return PathBuf::from(override_dir);
    }
    if let Some(cache) = dirs::cache_dir() {
        return cache.join("gat").join("datasets");
    }
    PathBuf::from("data").join("public")
}

/// Download a public dataset by ID then optionally unzip it (zip detection is automatic).
pub fn fetch_public_dataset(
    id: &str,
    out_dir: Option<&Path>,
    extract: bool,
    force: bool,
) -> Result<PathBuf> {
    let dataset = find_public_dataset(id).ok_or_else(|| {
        let available = PUBLIC_DATASETS
            .iter()
            .map(|d| d.id)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow!(
            "Unknown dataset \"{id}\". Available ids: {available}",
            id = id,
            available = available,
        )
    })?;
    let staging = out_dir
        .map(|path| path.to_path_buf())
        .unwrap_or_else(default_public_dataset_dir);
    fs::create_dir_all(&staging)?;
    let dest = staging.join(dataset.filename);
    if dest.exists() {
        if force {
            fs::remove_file(&dest)?;
        } else {
            println!(
                "Dataset {} already downloaded to {}. Use --force to refresh.",
                dataset.id,
                dest.display()
            );
            return Ok(dest);
        }
    }

    download_to_path(dataset.url, &dest)?;
    println!("{} downloaded to {}", dataset.id, dest.display());

    let should_extract = extract || dataset.extract;
    if should_extract {
        if dest
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("zip"))
            .unwrap_or(false)
        {
            extract_zip_archive(&dest, &staging)?;
        } else {
            println!(
                "Skipping extraction for {}; not a zip archive.",
                dest.display()
            );
        }
    }

    Ok(dest)
}

/// Perform a simple HTTP GET and stream the response into `dest`.
fn download_to_path(url: &str, dest: &Path) -> Result<()> {
    let response = ureq::get(url)
        .call()
        .with_context(|| format!("requesting {}", url))?;
    if response.status() >= 400 {
        bail!("failed to download {}: HTTP {}", url, response.status());
    }
    let mut reader = response.into_reader();
    let mut file = File::create(dest)
        .with_context(|| format!("creating download target {}", dest.display()))?;
    copy(&mut reader, &mut file)
        .with_context(|| format!("writing dataset to {}", dest.display()))?;
    Ok(())
}

/// Extract each entry from an archive into `out_dir`.
fn extract_zip_archive(zip_path: &Path, out_dir: &Path) -> Result<()> {
    let file =
        File::open(zip_path).with_context(|| format!("opening archive {}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .with_context(|| format!("reading zip archive {}", zip_path.display()))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if let Some(name) = entry.enclosed_name() {
            let target = out_dir.join(name);
            if entry.is_dir() {
                fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut outfile = File::create(&target)?;
                io::copy(&mut entry, &mut outfile)?;
            }
        }
    }
    println!(
        "Extracted {} into {}",
        zip_path.display(),
        out_dir.display()
    );
    Ok(())
}
