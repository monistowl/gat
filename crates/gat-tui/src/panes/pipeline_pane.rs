/// Pipeline Pane - Workflow definition and feature engineering
///
/// The pipeline pane provides:
/// - Workflow node visualization
/// - Transform configuration
/// - Data flow mapping
/// - Feature engineering tools
use crate::components::*;
use crate::ui::{
    ContextButton, GraphView, Pane, PaneContext, PaneLayout, PaneView, ResponsiveRules, Sidebar,
    SubTabs, TableView, Tabs, Tooltip,
};

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
        let valid = if self.valid_state {
            "✓ Valid"
        } else {
            "✗ Invalid"
        };
        self.status_text.set_content(format!(
            "Pipeline: {}\nNodes: {}\nConnected: {}",
            valid,
            self.node_count(),
            self.calculate_connections()
        ));
    }

    fn calculate_connections(&self) -> usize {
        self.nodes
            .windows(2)
            .filter(|w| w[0].outputs > 0 && w[1].inputs > 0)
            .count()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PaneView Implementation
// ─────────────────────────────────────────────────────────────────────────────

/// Pipeline pane for the TUI registry
pub struct PipelinePane;

impl PipelinePane {
    pub fn layout(context: &PaneContext) -> PaneLayout {
        let modal_hint = context
            .command_modal
            .as_ref()
            .map(|modal| {
                format!(
                    "[{}] Open command modal to inspect recent outputs inline",
                    modal.run_hotkey.to_ascii_lowercase()
                )
            })
            .unwrap_or_else(|| {
                "[x] Open command modal to inspect recent outputs inline".to_string()
            });

        let source_selection = Pane::new("Source selection").body([
            "Guided selectors avoid free-form hotkeys for choosing data.",
            "Radio: (•) Live telemetry stream  |  ( ) Batch archive replay",
            "Dropdown: Dataset variant ↴ [Day-ahead | Real-time | Sandbox]",
            "Quick tip: Swap sources without breaking downstream transforms.",
        ]);

        // Standard transforms
        let standard_transforms = Pane::new("Standard transforms").body([
            "Radio: (•) Use template 'Grid cleanup'  |  ( ) Start blank",
            "Dropdown: Insert transform ↴ [Resample, Gap-fill, Forecast smoothing]",
            "Quick tip: Add step drops a transform under the highlighted row.",
            "Quick tip: Reorder keeps dependencies intact and updates previews.",
        ]);

        // New feature transforms
        let scenario_transforms = Pane::new("Scenario materialization").body([
            "Materialize templated scenarios into full manifest",
            "File: scenarios.yaml → Manifest: scenarios_expanded.json",
            "Status: [Queued] Ready to load template",
        ]);

        let featurize_transforms = Pane::new("Feature engineering").body([
            "Transform grid data into ML-ready features",
            "Available:",
            "  • GNN: Export graph topology for neural networks",
            "  • KPI: Generate training features from batch results",
            "  • Geo: Spatial-temporal features from geospatial data",
        ]);

        let transform_tabs = Tabs::new(["Classic", "Scenarios", "Features"], 0);
        let transforms = Pane::new("Transforms")
            .with_child(standard_transforms)
            .with_child(scenario_transforms)
            .with_child(featurize_transforms)
            .with_tabs(transform_tabs);

        let delivery_outputs = Pane::new("Outputs").body([
            "Dropdown: Delivery target ↴ [Warehouse table, DERMS feed, Notebook]",
            "Radio: (•) Single run report  |  ( ) Continuous subscription",
            "Inline hint: Outputs inherit naming from the selected source and template.",
        ]);

        // Visual DAG graph representation (fancy-ui feature)
        let graph = GraphView::new()
            .add_node("n1", "Live telemetry", "◆", 0)
            .add_node("n2", "Resample", "▲", 1)
            .add_node("n3", "Gap fill", "▲", 2)
            .add_node("n4", "Validate", "○", 3)
            .add_node("n5", "Warehouse", "■", 4)
            .add_edge("n1", "n2")
            .add_edge("n2", "n3")
            .add_edge("n3", "n4")
            .add_edge("n4", "n5")
            .with_legend();

        let preview_table = TableView::new(["Step", "From", "To"])
            .add_row(["Source: Live telemetry", "edge-collector", "Normalizer"])
            .add_row(["Transform: Resample", "Normalizer", "Gap fill"])
            .add_row(["Transform: Validate", "Gap fill", "Outputs"])
            .add_row(["Output: Warehouse", "Outputs", "analytics.fact_runs"]);

        let preview = Pane::new("Pipeline graph preview")
            .body([
                "Auto-refreshes as you add or reorder steps; aligns with dropdown choices.",
                "Helpful for confirming fan-in/fan-out before dispatching a run.",
            ])
            .with_graph(graph)
            .with_table(preview_table);

        let node_metrics = TableView::new(["Node", "Runtime", "Rows", "Warnings"])
            .add_row(["Live telemetry", "35 ms", "12.3k", "–"])
            .add_row(["Resample", "48 ms", "12.3k", "0 drift flags"])
            .add_row(["Gap fill", "62 ms", "12.3k", "1 null column"])
            .add_row(["Validate", "41 ms", "12.3k", "Schema mismatch"])
            .add_row(["Warehouse", "85 ms", "12.3k", "Pending write"]);

        let metrics = Pane::new("Per-node metrics")
            .body([
                "Runtime, row counts, and warnings stay visible while composing the graph.",
                "Use them to spot slow transforms or schema drift before dispatching runs.",
            ])
            .with_table(node_metrics);

        let outputs_table = TableView::new(["Output", "Status", "Action"])
            .add_row([
                "envelope.parquet",
                "✓ Ready",
                "Use modal to stream the tail of the run logs",
            ])
            .add_row([
                "validation_report.txt",
                "⚠ Drift noted",
                "Open command modal to inspect warnings",
            ])
            .add_row([
                "run_manifest.json",
                "✓ Materialized",
                "Preview via command modal without switching panes",
            ]);

        let recent_outputs = Pane::new("Recent outputs & drill-ins")
            .body([
                "Review the freshest artifacts and drill into details without leaving the composer."
                    .to_string(),
                modal_hint,
            ])
            .with_table(outputs_table)
            .mark_visual();

        let controls = Pane::new("Controls")
            .body([
                "Button: [Ctrl+R] Run pipeline — executes the previewed path.",
                "Shows the visible hotkey on the control to reduce guesswork.",
                "Button: [n] Rerun focused node — repeats only the highlighted stage.",
                "Button: [c] Edit command template — opens Commands pane with the node snippet.",
            ])
            .mark_visual();

        PaneLayout::new(
            Pane::new("Pipeline composer")
                .body([
                    "Pick sources, transformations, and outputs with menus instead of ad-hoc keys.",
                    "Inline instructions keep each section self-guided; subtabs appear when crowded.",
                    "Scenario materialization sits next to classic and feature transforms for quick swaps.",
                    "Feature engineering, scenario materialization, and outputs stay in one view.",
                    "Feature engineering covers GNN, KPI, and Geo exporters upfront.",
                ])
                .with_child(source_selection)
                .with_child(transforms)
                .with_child(delivery_outputs),
        )
        .with_secondary(
            Pane::new("Review & dispatch")
                .with_child(preview)
                .with_child(metrics)
                .with_child(recent_outputs)
                .with_child(controls),
        )
        .with_sidebar(
            Sidebar::new("Section tips", false).lines([
                "Use 'Add step' to insert under the focused transform.",
                "'Reorder' toggles move mode; preview table updates live.",
                "Keep transforms concise - subtabs split dense step lists.",
            ]),
        )
        .with_subtabs(SubTabs::new(["Compose", "Graph"], 0))
        .with_responsive_rules(ResponsiveRules {
            wide_threshold: 92,
            tall_threshold: 26,
            expand_visuals_on_wide: true,
            collapse_secondary_first: true,
        })
    }
}

impl PaneView for PipelinePane {
    fn id(&self) -> &'static str {
        "pipeline"
    }

    fn label(&self) -> &'static str {
        "Pipeline"
    }

    fn hotkey(&self) -> char {
        '4'
    }

    fn layout(&self, context: &PaneContext) -> PaneLayout {
        Self::layout(context)
    }

    fn tooltip(&self, _context: &PaneContext) -> Option<Tooltip> {
        Some(Tooltip::new(
            "Compose, reorder, and run Gat pipelines while keeping controls nearby.",
        ))
    }

    fn context_buttons(&self, _context: &PaneContext) -> Vec<ContextButton> {
        vec![
            ContextButton::new('a', "[a] Add step — inserts under the focused transform"),
            ContextButton::new('o', "[o] Reorder — move step while preserving dependencies"),
            ContextButton::new('r', "[r] Run pipeline — mirrors the [Ctrl+R] control"),
            ContextButton::new('n', "[n] Rerun node — execute the highlighted stage"),
            ContextButton::new('c', "[c] Open command template in Commands pane"),
        ]
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
