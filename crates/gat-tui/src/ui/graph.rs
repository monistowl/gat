/// ASCII DAG (Directed Acyclic Graph) visualization
///
/// Renders pipeline nodes and edges using box-drawing characters
/// for a visual representation of data flow.

use super::{EmptyState, THEME};

/// A node in the graph
#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub symbol: String,
    pub level: usize,  // Depth in the DAG (0 = source, increases downstream)
}

/// An edge connecting two nodes
#[derive(Clone, Debug)]
pub struct GraphEdge {
    pub from_id: String,
    pub to_id: String,
}

/// Visual graph renderer for DAG pipelines
#[derive(Clone, Debug)]
pub struct GraphView {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub empty: Option<EmptyState>,
    pub compact: bool,
    pub show_legend: bool,
}

impl GraphView {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            empty: None,
            compact: false,
            show_legend: false,
        }
    }

    pub fn add_node(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        symbol: impl Into<String>,
        level: usize,
    ) -> Self {
        self.nodes.push(GraphNode {
            id: id.into(),
            label: label.into(),
            symbol: symbol.into(),
            level,
        });
        self
    }

    pub fn add_edge(mut self, from_id: impl Into<String>, to_id: impl Into<String>) -> Self {
        self.edges.push(GraphEdge {
            from_id: from_id.into(),
            to_id: to_id.into(),
        });
        self
    }

    pub fn with_empty_state(mut self, empty: EmptyState) -> Self {
        self.empty = Some(empty);
        self
    }

    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    pub fn with_legend(mut self) -> Self {
        self.show_legend = true;
        self
    }

    pub fn has_nodes(&self) -> bool {
        !self.nodes.is_empty()
    }

    /// Render the graph as ASCII art lines
    #[cfg(feature = "fancy-ui")]
    pub fn render_lines(&self) -> Vec<String> {
        if self.nodes.is_empty() {
            if let Some(empty) = &self.empty {
                return empty.render_lines(&THEME);
            }
            return vec!["(empty graph)".to_string()];
        }

        let mut lines = Vec::new();

        // Group nodes by level for vertical layout
        let max_level = self.nodes.iter().map(|n| n.level).max().unwrap_or(0);
        let mut levels: Vec<Vec<&GraphNode>> = vec![Vec::new(); max_level + 1];

        for node in &self.nodes {
            if node.level <= max_level {
                levels[node.level].push(node);
            }
        }

        // Render nodes level by level with connections
        for (level_idx, level_nodes) in levels.iter().enumerate() {
            if level_nodes.is_empty() {
                continue;
            }

            // Render all nodes at this level
            for (node_idx, node) in level_nodes.iter().enumerate() {
                let prefix = if node_idx == 0 && level_idx > 0 {
                    "    │"  // Vertical connector from previous level
                } else if node_idx > 0 {
                    "     "  // Indent for subsequent nodes at same level
                } else {
                    ""
                };

                let node_line = if self.compact {
                    format!("{} {} {}", prefix, node.symbol, node.label)
                } else {
                    format!("{}  ┌─ {} {} ─ {}", prefix, node.symbol, node.label, node.id)
                };
                lines.push(node_line);

                // Show connection to next level if this isn't the last level
                if level_idx < max_level && !self.edges.is_empty() {
                    let has_outgoing = self
                        .edges
                        .iter()
                        .any(|e| e.from_id == node.id);

                    if has_outgoing {
                        lines.push("    │".to_string());
                        lines.push("    ↓".to_string());
                    }
                }
            }

            // Add spacing between levels
            if level_idx < levels.len() - 1 && !levels[level_idx + 1].is_empty() {
                if !self.compact {
                    lines.push("".to_string());
                }
            }
        }

        // Add legend if requested
        if self.show_legend {
            lines.push("".to_string());
            lines.push("Legend:".to_string());

            let mut symbols: Vec<_> = self.nodes.iter()
                .map(|n| (n.symbol.clone(), format!("{} = {}", n.symbol, self.get_node_type_label(&n.symbol))))
                .collect();
            symbols.sort();
            symbols.dedup_by(|a, b| a.0 == b.0);

            for (_, desc) in symbols {
                lines.push(format!("  {}", desc));
            }
        }

        lines
    }

    /// Render simple fallback for minimal builds
    #[cfg(not(feature = "fancy-ui"))]
    pub fn render_lines(&self) -> Vec<String> {
        if self.nodes.is_empty() {
            if let Some(empty) = &self.empty {
                return empty.render_lines(&THEME);
            }
            return vec!["(empty graph)".to_string()];
        }

        let mut lines = Vec::new();

        // Simple list rendering without fancy graphics
        lines.push("Pipeline nodes:".to_string());
        for node in &self.nodes {
            lines.push(format!("  {} {} (level {})", node.symbol, node.label, node.level));
        }

        if !self.edges.is_empty() {
            lines.push("".to_string());
            lines.push(format!("Connections: {} edges", self.edges.len()));
        }

        lines
    }

    fn get_node_type_label(&self, symbol: &str) -> &'static str {
        match symbol {
            "◆" => "Source",
            "▲" => "Transform",
            "⬡" => "Aggregate",
            "○" => "Filter",
            "■" => "Output",
            "★" => "Feature",
            _ => "Node",
        }
    }
}

impl Default for GraphView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_empty() {
        let graph = GraphView::new();
        let lines = graph.render_lines();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_graph_single_node() {
        let graph = GraphView::new()
            .add_node("n1", "Load Data", "◆", 0);

        let lines = graph.render_lines();
        assert!(lines.iter().any(|l| l.contains("Load Data")));
    }

    #[test]
    fn test_graph_chain() {
        let graph = GraphView::new()
            .add_node("n1", "Load Data", "◆", 0)
            .add_node("n2", "Transform", "▲", 1)
            .add_node("n3", "Output", "■", 2)
            .add_edge("n1", "n2")
            .add_edge("n2", "n3");

        let lines = graph.render_lines();

        // Should have all three nodes
        assert!(lines.iter().any(|l| l.contains("Load Data")));
        assert!(lines.iter().any(|l| l.contains("Transform")));
        assert!(lines.iter().any(|l| l.contains("Output")));
    }

    #[test]
    fn test_graph_with_legend() {
        let graph = GraphView::new()
            .add_node("n1", "Load", "◆", 0)
            .add_node("n2", "Process", "★", 1)
            .with_legend();

        let lines = graph.render_lines();
        assert!(lines.iter().any(|l| l.contains("Legend")));
    }

    #[test]
    fn test_graph_compact_mode() {
        let graph = GraphView::new()
            .add_node("n1", "Load", "◆", 0)
            .compact();

        let lines = graph.render_lines();
        // Compact mode should produce shorter lines
        assert!(!lines.is_empty());
    }
}
