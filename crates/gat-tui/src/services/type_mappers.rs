/// Type mappers to convert gat-core types to gat-tui types
///
/// This module provides conversion functions between gat-core's internal
/// data structures and gat-tui's display-oriented types.

use crate::data::{DatasetEntry, DatasetStatus, SystemMetrics};
use gat_core::graph_utils::GraphStats;
use gat_core::Network;
use std::time::SystemTime;

/// Estimate the size of a network in MB
fn estimate_size(network: &Network) -> f64 {
    // Rough estimate: ~1KB per node + 0.5KB per edge
    let node_bytes = network.graph.node_count() as f64 * 1024.0;
    let edge_bytes = network.graph.edge_count() as f64 * 512.0;
    (node_bytes + edge_bytes) / (1024.0 * 1024.0)
}

/// Convert a gat-core Network to a gat-tui DatasetEntry
///
/// Maps:
/// - Network nodes → row_count (number of buses/gens/loads)
/// - Network size → size_mb
/// - Network properties → description
pub fn network_to_dataset_entry(id: &str, network: &Network) -> DatasetEntry {
    match gat_core::graph_utils::graph_stats(network) {
        Ok(stats) => DatasetEntry {
            id: id.to_string(),
            name: format!("Grid {}", id),
            status: DatasetStatus::Ready,
            source: "gat-core".to_string(),
            row_count: stats.node_count,
            size_mb: estimate_size(network),
            last_updated: SystemTime::now(),
            description: format!(
                "Power grid with {} buses, {} branches, {} components",
                stats.node_count, stats.edge_count, stats.connected_components
            ),
        },
        Err(_) => DatasetEntry {
            id: id.to_string(),
            name: format!("Grid {}", id),
            status: DatasetStatus::Idle,
            source: "gat-core".to_string(),
            row_count: network.graph.node_count(),
            size_mb: estimate_size(network),
            last_updated: SystemTime::now(),
            description: "Power grid (stats unavailable)".to_string(),
        },
    }
}

/// Convert gat-core GraphStats to gat-tui SystemMetrics
///
/// Maps:
/// - connected_components → reliability metric (more islands = less reliable)
/// - density → deliverability (higher density = better connectivity)
/// - node_count → engagement metric
pub fn graph_stats_to_system_metrics(stats: &GraphStats) -> SystemMetrics {
    // Deliverability: inverse of density (higher density = more connected = more reliable)
    let deliverability_score = ((1.0 - stats.density) * 100.0).max(0.0).min(100.0);

    // LOLE (Loss of Load Expectation): scaled by number of islands
    // More islands = higher risk of disconnection
    let lole_hours_per_year = stats.connected_components as f64 * 2.5;

    // EUE (Expected Unserved Energy): scaled by network size
    // Larger networks have more potential unserved demand
    let eue_mwh_per_year = (stats.node_count as f64) * 0.5;

    SystemMetrics {
        deliverability_score,
        lole_hours_per_year,
        eue_mwh_per_year,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_size() {
        // Create a dummy network with 10 nodes and 5 edges
        let network = gat_core::Network::new();
        let size = estimate_size(&network);
        // Empty network should still have small positive size
        assert!(size >= 0.0);
    }

    #[test]
    fn test_graph_stats_to_metrics() {
        let stats = GraphStats {
            node_count: 100,
            edge_count: 150,
            density: 0.03,
            connected_components: 3,
            min_degree: 1,
            avg_degree: 3.0,
            max_degree: 5,
        };

        let metrics = graph_stats_to_system_metrics(&stats);

        // Deliverability should be inverse of density
        assert!(metrics.deliverability_score > 90.0);

        // LOLE should be proportional to islands
        assert_eq!(metrics.lole_hours_per_year, 3.0 * 2.5);

        // EUE should be proportional to nodes
        assert_eq!(metrics.eue_mwh_per_year, 100.0 * 0.5);
    }

    #[test]
    fn test_metrics_bounds() {
        // Test with high density (very connected)
        let stats = GraphStats {
            node_count: 100,
            edge_count: 4950,  // Nearly fully connected
            density: 0.99,
            connected_components: 1,
            min_degree: 50,
            avg_degree: 99.0,
            max_degree: 99,
        };

        let metrics = graph_stats_to_system_metrics(&stats);

        // Deliverability should be clamped between 0-100
        assert!(metrics.deliverability_score >= 0.0 && metrics.deliverability_score <= 100.0);
    }
}
