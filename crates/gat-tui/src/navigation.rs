use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TabId {
    Workflow,
    Derms,
    Adms,
    Config,
}

#[derive(Clone, Debug)]
pub struct DetailPayload {
    pub title: Option<String>,
    pub body: Option<String>,
}

impl DetailPayload {
    pub fn empty() -> Self {
        DetailPayload {
            title: None,
            body: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pane {
    pub title: String,
    pub description: String,
}

impl Pane {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        Pane {
            title: title.into(),
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PaneStack {
    panes: VecDeque<Pane>,
    active: usize,
}

impl PaneStack {
    pub fn new(initial: Pane) -> Self {
        PaneStack {
            panes: VecDeque::from(vec![initial]),
            active: 0,
        }
    }

    pub fn active_pane(&self) -> &Pane {
        &self.panes[self.active]
    }

    #[allow(dead_code)]
    pub fn push(&mut self, pane: Pane) {
        self.panes.push_back(pane);
        self.active = self.panes.len() - 1;
    }

    #[allow(dead_code)]
    pub fn pop(&mut self) {
        if self.panes.len() > 1 {
            self.panes.pop_back();
            self.active = self.panes.len() - 1;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub id: TabId,
    pub label: &'static str,
    pub menu_items: Vec<&'static str>,
    pub panes: PaneStack,
}

impl Tab {
    pub fn new(id: TabId, label: &'static str, menu_items: Vec<&'static str>, pane: Pane) -> Self {
        Tab {
            id,
            label,
            menu_items,
            panes: PaneStack::new(pane),
        }
    }
}

pub struct NavigationController {
    tabs: Vec<Tab>,
    active: usize,
    detail: DetailPayload,
    backstack: Vec<(TabId, usize)>,
}

impl NavigationController {
    pub fn new() -> Self {
        let tabs = vec![
            Tab::new(
                TabId::Workflow,
                "Workflow",
                vec!["Run workflow", "Refresh"],
                Pane::new("Workflow overview", "Monitor workflows and live runs"),
            ),
            Tab::new(
                TabId::Derms,
                "DERMS",
                vec!["Inspect DERMS", "Launch dispatch"],
                Pane::new("DERMS queue", "Pending DERMS workflows"),
            ),
            Tab::new(
                TabId::Adms,
                "ADMS",
                vec!["View ADMS plans", "Sync metrics"],
                Pane::new("ADMS command center", "Co-sim insights and tooling"),
            ),
            Tab::new(
                TabId::Config,
                "Config",
                vec!["Open config", "Reload docs"],
                Pane::new("Config explorer", "Browse gat-tui settings and docs"),
            ),
        ];
        NavigationController {
            tabs,
            active: 0,
            detail: DetailPayload::empty(),
            backstack: Vec::new(),
        }
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    #[allow(dead_code)]
    pub fn active_tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn active_tab_id(&self) -> TabId {
        self.tabs[self.active].id
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active_menu_items(&self) -> &[&'static str] {
        &self.tabs[self.active].menu_items
    }

    pub fn set_detail(&mut self, title: Option<String>, body: Option<String>) {
        self.detail = DetailPayload { title, body };
    }

    pub fn detail(&self) -> &DetailPayload {
        &self.detail
    }

    #[allow(dead_code)]
    pub fn switch_tab(&mut self, tab_id: TabId) {
        if let Some((idx, _)) = self
            .tabs
            .iter()
            .enumerate()
            .find(|(_, tab)| tab.id == tab_id)
        {
            self.backstack.push((self.active_tab_id(), self.active));
            self.active = idx;
        }
    }

    pub fn next_tab(&mut self) {
        let next = (self.active + 1) % self.tabs.len();
        self.backstack.push((self.active_tab_id(), self.active));
        self.active = next;
    }

    pub fn prev_tab(&mut self) {
        let prev = if self.active == 0 {
            self.tabs.len() - 1
        } else {
            self.active - 1
        };
        self.backstack.push((self.active_tab_id(), self.active));
        self.active = prev;
    }

    #[allow(dead_code)]
    pub fn push_pane(&mut self, pane: Pane) {
        self.tabs[self.active].panes.push(pane);
    }

    #[allow(dead_code)]
    pub fn pop_pane(&mut self) {
        self.tabs[self.active].panes.pop();
    }

    pub fn active_pane(&self) -> &Pane {
        self.tabs[self.active].panes.active_pane()
    }
}
