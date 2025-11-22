/// Pipeline Pane - Workflow definition and feature engineering
///
/// The pipeline pane provides:
/// - Workflow node visualization
/// - Transform configuration
/// - Data flow mapping
/// - Feature engineering tools

use crate::components::*;

/// Pipeline transformation node
#[derive(Clone, Debug)]
pub struct PipelineNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub config: std::collections::HashMap<String, String>,
    pub inputs: usize,
    pub outputs: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    Source,
    Transform,
    Aggregate,
    Filter,
    Output,
    Feature,
}

impl NodeType {
    pub fn symbol(&self) -> &'static str {
        match self {
            NodeType::Source => "◆",
            NodeType::Transform => "▲",
            NodeType::Aggregate => "⬡",
            NodeType::Filter => "○",
            NodeType::Output => "■",
            NodeType::Feature => "★",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            NodeType::Source => "Source",
            NodeType::Transform => "Transform",
            NodeType::Aggregate => "Aggregate",
            NodeType::Filter => "Filter",
            NodeType::Output => "Output",
            NodeType::Feature => "Feature",
        }
    }
}

/// Transform template
#[derive(Clone, Debug)]
pub struct TransformTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub inputs_required: usize,
}

/// Pipeline pane state
#[derive(Clone, Debug)]
pub struct PipelinePaneState {
    // Nodes
    pub nodes: Vec<PipelineNode>,
    pub selected_node: usize,

    // Templates
    pub templates: Vec<TransformTemplate>,

    // Configuration
    pub active_config: std::collections::HashMap<String, String>,

    // Component states
    pub nodes_list: ListWidget,
    pub templates_list: ListWidget,
    pub config_input: InputWidget,
    pub status_text: ParagraphWidget,

    // UI state
    pub show_config: bool,
    pub valid_state: bool,
}

impl Default for PipelinePaneState {
    fn default() -> Self {
        let nodes = vec![
            PipelineNode {
                id: "node_001".into(),
                name: "Load Dataset".into(),
                node_type: NodeType::Source,
                config: Default::default(),
                inputs: 0,
                outputs: 1,
            },
            PipelineNode {
                id: "node_002".into(),
                name: "Clean Data".into(),
                node_type: NodeType::Transform,
                config: Default::default(),
                inputs: 1,
                outputs: 1,
            },
            PipelineNode {
                id: "node_003".into(),
                name: "Feature Engineering".into(),
                node_type: NodeType::Feature,
                config: Default::default(),
                inputs: 1,
                outputs: 1,
            },
            PipelineNode {
                id: "node_004".into(),
                name: "Output Results".into(),
                node_type: NodeType::Output,
                config: Default::default(),
                inputs: 1,
                outputs: 0,
            },
        ];

        let templates = vec![
            TransformTemplate {
                id: "tmpl_001".into(),
                name: "Normalize Values".into(),
                description: "Normalize numeric columns to 0-1 range".into(),
                inputs_required: 1,
            },
            TransformTemplate {
                id: "tmpl_002".into(),
                name: "Handle Missing".into(),
                description: "Fill or drop missing values".into(),
                inputs_required: 1,
            },
            TransformTemplate {
                id: "tmpl_003".into(),
                name: "Aggregate".into(),
                description: "Group and aggregate data".into(),
                inputs_required: 1,
            },
            TransformTemplate {
                id: "tmpl_004".into(),
                name: "Feature Extraction".into(),
                description: "Extract derived features".into(),
                inputs_required: 1,
            },
        ];

        let mut nodes_list = ListWidget::new("pipeline_nodes");
        for node in &nodes {
            nodes_list.add_item(
                format!("{} {}", node.node_type.symbol(), node.name),
                node.id.clone(),
            );
        }

        let mut templates_list = ListWidget::new("pipeline_templates");
        for tmpl in &templates {
            templates_list.add_item(tmpl.name.clone(), tmpl.id.clone());
        }

        PipelinePaneState {
            nodes,
            selected_node: 0,
            templates,
            active_config: Default::default(),
            nodes_list,
            templates_list,
            config_input: InputWidget::new("pipeline_config")
                .with_placeholder("Enter configuration..."),
            status_text: ParagraphWidget::new("pipeline_status"),
            show_config: false,
            valid_state: true,
        }
    }
}

impl PipelinePaneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_next_node(&mut self) {
        if self.selected_node < self.nodes.len().saturating_sub(1) {
            self.selected_node += 1;
        }
    }

    pub fn select_prev_node(&mut self) {
        if self.selected_node > 0 {
            self.selected_node -= 1;
        }
    }

    pub fn selected_node(&self) -> Option<&PipelineNode> {
        self.nodes.get(self.selected_node)
    }

    pub fn add_node(&mut self, node: PipelineNode) {
        self.nodes.push(node.clone());
        self.nodes_list.add_item(
            format!("{} {}", node.node_type.symbol(), node.name),
            node.id,
        );
    }

    pub fn remove_node(&mut self, index: usize) {
        if index < self.nodes.len() {
            self.nodes.remove(index);
            self.selected_node = self.selected_node.saturating_sub(1);
        }
    }

    pub fn validate_pipeline(&mut self) -> bool {
        // Check for source and output nodes
        let has_source = self.nodes.iter().any(|n| n.node_type == NodeType::Source);
        let has_output = self.nodes.iter().any(|n| n.node_type == NodeType::Output);
        self.valid_state = has_source && has_output;
        self.valid_state
    }

    pub fn update_node_config(&mut self, key: String, value: String) {
        if let Some(node) = self.nodes.get_mut(self.selected_node) {
            node.config.insert(key, value);
        }
    }

    pub fn get_node_config(&self) -> std::collections::HashMap<String, String> {
        self.selected_node()
            .map(|n| n.config.clone())
            .unwrap_or_default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn update_status(&mut self) {
        let valid = if self.valid_state { "✓ Valid" } else { "✗ Invalid" };
        self.status_text.set_content(format!(
            "Pipeline: {}\nNodes: {}\nConnected: {}",
            valid,
            self.node_count(),
            self.calculate_connections()
        ));
    }

    fn calculate_connections(&self) -> usize {
        self.nodes.windows(2).filter(|w| w[0].outputs > 0 && w[1].inputs > 0).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_init() {
        let mut state = PipelinePaneState::new();
        assert_eq!(state.node_count(), 4);
        assert!(state.validate_pipeline());
    }

    #[test]
    fn test_node_selection() {
        let mut state = PipelinePaneState::new();
        state.select_next_node();
        assert_eq!(state.selected_node, 1);
        state.select_prev_node();
        assert_eq!(state.selected_node, 0);
    }

    #[test]
    fn test_add_node() {
        let mut state = PipelinePaneState::new();
        let initial = state.node_count();
        let node = PipelineNode {
            id: "test_001".into(),
            name: "Test Node".into(),
            node_type: NodeType::Transform,
            config: Default::default(),
            inputs: 1,
            outputs: 1,
        };
        state.add_node(node);
        assert_eq!(state.node_count(), initial + 1);
    }

    #[test]
    fn test_node_type_symbol() {
        assert_eq!(NodeType::Source.symbol(), "◆");
        assert_eq!(NodeType::Feature.symbol(), "★");
    }

    #[test]
    fn test_node_config() {
        let mut state = PipelinePaneState::new();
        state.update_node_config("key".into(), "value".into());
        let config = state.get_node_config();
        assert_eq!(config.get("key").map(|s| s.as_str()), Some("value"));
    }

    #[test]
    fn test_pipeline_validation() {
        let state = PipelinePaneState::new();
        assert!(state.valid_state);
    }
}
