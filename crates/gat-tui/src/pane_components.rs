/// Tuirealm-based implementations of gat-tui panes
use tuirealm::{
    command::{Cmd, CmdResult},
    props::{Color, Style},
    ratatui::layout::{Constraint, Direction, Layout, Rect},
    ratatui::style::Stylize,
    ratatui::widgets::{Block, Borders, Paragraph},
    AttrValue, Attribute, Component, Event as TuiEvent, Frame, MockComponent, NoUserEvent, State,
};

// Dummy message type - pane components don't send messages
#[derive(Debug, Clone, PartialEq)]
pub struct PaneMsg;

/// Dashboard Pane - Status overview and quick actions
pub struct DashboardComponent;

impl MockComponent for DashboardComponent {
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

        // Status Card
        let status = Paragraph::new(
            "Status\n  Overall: healthy\n  Running: 1 workflow\n  Queued: 2 actions",
        )
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .style(Style::default().fg(Color::Green));
        frame.render_widget(status, chunks[0]);

        // Reliability Metrics
        let metrics = Paragraph::new("Reliability Metrics\n  ✓ Deliverability Score: 85.5%\n  ⚠ LOLE: 9.2 h/yr\n  ⚠ EUE: 15.3 MWh/yr")
            .block(Block::default().borders(Borders::ALL).title("Metrics"))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(metrics, chunks[1]);

        // Recent Runs (as text, not table)
        let runs = Paragraph::new("Recent Runs\n  ingest-2304       Succeeded  alice              42s\n  transform-7781    Running    ops                live\n  solve-9912        Pending    svc-derms          queued")
            .block(Block::default().borders(Borders::ALL).title("Recent Activity"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(runs, chunks[2]);

        // Quick Actions
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

impl Component<PaneMsg, NoUserEvent> for DashboardComponent {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<PaneMsg> {
        None
    }
}

/// Operations Pane - DERMS/ADMS/Batch operations
pub struct OperationsComponent;

impl MockComponent for OperationsComponent {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(4),
                Constraint::Min(1),
            ])
            .split(area);

        // DERMS/ADMS
        let derms = Paragraph::new("DERMS + ADMS\n  2 queued envelopes\n  1 stress-test running")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("DERMS/ADMS Queue"),
            )
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(derms, chunks[0]);

        // Batch Operations
        let batch = Paragraph::new("Batch Operations\n  Status: Ready\n  Active jobs: 0/4\n  Last run: scenarios_2024-11-21.json")
            .block(Block::default().borders(Borders::ALL).title("Batch Ops"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(batch, chunks[1]);

        // Allocation
        let alloc = Paragraph::new("Allocation Analysis\n  Available results:\n  • Congestion rents decomposition\n  • KPI contribution sensitivity")
            .block(Block::default().borders(Borders::ALL).title("Allocation"))
            .style(Style::default().fg(Color::Green));
        frame.render_widget(alloc, chunks[2]);

        // Summary
        let summary = Paragraph::new("Summary: 2 DERMS queued, Batch ready, Next: Dispatch")
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(summary, chunks[3]);
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

impl Component<PaneMsg, NoUserEvent> for OperationsComponent {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<PaneMsg> {
        None
    }
}

/// Datasets Pane - Data catalog and downloads
pub struct DatasetsComponent;

impl MockComponent for DatasetsComponent {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(5),
                Constraint::Min(1),
            ])
            .split(area);

        // Catalog
        let catalog = Paragraph::new("Data Catalog\n  OPSD snapshot\n  Airtravel tutorial")
            .block(Block::default().borders(Borders::ALL).title("Catalog"))
            .style(Style::default().fg(Color::Green));
        frame.render_widget(catalog, chunks[0]);

        // Workflows (as text)
        let workflows = Paragraph::new("Workflows\n  Ingest       Ready    just now\n  Transform    Idle     1m ago\n  Solve        Pending  3m ago")
            .block(Block::default().borders(Borders::ALL).title("Workflows"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(workflows, chunks[1]);

        // Downloads
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

impl Component<PaneMsg, NoUserEvent> for DatasetsComponent {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<PaneMsg> {
        None
    }
}

/// Pipeline Pane - Pipeline composition and transformation
pub struct PipelineComponent;

impl MockComponent for PipelineComponent {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(5),
                Constraint::Min(1),
            ])
            .split(area);

        // Source Selection
        let source = Paragraph::new("Source Selection\n  Radio: (•) Live telemetry stream\n  Dropdown: Dataset variant ↴\n  [Day-ahead | Real-time | Sandbox]")
            .block(Block::default().borders(Borders::ALL).title("Source"))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(source, chunks[0]);

        // Transforms
        let transforms = Paragraph::new("Transforms\n  Classic: Resample, Gap-fill, Forecast smoothing\n  Scenarios: Template materialization\n  Features: GNN, KPI, Geo features")
            .block(Block::default().borders(Borders::ALL).title("Transforms"))
            .style(Style::default().fg(Color::Magenta));
        frame.render_widget(transforms, chunks[1]);

        // Outputs
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

impl Component<PaneMsg, NoUserEvent> for PipelineComponent {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<PaneMsg> {
        None
    }
}

/// Commands Pane - gat-cli command snippets
pub struct CommandsComponent;

impl MockComponent for CommandsComponent {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(6),
                Constraint::Min(1),
            ])
            .split(area);

        // Instructions
        let instructions = Paragraph::new("Author gat-cli commands as snippets and run with hotkeys\nDry-runs print invocation; full runs stream output")
            .block(Block::default().borders(Borders::ALL).title("Workspace"))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(instructions, chunks[0]);

        // Snippets (as text)
        let snippets = Paragraph::new("Snippets\n  gat-cli datasets list --limit 5                    Verify connectivity\n  gat-cli derms envelope --grid-file <case>         Preview envelope\n  gat-cli dist import matpower --m <file>            Import test case")
            .block(Block::default().borders(Borders::ALL).title("Command Snippets"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(snippets, chunks[1]);

        // Recent Results
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

impl Component<PaneMsg, NoUserEvent> for CommandsComponent {
    fn on(&mut self, _ev: TuiEvent<NoUserEvent>) -> Option<PaneMsg> {
        None
    }
}
