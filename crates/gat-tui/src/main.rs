use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::Duration;
use tuirealm::{
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent},
    props::{Color, Style},
    ratatui::layout::{Constraint, Direction, Layout, Rect},
    ratatui::style::Stylize,
    ratatui::widgets::{Block, Borders, Paragraph},
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
    Application, AttrValue, Attribute, Component, Event as TuiEvent, EventListenerCfg, Frame,
    MockComponent, NoUserEvent, Props, State,
};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Id {
    Dashboard,
    Operations,
    Datasets,
    Pipeline,
    Commands,
}

#[derive(Debug, PartialEq)]
pub enum Msg {
    AppClose,
    SwitchPane(Id),
}

// Navigation state: which level of the hierarchy are we at?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationLevel {
    MenuBar,     // Top level: selecting which pane
    PaneContent, // Inside a pane: selecting which section
}

// Pane-specific state
#[derive(Debug, Clone)]
pub struct DashboardPaneState {
    pub selected_index: usize, // 0-3 for the 4 sections
}

#[derive(Debug, Clone)]
pub struct OperationsPaneState {
    pub selected_index: usize, // 0-3 for the 4 sections
}

#[derive(Debug, Clone)]
pub struct DatasetsPaneState {
    pub selected_index: usize, // 0-2 for the 3 sections
}

#[derive(Debug, Clone)]
pub struct PipelinePaneState {
    pub selected_index: usize, // 0-2 for the 3 sections
}

#[derive(Debug, Clone)]
pub struct CommandsPaneState {
    pub selected_index: usize, // 0-2 for the 3 sections
}

// Header component
pub struct Header {
    props: Props,
}

impl Header {
    pub fn new(title: &str) -> Self {
        let mut props = Props::default();
        props.set(Attribute::Text, AttrValue::String(title.to_string()));
        Self { props }
    }
}

impl MockComponent for Header {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let text = self
            .props
            .get_or(Attribute::Text, AttrValue::String(String::default()))
            .unwrap_string();

        frame.render_widget(
            Paragraph::new(text)
                .style(Style::default().fg(Color::Cyan).bold())
                .block(Block::default().borders(Borders::BOTTOM)),
            area,
        );
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.props.get(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        self.props.set(attr, value);
    }

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for Header {
    fn on(&mut self, ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        match ev {
            TuiEvent::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::AppClose),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('1'),
                ..
            }) => Some(Msg::SwitchPane(Id::Dashboard)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('2'),
                ..
            }) => Some(Msg::SwitchPane(Id::Operations)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('3'),
                ..
            }) => Some(Msg::SwitchPane(Id::Datasets)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('4'),
                ..
            }) => Some(Msg::SwitchPane(Id::Pipeline)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('5'),
                ..
            }) => Some(Msg::SwitchPane(Id::Commands)),
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Char('q'),
                ..
            }) => Some(Msg::AppClose),
            _ => None,
        }
    }
}

// Dashboard Component
pub struct DashboardPane;

impl MockComponent for DashboardPane {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Length(7),
                Constraint::Length(4),
            ])
            .split(area);

        let status = Paragraph::new(
            "Status\n  Overall: healthy\n  Running: 1 workflow\n  Queued: 2 actions",
        )
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .style(Style::default().fg(Color::Green));
        frame.render_widget(status, chunks[0]);

        let metrics = Paragraph::new("Reliability Metrics\n  ✓ Deliverability Score: 85.5%\n  ⚠ LOLE: 9.2 h/yr\n  ⚠ EUE: 15.3 MWh/yr")
            .block(Block::default().borders(Borders::ALL).title("Metrics"))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(metrics, chunks[1]);

        let runs = Paragraph::new("Recent Runs\n  ingest-2304       Succeeded  alice              42s\n  transform-7781    Running    ops                live\n  solve-9912        Pending    svc-derms          queued")
            .block(Block::default().borders(Borders::ALL).title("Recent Activity"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(runs, chunks[2]);

        let actions = Paragraph::new("Quick Actions\n  [Enter] Run highlighted workflow\n  [R] Retry last failed step\n  [E] Edit config before dispatch")
            .block(Block::default().borders(Borders::ALL).title("Actions"))
            .style(Style::default().fg(Color::Magenta));
        frame.render_widget(actions, chunks[3]);
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for DashboardPane {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        None
    }
}

// Helper function to render KPI cards in a row
fn render_kpi_cards(frame: &mut Frame, area: Rect, is_selected: bool) {
    let card_style = if is_selected {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::Cyan)
    };

    // Create three columns for three KPI cards
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    // Deliverability Score (DS)
    let ds_content = "┌────────────────┐\n│ Deliverability │\n│     Score      │\n│    85.5% ✓     │\n└────────────────┘";
    let ds = Paragraph::new(ds_content)
        .style(card_style)
        .alignment(tuirealm::ratatui::layout::Alignment::Center);
    frame.render_widget(ds, chunks[0]);

    // Loss of Load Expectation (LOLE)
    let lole_content = "┌────────────────┐\n│     LOLE       │\n│   9.2 h/yr ⚠   │\n│ (Loss of Load) │\n└────────────────┘";
    let lole = Paragraph::new(lole_content)
        .style(card_style)
        .alignment(tuirealm::ratatui::layout::Alignment::Center);
    frame.render_widget(lole, chunks[1]);

    // Expected Unserved Energy (EUE)
    let eue_content = "┌────────────────┐\n│      EUE       │\n│ 15.3 MWh/yr ⚠  │\n│ (Unserved En.) │\n└────────────────┘";
    let eue = Paragraph::new(eue_content)
        .style(card_style)
        .alignment(tuirealm::ratatui::layout::Alignment::Center);
    frame.render_widget(eue, chunks[2]);
}

// Helper function to render menu bar with pane selection
fn render_menu_bar(frame: &mut Frame, area: Rect, current_pane: &Id, nav_level: NavigationLevel) {
    let panes = [
        ("Dashboard", Id::Dashboard),
        ("Operations", Id::Operations),
        ("Datasets", Id::Datasets),
        ("Pipeline", Id::Pipeline),
        ("Commands", Id::Commands),
    ];

    let mut menu_parts = Vec::new();

    for (_i, (name, id)) in panes.iter().enumerate() {
        let is_current = id == current_pane;
        let is_focused = is_current && nav_level == NavigationLevel::MenuBar;

        // Create styled pane name
        let pane_text = if is_focused {
            // Focused: highlighted with full formatting
            format!("[*] {}", name)
        } else if is_current {
            // Current but not focused: show it's active
            format!("[ ] {}", name)
        } else {
            // Other panes: plain
            format!("   {}", name)
        };

        // Style based on focus state
        let style = if is_focused {
            Style::default().fg(Color::Cyan).bold()
        } else if is_current {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        menu_parts.push((pane_text, style));
    }

    // Build the menu line with separators
    let mut menu_text = String::new();
    for (i, (name, _)) in menu_parts.iter().enumerate() {
        if i > 0 {
            menu_text.push_str("  │  ");
        }
        menu_text.push_str(name);
    }

    // Add help text
    menu_text.push_str("  |  ↑↓ navigate sections  ↑ go to menu  ESC menu");

    let default_style = if nav_level == NavigationLevel::MenuBar {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::White)
    };

    frame.render_widget(Paragraph::new(menu_text).style(default_style), area);
}

// Helper function to render Dashboard pane with state
fn render_dashboard_pane(frame: &mut Frame, area: Rect, state: &DashboardPaneState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Length(7),
            Constraint::Length(4),
        ])
        .split(area);

    // Status section
    let status_style = if state.selected_index == 0 {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Green)
    };
    let status =
        Paragraph::new("Status\n  Overall: healthy\n  Running: 1 workflow\n  Queued: 2 actions")
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(status_style);
    frame.render_widget(status, chunks[0]);

    // Metrics section with KPI cards
    let metrics_block_style = if state.selected_index == 1 {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::Cyan)
    };

    // Create inner layout for metrics block
    let metrics_area = chunks[1];
    let metrics_inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(metrics_area);

    // Render the metrics block border and title
    let metrics_block = Block::default()
        .borders(Borders::ALL)
        .title("Reliability Metrics")
        .style(metrics_block_style);
    frame.render_widget(metrics_block, metrics_area);

    // Render KPI cards inside the block (with some padding)
    let inner_padding = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(metrics_inner[1]);

    render_kpi_cards(frame, inner_padding[1], state.selected_index == 1);

    // Recent Activity section
    let activity_style = if state.selected_index == 2 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::Yellow)
    };
    let runs = Paragraph::new("Recent Runs\n  ingest-2304       Succeeded  alice              42s\n  transform-7781    Running    ops                live\n  solve-9912        Pending    svc-derms          queued")
        .block(Block::default().borders(Borders::ALL).title("Recent Activity"))
        .style(activity_style);
    frame.render_widget(runs, chunks[2]);

    // Actions section
    let actions_style = if state.selected_index == 3 {
        Style::default().fg(Color::Magenta).bold()
    } else {
        Style::default().fg(Color::Magenta)
    };
    let actions = Paragraph::new("Quick Actions\n  [Enter] Run highlighted workflow\n  [R] Retry last failed step\n  [E] Edit config before dispatch")
        .block(Block::default().borders(Borders::ALL).title("Actions"))
        .style(actions_style);
    frame.render_widget(actions, chunks[3]);
}

// Helper function to render Operations pane with state
fn render_operations_pane(frame: &mut Frame, area: Rect, state: &OperationsPaneState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(1),
        ])
        .split(area);

    // DERMS section
    let derms_style = if state.selected_index == 0 {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::Cyan)
    };
    let derms = Paragraph::new("DERMS + ADMS\n  2 queued envelopes\n  1 stress-test running")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("DERMS/ADMS Queue"),
        )
        .style(derms_style);
    frame.render_widget(derms, chunks[0]);

    // Batch section
    let batch_style = if state.selected_index == 1 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::Yellow)
    };
    let batch = Paragraph::new("Batch Operations\n  Status: Ready\n  Active jobs: 0/4\n  Last run: scenarios_2024-11-21.json")
        .block(Block::default().borders(Borders::ALL).title("Batch Ops"))
        .style(batch_style);
    frame.render_widget(batch, chunks[1]);

    // Allocation section
    let alloc_style = if state.selected_index == 2 {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Green)
    };
    let alloc = Paragraph::new("Allocation Analysis\n  Available results:\n  • Congestion rents decomposition\n  • KPI contribution sensitivity")
        .block(Block::default().borders(Borders::ALL).title("Allocation"))
        .style(alloc_style);
    frame.render_widget(alloc, chunks[2]);

    // Summary section
    let summary_style = if state.selected_index == 3 {
        Style::default().fg(Color::White).bold()
    } else {
        Style::default().fg(Color::White)
    };
    let summary = Paragraph::new("Summary: 2 DERMS queued, Batch ready, Next: Dispatch")
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .style(summary_style);
    frame.render_widget(summary, chunks[3]);
}

// Helper function to render Datasets pane with state
fn render_datasets_pane(frame: &mut Frame, area: Rect, state: &DatasetsPaneState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Min(1),
        ])
        .split(area);

    // Catalog section
    let catalog_style = if state.selected_index == 0 {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Green)
    };
    let catalog = Paragraph::new("Data Catalog\n  OPSD snapshot\n  Airtravel tutorial")
        .block(Block::default().borders(Borders::ALL).title("Catalog"))
        .style(catalog_style);
    frame.render_widget(catalog, chunks[0]);

    // Workflows section
    let workflows_style = if state.selected_index == 1 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::Yellow)
    };
    let workflows = Paragraph::new("Workflows\n  Ingest       Ready    just now\n  Transform    Idle     1m ago\n  Solve        Pending   3m ago")
        .block(Block::default().borders(Borders::ALL).title("Workflows"))
        .style(workflows_style);
    frame.render_widget(workflows, chunks[1]);

    // Downloads section
    let downloads_style = if state.selected_index == 2 {
        Style::default().fg(Color::DarkGray).bold()
    } else {
        Style::default().fg(Color::DarkGray).dim()
    };
    let downloads = Paragraph::new("No downloads queued\nRun a fetch to pull sample data")
        .block(Block::default().borders(Borders::ALL).title("Downloads"))
        .style(downloads_style);
    frame.render_widget(downloads, chunks[2]);
}

// Helper function to render Pipeline pane with state
fn render_pipeline_pane(frame: &mut Frame, area: Rect, state: &PipelinePaneState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Min(1),
        ])
        .split(area);

    // Source section
    let source_style = if state.selected_index == 0 {
        Style::default().fg(Color::Cyan).bold()
    } else {
        Style::default().fg(Color::Cyan)
    };
    let source = Paragraph::new("Source Selection\n  Radio: (•) Live telemetry stream\n  Dropdown: Dataset variant ↴\n  [Day-ahead | Real-time | Sandbox]")
        .block(Block::default().borders(Borders::ALL).title("Source"))
        .style(source_style);
    frame.render_widget(source, chunks[0]);

    // Transforms section
    let transforms_style = if state.selected_index == 1 {
        Style::default().fg(Color::Magenta).bold()
    } else {
        Style::default().fg(Color::Magenta)
    };
    let transforms = Paragraph::new("Transforms\n  Classic: Resample, Gap-fill, Forecast smoothing\n  Scenarios: Template materialization\n  Features: GNN, KPI, Geo features")
        .block(Block::default().borders(Borders::ALL).title("Transforms"))
        .style(transforms_style);
    frame.render_widget(transforms, chunks[1]);

    // Outputs section
    let outputs_style = if state.selected_index == 2 {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Green)
    };
    let outputs = Paragraph::new("Outputs: Warehouse table, DERMS feed, Notebook\nDelivery: Single run report or Continuous subscription")
        .block(Block::default().borders(Borders::ALL).title("Outputs"))
        .style(outputs_style);
    frame.render_widget(outputs, chunks[2]);
}

// Helper function to render Commands pane with state
fn render_commands_pane(frame: &mut Frame, area: Rect, state: &CommandsPaneState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(6),
            Constraint::Min(1),
        ])
        .split(area);

    // Workspace section
    let workspace_style = if state.selected_index == 0 {
        Style::default().fg(Color::Yellow).bold()
    } else {
        Style::default().fg(Color::Yellow)
    };
    let instructions = Paragraph::new("Author gat-cli commands as snippets and run with hotkeys\nDry-runs print invocation; full runs stream output")
        .block(Block::default().borders(Borders::ALL).title("Workspace"))
        .style(workspace_style);
    frame.render_widget(instructions, chunks[0]);

    // Snippets section
    let snippets_style = if state.selected_index == 1 {
        Style::default().fg(Color::White).bold()
    } else {
        Style::default().fg(Color::White)
    };
    let snippets = Paragraph::new("Snippets\n  gat-cli datasets list --limit 5                    Verify connectivity\n  gat-cli derms envelope --grid-file <case>         Preview envelope\n  gat-cli dist import matpower --m <file>            Import test case")
        .block(Block::default().borders(Borders::ALL).title("Command Snippets"))
        .style(snippets_style);
    frame.render_widget(snippets, chunks[1]);

    // Recent Results section
    let recent_style = if state.selected_index == 2 {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::Green)
    };
    let recent = Paragraph::new("Recent: ✔ datasets list (5 rows), ✔ envelope preview")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recent Results"),
        )
        .style(recent_style);
    frame.render_widget(recent, chunks[2]);
}

// Operations Component (stateless in tuirealm)
pub struct OperationsPane;

impl MockComponent for OperationsPane {
    fn view(&mut self, _frame: &mut Frame, _area: Rect) {
        // State is managed in main, rendered separately
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for OperationsPane {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        None
    }
}

// Datasets Component
pub struct DatasetsPane;

impl MockComponent for DatasetsPane {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(1),
            ])
            .split(area);

        let catalog = Paragraph::new("Data Catalog\n  OPSD snapshot\n  Airtravel tutorial")
            .block(Block::default().borders(Borders::ALL).title("Catalog"))
            .style(Style::default().fg(Color::Green));
        frame.render_widget(catalog, chunks[0]);

        let workflows = Paragraph::new("Workflows\n  Ingest       Ready    just now\n  Transform    Idle     1m ago\n  Solve        Pending  3m ago")
            .block(Block::default().borders(Borders::ALL).title("Workflows"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(workflows, chunks[1]);

        let downloads = Paragraph::new("No downloads queued\nRun a fetch to pull sample data")
            .block(Block::default().borders(Borders::ALL).title("Downloads"))
            .style(Style::default().fg(Color::DarkGray).dim());
        frame.render_widget(downloads, chunks[2]);
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for DatasetsPane {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        None
    }
}

// Pipeline Component
pub struct PipelinePane;

impl MockComponent for PipelinePane {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Min(1),
            ])
            .split(area);

        let source = Paragraph::new("Source Selection\n  Radio: (•) Live telemetry stream\n  Dropdown: Dataset variant ↴\n  [Day-ahead | Real-time | Sandbox]")
            .block(Block::default().borders(Borders::ALL).title("Source"))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(source, chunks[0]);

        let transforms = Paragraph::new("Transforms\n  Classic: Resample, Gap-fill, Forecast smoothing\n  Scenarios: Template materialization\n  Features: GNN, KPI, Geo features")
            .block(Block::default().borders(Borders::ALL).title("Transforms"))
            .style(Style::default().fg(Color::Magenta));
        frame.render_widget(transforms, chunks[1]);

        let outputs = Paragraph::new("Outputs: Warehouse table, DERMS feed, Notebook\nDelivery: Single run report or Continuous subscription")
            .block(Block::default().borders(Borders::ALL).title("Outputs"))
            .style(Style::default().fg(Color::Green));
        frame.render_widget(outputs, chunks[2]);
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for PipelinePane {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        None
    }
}

// Commands Component
pub struct CommandsPane;

impl MockComponent for CommandsPane {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(6),
                Constraint::Min(1),
            ])
            .split(area);

        let instructions = Paragraph::new("Author gat-cli commands as snippets and run with hotkeys\nDry-runs print invocation; full runs stream output")
            .block(Block::default().borders(Borders::ALL).title("Workspace"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(instructions, chunks[0]);

        let snippets = Paragraph::new("Snippets\n  gat-cli datasets list --limit 5                    Verify connectivity\n  gat-cli derms envelope --grid-file <case>         Preview envelope\n  gat-cli dist import matpower --m <file>            Import test case")
            .block(Block::default().borders(Borders::ALL).title("Command Snippets"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(snippets, chunks[1]);

        let recent = Paragraph::new("Recent: ✔ datasets list (5 rows), ✔ envelope preview")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Recent Results"),
            )
            .style(Style::default().fg(Color::Green));
        frame.render_widget(recent, chunks[2]);
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, _: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for CommandsPane {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Create tuirealm application without automatic event listener
    // We'll handle events manually via crossterm
    let mut app: Application<Id, Msg, NoUserEvent> = Application::init(EventListenerCfg::default());

    // Mount pane components
    app.mount(Id::Dashboard, Box::new(DashboardPane), vec![])?;
    app.mount(Id::Operations, Box::new(OperationsPane), vec![])?;
    app.mount(Id::Datasets, Box::new(DatasetsPane), vec![])?;
    app.mount(Id::Pipeline, Box::new(PipelinePane), vec![])?;
    app.mount(Id::Commands, Box::new(CommandsPane), vec![])?;

    app.active(&Id::Dashboard)?;

    // Initialize terminal bridge
    let mut terminal = TerminalBridge::init(CrosstermTerminalAdapter::new()?)?;
    let mut current_pane = Id::Dashboard;
    let mut should_quit = false;
    let mut nav_level = NavigationLevel::MenuBar; // Start at menu bar

    // Pane-specific state
    let mut dashboard_state = DashboardPaneState { selected_index: 0 };
    let mut operations_state = OperationsPaneState { selected_index: 0 };
    let mut datasets_state = DatasetsPaneState { selected_index: 0 };
    let mut pipeline_state = PipelinePaneState { selected_index: 0 };
    let mut commands_state = CommandsPaneState { selected_index: 0 };

    // Main loop
    while !should_quit {
        // Render
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Min(5),
                ])
                .split(f.area());

            // Title
            let pane_name = match current_pane {
                Id::Dashboard => "Dashboard - Status overview and quick actions",
                Id::Operations => "Operations - DERMS/ADMS/Batch operations",
                Id::Datasets => "Datasets - Data catalog and workflows",
                Id::Pipeline => "Pipeline - Transformation and composition",
                Id::Commands => "Commands - gat-cli snippets and execution",
            };
            let header_text = format!("GAT TUI - {}", pane_name);
            f.render_widget(
                Paragraph::new(header_text)
                    .style(Style::default().fg(Color::Cyan).bold())
                    .block(Block::default().borders(Borders::BOTTOM)),
                chunks[0],
            );

            // Menu bar with pane selection
            render_menu_bar(f, chunks[1], &current_pane, nav_level);

            // Active pane content
            match current_pane {
                Id::Dashboard => render_dashboard_pane(f, chunks[2], &dashboard_state),
                Id::Operations => render_operations_pane(f, chunks[2], &operations_state),
                Id::Datasets => render_datasets_pane(f, chunks[2], &datasets_state),
                Id::Pipeline => render_pipeline_pane(f, chunks[2], &pipeline_state),
                Id::Commands => render_commands_pane(f, chunks[2], &commands_state),
            }
        })?;

        // Poll for crossterm events
        if event::poll(Duration::from_millis(20))? {
            if let Event::Key(key) = event::read()? {
                match nav_level {
                    NavigationLevel::MenuBar => {
                        // At menu bar: navigate between panes with Left/Right, enter with Down
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                should_quit = true;
                            }
                            KeyCode::Left => {
                                // Switch to previous pane
                                current_pane = match current_pane {
                                    Id::Dashboard => Id::Commands,
                                    Id::Operations => Id::Dashboard,
                                    Id::Datasets => Id::Operations,
                                    Id::Pipeline => Id::Datasets,
                                    Id::Commands => Id::Pipeline,
                                };
                                app.active(&current_pane)?;
                            }
                            KeyCode::Right => {
                                // Switch to next pane
                                current_pane = match current_pane {
                                    Id::Dashboard => Id::Operations,
                                    Id::Operations => Id::Datasets,
                                    Id::Datasets => Id::Pipeline,
                                    Id::Pipeline => Id::Commands,
                                    Id::Commands => Id::Dashboard,
                                };
                                app.active(&current_pane)?;
                            }
                            KeyCode::Down | KeyCode::Enter => {
                                // Enter pane content
                                nav_level = NavigationLevel::PaneContent;
                            }
                            _ => {}
                        }
                    }
                    NavigationLevel::PaneContent => {
                        // Inside a pane: navigate sections with Up/Down, exit with ESC
                        match key.code {
                            KeyCode::Esc => {
                                // Return to menu bar
                                nav_level = NavigationLevel::MenuBar;
                            }
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                // Allow Q to quit from anywhere
                                should_quit = true;
                            }
                            KeyCode::Up => {
                                // Navigate up within pane, or go to menu bar if at top
                                let at_top = match current_pane {
                                    Id::Dashboard => dashboard_state.selected_index == 0,
                                    Id::Operations => operations_state.selected_index == 0,
                                    Id::Datasets => datasets_state.selected_index == 0,
                                    Id::Pipeline => pipeline_state.selected_index == 0,
                                    Id::Commands => commands_state.selected_index == 0,
                                };

                                if at_top {
                                    // At top of pane, go to menu bar
                                    nav_level = NavigationLevel::MenuBar;
                                } else {
                                    // Navigate up within pane
                                    match current_pane {
                                        Id::Dashboard => {
                                            if dashboard_state.selected_index > 0 {
                                                dashboard_state.selected_index -= 1;
                                            }
                                        }
                                        Id::Operations => {
                                            if operations_state.selected_index > 0 {
                                                operations_state.selected_index -= 1;
                                            }
                                        }
                                        Id::Datasets => {
                                            if datasets_state.selected_index > 0 {
                                                datasets_state.selected_index -= 1;
                                            }
                                        }
                                        Id::Pipeline => {
                                            if pipeline_state.selected_index > 0 {
                                                pipeline_state.selected_index -= 1;
                                            }
                                        }
                                        Id::Commands => {
                                            if commands_state.selected_index > 0 {
                                                commands_state.selected_index -= 1;
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                // Navigate down within pane
                                match current_pane {
                                    Id::Dashboard => {
                                        if dashboard_state.selected_index < 3 {
                                            dashboard_state.selected_index += 1;
                                        }
                                    }
                                    Id::Operations => {
                                        if operations_state.selected_index < 3 {
                                            operations_state.selected_index += 1;
                                        }
                                    }
                                    Id::Datasets => {
                                        if datasets_state.selected_index < 2 {
                                            datasets_state.selected_index += 1;
                                        }
                                    }
                                    Id::Pipeline => {
                                        if pipeline_state.selected_index < 2 {
                                            pipeline_state.selected_index += 1;
                                        }
                                    }
                                    Id::Commands => {
                                        if commands_state.selected_index < 2 {
                                            commands_state.selected_index += 1;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    // Cleanup
    terminal.restore()?;
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}
