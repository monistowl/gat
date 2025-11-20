use std::collections::VecDeque;

const CMD_DIST_IMPORT_MATPOWER: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "dist",
    "import",
    "matpower",
    "--m",
    "test_data/derms/ieee33/case33bw.m",
    "--output",
    "test_data/derms/ieee33/case33bw.arrow",
];
const CMD_DERMS_ENVELOPE: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "derms",
    "envelope",
    "--grid-file",
    "test_data/derms/ieee33/case33bw.arrow",
    "--assets",
    "test_data/derms/ieee33/ieee33/IEEE33_dataset.csv",
    "--out",
    "docs/derms/envelope.parquet",
];
const CMD_DERMS_SCHEDULE: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "derms",
    "schedule",
    "--assets",
    "test_data/derms/ieee33/der_assets.parquet",
    "--price-series",
    "test_data/derms/ieee33/prices.parquet",
    "--out",
    "docs/derms/schedule.parquet",
];
const CMD_DERMS_STRESS: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "derms",
    "stress-test",
    "--assets",
    "test_data/derms/ieee33/der_assets.parquet",
    "--price-series",
    "test_data/derms/ieee33/prices.parquet",
    "--output-dir",
    "docs/derms/stress",
    "--scenarios",
    "8",
];
const CMD_ADMS_FLISR: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "adms",
    "flisr-sim",
    "--grid-file",
    "test_data/derms/ieee33/case33bw.arrow",
    "--reliability",
    "test_data/derms/ieee33/reliability.parquet",
    "--output-dir",
    "docs/adms/flisr",
    "--scenarios",
    "3",
];
const CMD_ADMS_VVO: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "adms",
    "vvo-plan",
    "--grid-file",
    "test_data/derms/ieee33/case33bw.arrow",
    "--output-dir",
    "docs/adms/vvo",
    "--day-types",
    "low,high",
];
const CMD_ADMS_OUTAGE: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "adms",
    "outage-mc",
    "--reliability",
    "test_data/derms/ieee33/reliability.parquet",
    "--output-dir",
    "docs/adms/outage",
    "--samples",
    "20",
    "--seed",
    "42",
];
const CMD_ADMS_STATE_EST: &[&str] = &[
    "cargo",
    "run",
    "-p",
    "gat-cli",
    "--",
    "adms",
    "state-estimation",
    "--grid-file",
    "test_data/derms/ieee33/case33bw.arrow",
    "--measurements",
    "test_data/se/measurements.csv",
    "--out",
    "docs/adms/state_est.parquet",
    "--state-out",
    "docs/adms/estimated_state.parquet",
    "--solver",
    "gauss",
];

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
pub struct MenuAction {
    pub key: char,
    pub label: &'static str,
    pub detail: &'static str,
    pub command: &'static [&'static str],
}

impl MenuAction {
    pub const fn new(
        key: char,
        label: &'static str,
        detail: &'static str,
        command: &'static [&'static str],
    ) -> Self {
        Self {
            key,
            label,
            detail,
            command,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tab {
    pub id: TabId,
    pub label: &'static str,
    pub menu_actions: Vec<MenuAction>,
    pub panes: PaneStack,
}

impl Tab {
    pub fn new(id: TabId, label: &'static str, menu_actions: Vec<MenuAction>, pane: Pane) -> Self {
        Tab {
            id,
            label,
            menu_actions,
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
                vec![],
                Pane::new("Workflow overview", "Monitor workflows and live runs"),
            ),
            Tab::new(
                TabId::Derms,
                "DERMS",
                vec![
                    MenuAction::new(
                        '0',
                        "Import MATPOWER",
                        "Generate case33bw.arrow so DERMS/ADMS commands have a grid",
                        CMD_DIST_IMPORT_MATPOWER,
                    ),
                    MenuAction::new(
                        '1',
                        "DERMS envelope",
                        "Summarize P/Q/S flexibility for each aggregator",
                        CMD_DERMS_ENVELOPE,
                    ),
                    MenuAction::new(
                        '2',
                        "DERMS schedule",
                        "Heuristic dispatch schedule + curtailment metrics",
                        CMD_DERMS_SCHEDULE,
                    ),
                    MenuAction::new(
                        '3',
                        "DERMS stress-test",
                        "Run Monte Carlo price perturbations to validate robustness",
                        CMD_DERMS_STRESS,
                    ),
                ],
                Pane::new("DERMS queue", "Pending DERMS workflows"),
            ),
            Tab::new(
                TabId::Adms,
                "ADMS",
                vec![
                    MenuAction::new(
                        '1',
                        "ADMS FLISR sim",
                        "Simulate reliability runs and capture SAIDI/SAIFI/CAIDI",
                        CMD_ADMS_FLISR,
                    ),
                    MenuAction::new(
                        '2',
                        "ADMS VVO plan",
                        "Produce tap/VAR recommendations for low/high day types",
                        CMD_ADMS_VVO,
                    ),
                    MenuAction::new(
                        '3',
                        "ADMS outage MC",
                        "Monte Carlo outage stats referencing the reliability catalog",
                        CMD_ADMS_OUTAGE,
                    ),
                    MenuAction::new(
                        '4',
                        "ADMS state-estimation",
                        "Run WLS state estimation and materialize state artifacts",
                        CMD_ADMS_STATE_EST,
                    ),
                ],
                Pane::new("ADMS command center", "Co-sim insights and tooling"),
            ),
            Tab::new(
                TabId::Config,
                "Config",
                vec![],
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

    pub fn active_actions(&self) -> &[MenuAction] {
        &self.tabs[self.active].menu_actions
    }

    pub fn set_detail(&mut self, title: Option<String>, body: Option<String>) {
        self.detail = DetailPayload { title, body };
    }

    pub fn detail(&self) -> &DetailPayload {
        &self.detail
    }

    pub fn action_for_key(&self, key: char) -> Option<&MenuAction> {
        self.active_actions()
            .iter()
            .find(|action| action.key == key)
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
