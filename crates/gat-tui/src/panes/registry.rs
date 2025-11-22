use crate::models::PaneId;
use std::collections::HashMap;

/// Registry trait for pane implementations
pub trait PaneRegistry {
    fn id(&self) -> PaneId;
    fn label(&self) -> &'static str;
    fn hotkey(&self) -> char;
}

/// Concrete pane registry
pub struct DefaultPaneRegistry {
    panes: HashMap<PaneId, String>,
}

impl DefaultPaneRegistry {
    pub fn new() -> Self {
        let mut panes = HashMap::new();
        panes.insert(PaneId::Dashboard, "Dashboard".to_string());
        panes.insert(PaneId::Operations, "Operations".to_string());
        panes.insert(PaneId::Datasets, "Datasets".to_string());
        panes.insert(PaneId::Pipeline, "Pipeline".to_string());
        panes.insert(PaneId::Commands, "Commands".to_string());
        panes.insert(PaneId::Help, "Help".to_string());

        DefaultPaneRegistry { panes }
    }

    pub fn get_pane_label(&self, pane_id: PaneId) -> Option<&str> {
        self.panes.get(&pane_id).map(|s| s.as_str())
    }

    pub fn all_panes(&self) -> Vec<PaneId> {
        vec![
            PaneId::Dashboard,
            PaneId::Operations,
            PaneId::Datasets,
            PaneId::Pipeline,
            PaneId::Commands,
            PaneId::Help,
        ]
    }

    pub fn pane_hotkeys(&self) -> Vec<(PaneId, char)> {
        vec![
            (PaneId::Dashboard, '1'),
            (PaneId::Operations, '2'),
            (PaneId::Datasets, '3'),
            (PaneId::Pipeline, '4'),
            (PaneId::Commands, '5'),
            (PaneId::Help, 'h'),
        ]
    }
}

impl Default for DefaultPaneRegistry {
    fn default() -> Self {
        Self::new()
    }
}
