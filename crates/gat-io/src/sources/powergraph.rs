//! PowerGraph GNN Benchmark Dataset Loader
//!
//! This module loads the PowerGraph benchmark dataset from NeurIPS 2024:
//! "PowerGraph: A power grid benchmark dataset for graph neural networks"
//!
//! Dataset source: <https://github.com/PowerGraph-Datasets>
//! Paper: <https://arxiv.org/abs/2402.02827>
//!
//! The dataset uses MATLAB `.mat` files containing:
//! - `Bf.mat`: Node features (P_net, S_net, V)
//! - `Ef.mat`: Edge features (P_flow, Q_flow, reactance, line_rating)
//! - `blist.mat`: Edge connectivity (from, to indices)
//! - `of_bi.mat`: Binary classification labels
//! - `of_reg.mat`: Regression labels
//! - `of_mc.mat`: Multi-class labels
//! - `exp.mat`: Explainability ground truth

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[cfg(feature = "powergraph")]
use matfile::MatFile;

/// A single graph sample from the PowerGraph benchmark.
///
/// Represents one power grid state with node features, edge connectivity,
/// edge features, and optional labels for various tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerGraphSample {
    /// Number of nodes (buses) in this graph
    pub num_nodes: usize,
    /// Number of edges (branches) in this graph
    pub num_edges: usize,
    /// Node feature matrix [num_nodes, 3]: P_net, S_net, V
    pub node_features: Vec<[f64; 3]>,
    /// Edge index list [(from, to), ...]
    pub edge_index: Vec<(usize, usize)>,
    /// Edge feature matrix [num_edges, 4]: P_flow, Q_flow, reactance, line_rating
    pub edge_features: Vec<[f64; 4]>,
    /// Binary classification label (optional)
    pub label_binary: Option<bool>,
    /// Regression target (optional)
    pub label_regression: Option<f64>,
    /// Multi-class label (optional)
    pub label_multiclass: Option<usize>,
    /// Explainability mask: edges involved in cascading failure (optional)
    pub explanation_mask: Option<Vec<bool>>,
}

/// Node feature indices for PowerGraph format
#[derive(Debug, Clone, Copy)]
pub struct NodeFeatureSpec {
    /// Net active power at bus (P_net) in MW
    pub p_net: usize,
    /// Net apparent power at bus (S_net) in MVA
    pub s_net: usize,
    /// Voltage magnitude (V) in p.u.
    pub voltage: usize,
}

impl Default for NodeFeatureSpec {
    fn default() -> Self {
        Self {
            p_net: 0,
            s_net: 1,
            voltage: 2,
        }
    }
}

/// Edge feature indices for PowerGraph format
#[derive(Debug, Clone, Copy)]
pub struct EdgeFeatureSpec {
    /// Active power flow (P_i,j) in MW
    pub p_flow: usize,
    /// Reactive power flow (Q_i,j) in MVAr
    pub q_flow: usize,
    /// Line reactance (X_i,j) in p.u.
    pub reactance: usize,
    /// Line rating (lr_i,j) in MVA
    pub line_rating: usize,
}

impl Default for EdgeFeatureSpec {
    fn default() -> Self {
        Self {
            p_flow: 0,
            q_flow: 1,
            reactance: 2,
            line_rating: 3,
        }
    }
}

/// Information about a PowerGraph dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerGraphDatasetInfo {
    /// Dataset name (e.g., "ieee24", "ieee39", "ieee118", "uk")
    pub name: String,
    /// Number of samples in the dataset
    pub num_samples: usize,
    /// Number of nodes per graph (fixed for each dataset)
    pub num_nodes: usize,
    /// Number of edges per graph (fixed for each dataset)
    pub num_edges: usize,
    /// Whether binary labels are available
    pub has_binary_labels: bool,
    /// Whether regression labels are available
    pub has_regression_labels: bool,
    /// Whether multi-class labels are available
    pub has_multiclass_labels: bool,
    /// Whether explainability masks are available
    pub has_explanations: bool,
}

/// Task type for PowerGraph benchmark
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerGraphTask {
    /// Binary classification (cascading failure detection)
    Binary,
    /// Regression (power flow prediction)
    Regression,
    /// Multi-class classification (failure severity)
    MultiClass,
}

/// List available PowerGraph datasets in a directory.
///
/// Scans the directory for valid PowerGraph dataset folders (containing required .mat files).
///
/// # Arguments
/// * `root` - Root directory containing dataset folders (ieee24, ieee39, etc.)
///
/// # Returns
/// Vector of dataset info for each valid dataset found
#[cfg(feature = "powergraph")]
pub fn list_powergraph_datasets(root: &Path) -> Result<Vec<PowerGraphDatasetInfo>> {
    let known_datasets = ["ieee24", "ieee39", "ieee118", "uk"];
    let mut datasets = Vec::new();

    for name in &known_datasets {
        let dataset_path = root.join(name);
        if dataset_path.exists() && dataset_path.is_dir() {
            match get_dataset_info(&dataset_path, name) {
                Ok(info) => datasets.push(info),
                Err(e) => {
                    eprintln!("Warning: Could not load dataset '{}': {}", name, e);
                }
            }
        }
    }

    Ok(datasets)
}

/// Get information about a specific PowerGraph dataset.
#[cfg(feature = "powergraph")]
fn get_dataset_info(path: &Path, name: &str) -> Result<PowerGraphDatasetInfo> {
    // Check for required files
    let raw_path = path.join("raw");
    let base_path = if raw_path.exists() { raw_path } else { path.to_path_buf() };

    let bf_path = base_path.join("Bf.mat");
    let blist_path = base_path.join("blist.mat");

    if !bf_path.exists() {
        return Err(anyhow!("Missing Bf.mat (node features)"));
    }
    if !blist_path.exists() {
        return Err(anyhow!("Missing blist.mat (edge list)"));
    }

    // Load node features to get dimensions
    let bf_file = MatFile::parse(std::fs::File::open(&bf_path)?)
        .context("Failed to parse Bf.mat")?;

    let node_array = bf_file
        .find_by_name("Bf")
        .ok_or_else(|| anyhow!("Bf array not found in Bf.mat"))?;

    let (num_samples, num_nodes, _num_features) = get_3d_dimensions(node_array)?;

    // Load edge list to get edge count
    let blist_file = MatFile::parse(std::fs::File::open(&blist_path)?)
        .context("Failed to parse blist.mat")?;

    let edge_array = blist_file
        .find_by_name("blist")
        .ok_or_else(|| anyhow!("blist array not found in blist.mat"))?;

    let num_edges = get_edge_count(edge_array)?;

    // Check for optional label files
    let has_binary_labels = base_path.join("of_bi.mat").exists();
    let has_regression_labels = base_path.join("of_reg.mat").exists();
    let has_multiclass_labels = base_path.join("of_mc.mat").exists();
    let has_explanations = base_path.join("exp.mat").exists();

    Ok(PowerGraphDatasetInfo {
        name: name.to_string(),
        num_samples,
        num_nodes,
        num_edges,
        has_binary_labels,
        has_regression_labels,
        has_multiclass_labels,
        has_explanations,
    })
}

/// Load a PowerGraph dataset.
///
/// # Arguments
/// * `path` - Path to the dataset folder (e.g., `data/powergraph/ieee24`)
/// * `task` - Which task labels to load (Binary, Regression, or MultiClass)
/// * `max_samples` - Maximum number of samples to load (0 = all)
///
/// # Returns
/// Vector of PowerGraphSample with node features, edge features, and labels
#[cfg(feature = "powergraph")]
pub fn load_powergraph_dataset(
    path: &Path,
    task: PowerGraphTask,
    max_samples: usize,
) -> Result<Vec<PowerGraphSample>> {
    // Locate raw data directory
    let raw_path = path.join("raw");
    let base_path = if raw_path.exists() { raw_path } else { path.to_path_buf() };

    // Load node features: Bf.mat -> [num_samples, num_nodes, 3]
    let bf_path = base_path.join("Bf.mat");
    let bf_file = MatFile::parse(std::fs::File::open(&bf_path)?)
        .context("Failed to parse Bf.mat")?;

    let bf_array = bf_file
        .find_by_name("Bf")
        .ok_or_else(|| anyhow!("Bf array not found in Bf.mat"))?;

    let (node_features, num_samples) = load_node_features(bf_array)?;

    // Load edge list: blist.mat -> [(from, to), ...]
    let blist_path = base_path.join("blist.mat");
    let blist_file = MatFile::parse(std::fs::File::open(&blist_path)?)
        .context("Failed to parse blist.mat")?;

    let blist_array = blist_file
        .find_by_name("blist")
        .ok_or_else(|| anyhow!("blist array not found in blist.mat"))?;

    let edge_index = load_edge_index(blist_array)?;

    // Load edge features: Ef.mat -> [num_samples, num_edges, 4]
    let ef_path = base_path.join("Ef.mat");
    let edge_features = if ef_path.exists() {
        let ef_file = MatFile::parse(std::fs::File::open(&ef_path)?)
            .context("Failed to parse Ef.mat")?;

        let ef_array = ef_file
            .find_by_name("Ef")
            .ok_or_else(|| anyhow!("Ef array not found in Ef.mat"))?;

        load_edge_features(ef_array)?
    } else {
        // Generate empty edge features if not available
        vec![vec![[0.0, 0.0, 0.0, 0.0]; edge_index.len()]; num_samples]
    };

    // Load labels based on task
    let labels = load_labels(&base_path, task, num_samples)?;

    // Load explanation masks if available
    let explanations = load_explanations(&base_path, num_samples, edge_index.len())?;

    // Construct samples
    let sample_limit = if max_samples == 0 { num_samples } else { max_samples.min(num_samples) };
    let mut samples = Vec::with_capacity(sample_limit);

    for i in 0..sample_limit {
        let sample = PowerGraphSample {
            num_nodes: node_features[i].len(),
            num_edges: edge_index.len(),
            node_features: node_features[i].clone(),
            edge_index: edge_index.clone(),
            edge_features: edge_features[i].clone(),
            label_binary: labels.binary.as_ref().map(|l| l[i]),
            label_regression: labels.regression.as_ref().map(|l| l[i]),
            label_multiclass: labels.multiclass.as_ref().map(|l| l[i]),
            explanation_mask: explanations.as_ref().map(|e| e[i].clone()),
        };
        samples.push(sample);
    }

    Ok(samples)
}

/// Labels container for different task types
struct Labels {
    binary: Option<Vec<bool>>,
    regression: Option<Vec<f64>>,
    multiclass: Option<Vec<usize>>,
}

#[cfg(feature = "powergraph")]
fn load_labels(base_path: &Path, task: PowerGraphTask, num_samples: usize) -> Result<Labels> {
    let mut labels = Labels {
        binary: None,
        regression: None,
        multiclass: None,
    };

    match task {
        PowerGraphTask::Binary => {
            let path = base_path.join("of_bi.mat");
            if path.exists() {
                let file = MatFile::parse(std::fs::File::open(&path)?)?;
                let array = file.find_by_name("of_bi")
                    .ok_or_else(|| anyhow!("of_bi array not found"))?;
                labels.binary = Some(load_binary_labels(array, num_samples)?);
            }
        }
        PowerGraphTask::Regression => {
            let path = base_path.join("of_reg.mat");
            if path.exists() {
                let file = MatFile::parse(std::fs::File::open(&path)?)?;
                let array = file.find_by_name("of_reg")
                    .ok_or_else(|| anyhow!("of_reg array not found"))?;
                labels.regression = Some(load_regression_labels(array, num_samples)?);
            }
        }
        PowerGraphTask::MultiClass => {
            let path = base_path.join("of_mc.mat");
            if path.exists() {
                let file = MatFile::parse(std::fs::File::open(&path)?)?;
                let array = file.find_by_name("of_mc")
                    .ok_or_else(|| anyhow!("of_mc array not found"))?;
                labels.multiclass = Some(load_multiclass_labels(array, num_samples)?);
            }
        }
    }

    Ok(labels)
}

#[cfg(feature = "powergraph")]
fn load_explanations(
    base_path: &Path,
    num_samples: usize,
    num_edges: usize,
) -> Result<Option<Vec<Vec<bool>>>> {
    let path = base_path.join("exp.mat");
    if !path.exists() {
        return Ok(None);
    }

    let file = MatFile::parse(std::fs::File::open(&path)?)?;
    let array = file.find_by_name("exp")
        .ok_or_else(|| anyhow!("exp array not found in exp.mat"))?;

    let masks = load_explanation_masks(array, num_samples, num_edges)?;
    Ok(Some(masks))
}

// Helper functions for parsing matfile arrays

#[cfg(feature = "powergraph")]
fn get_3d_dimensions(array: &matfile::Array) -> Result<(usize, usize, usize)> {
    let dims = array.size();
    if dims.len() < 2 {
        return Err(anyhow!("Expected at least 2D array, got {}D", dims.len()));
    }

    // MATLAB stores in column-major order, dimensions might be [features, nodes, samples]
    // or [nodes, features, samples] depending on how data was saved
    if dims.len() == 2 {
        Ok((1, dims[0], dims[1]))
    } else if dims.len() == 3 {
        Ok((dims[2], dims[0], dims[1]))
    } else {
        Err(anyhow!("Unexpected array dimensions: {:?}", dims))
    }
}

#[cfg(feature = "powergraph")]
fn get_edge_count(array: &matfile::Array) -> Result<usize> {
    let dims = array.size();
    if dims.len() < 2 {
        return Err(anyhow!("Edge list must be 2D, got {}D", dims.len()));
    }
    // Edge list is [num_edges, 2] or [2, num_edges]
    Ok(dims[0].max(dims[1]) / 2 * 2 / dims.len().min(2))
}

#[cfg(feature = "powergraph")]
fn load_node_features(array: &matfile::Array) -> Result<(Vec<Vec<[f64; 3]>>, usize)> {
    let data = array.data();
    let dims = array.size();

    // Try to extract as f64 or convert from other numeric types
    let values: Vec<f64> = match data {
        matfile::NumericData::Double { real, .. } => real.clone(),
        matfile::NumericData::Single { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
    };

    // Parse dimensions: typically [nodes, features] or [nodes, features, samples]
    let (num_samples, num_nodes, num_features) = if dims.len() == 2 {
        (1, dims[0], dims[1])
    } else if dims.len() == 3 {
        (dims[2], dims[0], dims[1])
    } else {
        return Err(anyhow!("Unexpected node feature dimensions: {:?}", dims));
    };

    if num_features < 3 {
        return Err(anyhow!("Expected at least 3 node features, got {}", num_features));
    }

    let mut result = Vec::with_capacity(num_samples);

    for sample_idx in 0..num_samples {
        let mut nodes = Vec::with_capacity(num_nodes);
        for node_idx in 0..num_nodes {
            // MATLAB uses column-major order
            let base_idx = if num_samples == 1 {
                node_idx
            } else {
                sample_idx * num_nodes * num_features + node_idx
            };

            let features = [
                values.get(base_idx).copied().unwrap_or(0.0),
                values.get(base_idx + num_nodes).copied().unwrap_or(0.0),
                values.get(base_idx + 2 * num_nodes).copied().unwrap_or(0.0),
            ];
            nodes.push(features);
        }
        result.push(nodes);
    }

    Ok((result, num_samples))
}

#[cfg(feature = "powergraph")]
fn load_edge_index(array: &matfile::Array) -> Result<Vec<(usize, usize)>> {
    let data = array.data();
    let dims = array.size();

    let values: Vec<f64> = match data {
        matfile::NumericData::Double { real, .. } => real.clone(),
        matfile::NumericData::Single { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
    };

    // Edge list is typically [num_edges, 2] in MATLAB
    let num_edges = if dims[0] == 2 { dims[1] } else { dims[0] };
    let mut edges = Vec::with_capacity(num_edges);

    for i in 0..num_edges {
        // MATLAB indices are 1-based, convert to 0-based
        let from_idx = if dims[0] == 2 {
            values[i] as usize - 1
        } else {
            values[i] as usize - 1
        };
        let to_idx = if dims[0] == 2 {
            values[num_edges + i] as usize - 1
        } else {
            values[i + num_edges] as usize - 1
        };
        edges.push((from_idx, to_idx));
    }

    Ok(edges)
}

#[cfg(feature = "powergraph")]
fn load_edge_features(array: &matfile::Array) -> Result<Vec<Vec<[f64; 4]>>> {
    let data = array.data();
    let dims = array.size();

    let values: Vec<f64> = match data {
        matfile::NumericData::Double { real, .. } => real.clone(),
        matfile::NumericData::Single { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
    };

    let (num_samples, num_edges, num_features) = if dims.len() == 2 {
        (1, dims[0], dims[1])
    } else if dims.len() == 3 {
        (dims[2], dims[0], dims[1])
    } else {
        return Err(anyhow!("Unexpected edge feature dimensions: {:?}", dims));
    };

    if num_features < 4 {
        return Err(anyhow!("Expected at least 4 edge features, got {}", num_features));
    }

    let mut result = Vec::with_capacity(num_samples);

    for sample_idx in 0..num_samples {
        let mut edges = Vec::with_capacity(num_edges);
        for edge_idx in 0..num_edges {
            let base_idx = if num_samples == 1 {
                edge_idx
            } else {
                sample_idx * num_edges * num_features + edge_idx
            };

            let features = [
                values.get(base_idx).copied().unwrap_or(0.0),
                values.get(base_idx + num_edges).copied().unwrap_or(0.0),
                values.get(base_idx + 2 * num_edges).copied().unwrap_or(0.0),
                values.get(base_idx + 3 * num_edges).copied().unwrap_or(0.0),
            ];
            edges.push(features);
        }
        result.push(edges);
    }

    Ok(result)
}

#[cfg(feature = "powergraph")]
fn load_binary_labels(array: &matfile::Array, _num_samples: usize) -> Result<Vec<bool>> {
    let data = array.data();

    let values: Vec<f64> = match data {
        matfile::NumericData::Double { real, .. } => real.clone(),
        matfile::NumericData::Single { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
    };

    Ok(values.iter().map(|&v| v != 0.0).collect())
}

#[cfg(feature = "powergraph")]
fn load_regression_labels(array: &matfile::Array, _num_samples: usize) -> Result<Vec<f64>> {
    let data = array.data();

    let values: Vec<f64> = match data {
        matfile::NumericData::Double { real, .. } => real.clone(),
        matfile::NumericData::Single { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
    };

    Ok(values)
}

#[cfg(feature = "powergraph")]
fn load_multiclass_labels(array: &matfile::Array, _num_samples: usize) -> Result<Vec<usize>> {
    let data = array.data();

    let values: Vec<f64> = match data {
        matfile::NumericData::Double { real, .. } => real.clone(),
        matfile::NumericData::Single { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
    };

    // MATLAB class labels are 1-indexed, convert to 0-indexed
    Ok(values.iter().map(|&v| (v as usize).saturating_sub(1)).collect())
}

#[cfg(feature = "powergraph")]
fn load_explanation_masks(
    array: &matfile::Array,
    num_samples: usize,
    num_edges: usize,
) -> Result<Vec<Vec<bool>>> {
    let data = array.data();

    let values: Vec<f64> = match data {
        matfile::NumericData::Double { real, .. } => real.clone(),
        matfile::NumericData::Single { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt8 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt16 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt32 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::Int64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
        matfile::NumericData::UInt64 { real, .. } => real.iter().map(|&x| x as f64).collect(),
    };

    let mut result = Vec::with_capacity(num_samples);
    for sample_idx in 0..num_samples {
        let mask: Vec<bool> = (0..num_edges)
            .map(|edge_idx| {
                let idx = sample_idx * num_edges + edge_idx;
                values.get(idx).map(|&v| v != 0.0).unwrap_or(false)
            })
            .collect();
        result.push(mask);
    }

    Ok(result)
}

/// Convert a PowerGraph sample to PyTorch Geometric compatible JSON format.
///
/// This produces a JSON object with:
/// - `x`: Node features [num_nodes, 3]
/// - `edge_index`: Edge connectivity [2, num_edges]
/// - `edge_attr`: Edge features [num_edges, 4]
/// - `y`: Label (if available)
#[cfg(feature = "powergraph")]
pub fn sample_to_pytorch_geometric_json(sample: &PowerGraphSample) -> serde_json::Value {
    use serde_json::json;

    // Node features: [num_nodes, 3]
    let x: Vec<Vec<f64>> = sample.node_features.iter()
        .map(|f| f.to_vec())
        .collect();

    // Edge index: [2, num_edges] - transposed from [(from, to), ...]
    let edge_index: Vec<Vec<usize>> = vec![
        sample.edge_index.iter().map(|(from, _)| *from).collect(),
        sample.edge_index.iter().map(|(_, to)| *to).collect(),
    ];

    // Edge features: [num_edges, 4]
    let edge_attr: Vec<Vec<f64>> = sample.edge_features.iter()
        .map(|f| f.to_vec())
        .collect();

    let mut obj = json!({
        "num_nodes": sample.num_nodes,
        "num_edges": sample.num_edges,
        "x": x,
        "edge_index": edge_index,
        "edge_attr": edge_attr,
    });

    // Add labels if available
    if let Some(y) = sample.label_binary {
        obj["y"] = json!(y as u8);
    } else if let Some(y) = sample.label_regression {
        obj["y"] = json!(y);
    } else if let Some(y) = sample.label_multiclass {
        obj["y"] = json!(y);
    }

    // Add explanation mask if available
    if let Some(ref mask) = sample.explanation_mask {
        let mask_int: Vec<u8> = mask.iter().map(|&b| b as u8).collect();
        obj["edge_mask"] = json!(mask_int);
    }

    obj
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_powergraph_sample_serialization() {
        let sample = PowerGraphSample {
            num_nodes: 3,
            num_edges: 2,
            node_features: vec![
                [1.0, 2.0, 1.0],
                [0.5, 1.0, 1.02],
                [-0.5, -1.0, 0.98],
            ],
            edge_index: vec![(0, 1), (1, 2)],
            edge_features: vec![
                [10.0, 5.0, 0.1, 100.0],
                [8.0, 4.0, 0.15, 80.0],
            ],
            label_binary: Some(true),
            label_regression: None,
            label_multiclass: None,
            explanation_mask: Some(vec![true, false]),
        };

        let json_str = serde_json::to_string(&sample).unwrap();
        let parsed: PowerGraphSample = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed.num_nodes, 3);
        assert_eq!(parsed.num_edges, 2);
        assert_eq!(parsed.label_binary, Some(true));
        assert_eq!(parsed.explanation_mask, Some(vec![true, false]));
    }

    #[test]
    fn test_feature_specs() {
        let node_spec = NodeFeatureSpec::default();
        assert_eq!(node_spec.p_net, 0);
        assert_eq!(node_spec.s_net, 1);
        assert_eq!(node_spec.voltage, 2);

        let edge_spec = EdgeFeatureSpec::default();
        assert_eq!(edge_spec.p_flow, 0);
        assert_eq!(edge_spec.q_flow, 1);
        assert_eq!(edge_spec.reactance, 2);
        assert_eq!(edge_spec.line_rating, 3);
    }

    #[cfg(feature = "powergraph")]
    #[test]
    fn test_sample_to_pytorch_geometric_json() {
        let sample = PowerGraphSample {
            num_nodes: 2,
            num_edges: 1,
            node_features: vec![[1.0, 2.0, 1.0], [0.5, 1.0, 1.02]],
            edge_index: vec![(0, 1)],
            edge_features: vec![[10.0, 5.0, 0.1, 100.0]],
            label_binary: Some(true),
            label_regression: None,
            label_multiclass: None,
            explanation_mask: None,
        };

        let json = sample_to_pytorch_geometric_json(&sample);

        assert_eq!(json["num_nodes"], 2);
        assert_eq!(json["num_edges"], 1);
        assert_eq!(json["y"], 1);
        assert_eq!(json["edge_index"][0][0], 0);
        assert_eq!(json["edge_index"][1][0], 1);
    }
}
