use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::Duration;
use tuirealm::{
    application::PollStrategy,
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
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Esc, ..
            }) => Some(Msg::AppClose),
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

        let status = Paragraph::new("Status\n  Overall: healthy\n  Running: 1 workflow\n  Queued: 2 actions")
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

// Operations Component
pub struct OperationsPane {
    selected_index: usize,  // 0-3 for the 4 sections
}

impl OperationsPane {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
        }
    }
}

impl MockComponent for OperationsPane {
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

        // DERMS section
        let derms_style = if self.selected_index == 0 {
            Style::default().fg(Color::Cyan).bold()
        } else {
            Style::default().fg(Color::Cyan)
        };
        let derms = Paragraph::new("DERMS + ADMS\n  2 queued envelopes\n  1 stress-test running")
            .block(Block::default().borders(Borders::ALL).title("DERMS/ADMS Queue"))
            .style(derms_style);
        frame.render_widget(derms, chunks[0]);

        // Batch section
        let batch_style = if self.selected_index == 1 {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::Yellow)
        };
        let batch = Paragraph::new("Batch Operations\n  Status: Ready\n  Active jobs: 0/4\n  Last run: scenarios_2024-11-21.json")
            .block(Block::default().borders(Borders::ALL).title("Batch Ops"))
            .style(batch_style);
        frame.render_widget(batch, chunks[1]);

        // Allocation section
        let alloc_style = if self.selected_index == 2 {
            Style::default().fg(Color::Green).bold()
        } else {
            Style::default().fg(Color::Green)
        };
        let alloc = Paragraph::new("Allocation Analysis\n  Available results:\n  • Congestion rents decomposition\n  • KPI contribution sensitivity")
            .block(Block::default().borders(Borders::ALL).title("Allocation"))
            .style(alloc_style);
        frame.render_widget(alloc, chunks[2]);

        // Summary section
        let summary_style = if self.selected_index == 3 {
            Style::default().fg(Color::White).bold()
        } else {
            Style::default().fg(Color::White)
        };
        let summary = Paragraph::new("Summary: 2 DERMS queued, Batch ready, Next: Dispatch")
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .style(summary_style);
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

impl Component<Msg, NoUserEvent> for OperationsPane {
    fn on(&mut self, ev: TuiEvent<NoUserEvent>) -> Option<Msg> {
        match ev {
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Up, ..
            }) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                None
            }
            TuiEvent::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                if self.selected_index < 3 {
                    self.selected_index += 1;
                }
                None
            }
            _ => None,
        }
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
            .block(Block::default().borders(Borders::ALL).title("Recent Results"))
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

    // Create tuirealm application
    let event_listener = EventListenerCfg::default()
        .crossterm_input_listener(Duration::from_millis(20), 20);
    let mut app: Application<Id, Msg, NoUserEvent> = Application::init(event_listener);

    // Mount pane components
    app.mount(Id::Dashboard, Box::new(DashboardPane), vec![])?;
    app.mount(Id::Operations, Box::new(OperationsPane::new()), vec![])?;
    app.mount(Id::Datasets, Box::new(DatasetsPane), vec![])?;
    app.mount(Id::Pipeline, Box::new(PipelinePane), vec![])?;
    app.mount(Id::Commands, Box::new(CommandsPane), vec![])?;

    app.active(&Id::Dashboard)?;

    // Initialize terminal bridge
    let mut terminal = TerminalBridge::init(CrosstermTerminalAdapter::new()?)?;
    let mut current_pane = Id::Dashboard;
    let mut should_quit = false;

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

            // Header
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

            // Menu
            let menu_indicators = [
                if matches!(current_pane, Id::Dashboard) { "[*1]" } else { "[ 1]" },
                if matches!(current_pane, Id::Operations) { "[*2]" } else { "[ 2]" },
                if matches!(current_pane, Id::Datasets) { "[*3]" } else { "[ 3]" },
                if matches!(current_pane, Id::Pipeline) { "[*4]" } else { "[ 4]" },
                if matches!(current_pane, Id::Commands) { "[*5]" } else { "[ 5]" },
            ];
            let menu_text = format!(
                "{} Dashboard  {} Operations  {} Datasets  {} Pipeline  {} Commands  |  ESC/Q to quit",
                menu_indicators[0], menu_indicators[1], menu_indicators[2], menu_indicators[3], menu_indicators[4]
            );
            f.render_widget(
                Paragraph::new(menu_text)
                    .style(Style::default().fg(Color::White))
                    .block(Block::default().borders(Borders::BOTTOM)),
                chunks[1],
            );

            // Active pane content
            app.view(&current_pane, f, chunks[2]);
        })?;

        // Handle events
        match app.tick(PollStrategy::UpTo(1)) {
            Ok(messages) if !messages.is_empty() => {
                for msg in messages {
                    match msg {
                        Msg::AppClose => {
                            should_quit = true;
                        }
                        Msg::SwitchPane(pane) => {
                            current_pane = pane.clone();
                            app.active(&pane)?;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Cleanup
    terminal.restore()?;
    disable_raw_mode()?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}
