use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use gat_core::{Branch, BranchId, Bus, BusId, Edge as CoreEdge, Network, Node as CoreNode};
use gat_viz::layout::layout_network;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine, Points};
use ratatui::widgets::Clear;
use ratatui::widgets::{
    Axis, Block, Borders, Cell, Chart, Dataset, Gauge, Paragraph, Row, Table, Wrap,
};
use ratatui::{backend::Backend, Frame, Terminal};
use ratatui_code_editor::editor::Editor;
use ratatui_code_editor::theme;
use ratatui_explorer::{FileExplorer, Input, Theme};
use shlex::split;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

mod demo_stats;
use demo_stats::DemoStats;

/// Represents one of the key GAT workflow stages shown by the UI.
struct Workflow {
    name: &'static str,
    stage: &'static str,
    status: &'static str,
    detail: &'static str,
}

#[derive(Clone)]
struct ControlSettings {
    poll_secs: u64,
    solver: SolverMode,
    verbose: bool,
    command: Vec<String>,
}

#[derive(Clone, Copy)]
enum SolverMode {
    Gauss,
    Clarabel,
    Highs,
}

impl SolverMode {
    fn next(self) -> Self {
        match self {
            SolverMode::Gauss => SolverMode::Clarabel,
            SolverMode::Clarabel => SolverMode::Highs,
            SolverMode::Highs => SolverMode::Gauss,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            SolverMode::Gauss => "Gauss",
            SolverMode::Clarabel => "Clarabel",
            SolverMode::Highs => "Highs",
        }
    }
}

struct Preset {
    name: &'static str,
    description: &'static str,
    settings: ControlSettings,
}

struct ConfigSnapshot {
    source: String,
    entries: Vec<(String, String)>,
    status: String,
}

impl ConfigSnapshot {
    fn load(path: Option<PathBuf>) -> Self {
        let resolved_path = path.or_else(default_config_path);
        let (source, entries, status) = if let Some(path) = resolved_path {
            match fs::read_to_string(&path) {
                Ok(text) => {
                    let entries: Vec<(String, String)> = text
                        .lines()
                        .filter_map(|line| {
                            let trimmed = line.trim();
                            if trimmed.is_empty() || trimmed.starts_with('#') {
                                return None;
                            }
                            let mut parts = trimmed.splitn(2, '=');
                            let key = parts.next()?.trim();
                            let value = parts.next().unwrap_or("").trim();
                            Some((key.to_string(), value.to_string()))
                        })
                        .collect();
                    let total = entries.len();
                    (
                        path.display().to_string(),
                        entries,
                        format!("Loaded {} values", total),
                    )
                }
                Err(err) => (
                    path.display().to_string(),
                    Vec::new(),
                    format!("Failed to load config: {}", err),
                ),
            }
        } else {
            (
                "embedded defaults".to_string(),
                vec![
                    ("workspace".to_string(), "gat".to_string()),
                    ("preset".to_string(), "cournot".to_string()),
                ],
                "Using built-in demo config".to_string(),
            )
        };
        ConfigSnapshot {
            source,
            entries,
            status,
        }
    }
}

fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("gat-tui").join("config.toml"))
}

struct LiveRunHandle {
    receiver: Receiver<String>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl LiveRunHandle {
    fn poll(&mut self) -> (Vec<String>, bool) {
        let mut lines = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(line) => lines.push(line),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if let Some(handle) = self.join_handle.take() {
                        let _ = handle.join();
                    }
                    return (lines, true);
                }
            }
        }
        (lines, false)
    }
}

impl ControlSettings {
    fn default_command() -> Vec<String> {
        vec![
            "cargo".into(),
            "run".into(),
            "-p".into(),
            "gat-cli".into(),
            "--".into(),
            "--help".into(),
        ]
    }

    fn default() -> Self {
        ControlSettings {
            poll_secs: 1,
            solver: SolverMode::Gauss,
            verbose: false,
            command: Self::default_command(),
        }
    }
}

fn default_presets() -> Vec<Preset> {
    vec![
        Preset {
            name: "Baseline",
            description: "Quick poll + Gauss solver (default)",
            settings: ControlSettings::default(),
        },
        Preset {
            name: "Cournot Demo",
            description: "Slower polls, Clarabel focus and verbose logs",
            settings: ControlSettings {
                poll_secs: 2,
                solver: SolverMode::Clarabel,
                verbose: true,
                command: ControlSettings::default_command(),
            },
        },
        Preset {
            name: "Dispatch Check",
            description: "Highs solver with extra breathing room",
            settings: ControlSettings {
                poll_secs: 3,
                solver: SolverMode::Highs,
                verbose: false,
                command: ControlSettings::default_command(),
            },
        },
    ]
}

pub struct App {
    workflows: Vec<Workflow>,
    selected: usize,
    logs: VecDeque<String>,
    demo_stats: DemoStats,
    layout_preview: LayoutPreview,
    control: ControlSettings,
    presets: Vec<Preset>,
    active_preset: usize,
    custom_override: bool,
    config_snapshot: ConfigSnapshot,
    config_explorer: FileExplorer,
    config_explorer_active: bool,
    live_run_handle: Option<LiveRunHandle>,
    live_run_status: Option<String>,
    show_help: bool,
    command_editor: Editor,
    command_editor_visible: bool,
    command_editor_area: Option<Rect>,
}

pub trait EventSource {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool>;
    fn read(&mut self) -> io::Result<Event>;
}

pub struct CrosstermEventSource;

impl EventSource for CrosstermEventSource {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<Event> {
        event::read()
    }
}

impl App {
    #[allow(dead_code)]
    pub fn new() -> Self {
        let workflows = vec![
            Workflow {
                name: "Import & Validate",
                stage: "Data",
                status: "Ready",
                detail: "psse/matpower → Arrow → schema",
            },
            Workflow {
                name: "DC/AC power flow",
                stage: "Simulation",
                status: "Live",
                detail: "B'θ = P & Newton loops",
            },
            Workflow {
                name: "OPF batching",
                stage: "Dispatch",
                status: "Queued",
                detail: "LP cost minimization",
            },
            Workflow {
                name: "Contingency sweep",
                stage: "Screening",
                status: "Cloned",
                detail: "N-1 fan-out",
            },
        ];
        let mut logs = VecDeque::with_capacity(5);
        logs.push_back("gat-tui ready → q to quit, arrows to explore".into());
        let layout_preview = LayoutPreview::from_network(build_demo_network());
        let presets = default_presets();
        let control = presets[0].settings.clone();
        let command_initial = control.command.join(" ");
        let command_editor = Editor::new("shell", &command_initial, theme::vesper());
        let explorer_theme = Theme::default()
            .add_default_title()
            .with_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Config explorer"),
            )
            .with_highlight_item_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )
            .with_highlight_dir_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .with_highlight_symbol("→ ".into());
        let config_explorer = FileExplorer::with_theme(explorer_theme)
            .unwrap_or_else(|err| panic!("failed to initialize config explorer: {err}"));
        let mut app = Self {
            workflows,
            selected: 0,
            logs,
            demo_stats: DemoStats::load_default(),
            layout_preview,
            control,
            presets,
            active_preset: 0,
            custom_override: false,
            config_snapshot: ConfigSnapshot::load(None),
            config_explorer,
            config_explorer_active: false,
            live_run_handle: None,
            live_run_status: None,
            show_help: false,
            command_editor,
            command_editor_visible: false,
            command_editor_area: None,
        };
        app.reload_config(None);
        app
    }

    fn next(&mut self) {
        if self.selected + 1 < self.workflows.len() {
            self.selected += 1;
            self.push_log("Moved selection down");
        }
    }

    fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.push_log("Moved selection up");
        }
    }

    fn tick(&mut self) {
        if self.logs.len() == 5 {
            self.logs.pop_front();
        }
        let timestamp = Local::now().format("%H:%M:%S");
        let workflow = &self.workflows[self.selected];
        self.logs.push_back(format!(
            "{} | {} on stage {}",
            timestamp, workflow.name, workflow.stage
        ));
        if self.control.verbose {
            self.logs
                .push_back("Verbose monitor is collecting extra context".into());
            if self.logs.len() > 5 {
                self.logs.pop_front();
            }
        }
        if !self.demo_stats.records().is_empty() {
            self.push_log(&format!(
                "demo avg price {:.1} $/MWh",
                self.demo_stats.avg_price()
            ));
        }
    }

    fn push_log(&mut self, entry: &str) {
        if self.logs.len() == 5 {
            self.logs.pop_front();
        }
        self.logs.push_back(entry.to_string());
    }

    fn demo_summary(&self) -> String {
        self.demo_stats.summary()
    }

    fn gauge_metrics(&self) -> Vec<(&'static str, f64)> {
        self.demo_stats.gauge_metrics()
    }

    fn stage_graph(&self) -> Vec<Line<'static>> {
        let mut spans = Vec::new();
        let stages = ["Data", "Simulation", "Dispatch", "Screening"];
        for stage in stages {
            let style = if self.workflows[self.selected].stage == stage {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            spans.push(Line::from(vec![Span::styled(stage, style)]));
        }
        spans
    }

    fn reload_config(&mut self, path: Option<PathBuf>) {
        self.config_snapshot = ConfigSnapshot::load(path);
        self.push_log(&format!("Config: {}", self.config_snapshot.status));
    }

    fn adjust_poll_rate(&mut self, delta: i64) {
        let new = (self.control.poll_secs as i64 + delta).max(1) as u64;
        if new != self.control.poll_secs {
            self.control.poll_secs = new;
            self.mark_custom();
            self.push_log(&format!("Poll interval {}s", new));
        }
    }

    fn cycle_solver(&mut self) {
        self.control.solver = self.control.solver.next();
        self.mark_custom();
        self.push_log(&format!("Solver {}", self.control.solver.as_str()));
    }

    fn toggle_verbose(&mut self) {
        self.control.verbose = !self.control.verbose;
        self.mark_custom();
        self.push_log(&format!(
            "Verbose logging {}",
            if self.control.verbose {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }

    fn cycle_preset(&mut self, forward: bool) {
        let len = self.presets.len();
        if len == 0 {
            return;
        }
        let next = if forward {
            (self.active_preset + 1) % len
        } else if self.active_preset == 0 {
            len - 1
        } else {
            self.active_preset - 1
        };
        self.active_preset = next;
        self.control = self.presets[next].settings.clone();
        self.custom_override = false;
        self.push_log(&format!("Applied preset {}", self.presets[next].name));
    }

    fn mark_custom(&mut self) {
        if !self.custom_override {
            self.custom_override = true;
            self.push_log("Switched to custom settings");
        }
    }

    fn start_live_run(&mut self) {
        if self.live_run_handle.is_some() {
            self.push_log("Live run already in progress");
            return;
        }
        if self.control.command.is_empty() {
            self.push_log("No live-run command configured");
            return;
        }
        let command_parts = self.control.command.clone();
        let summary = command_parts.join(" ");
        self.push_log(&format!("Starting live run: {}", summary));
        let (sender, receiver) = mpsc::channel();
        let handle = thread::spawn(move || {
            sender.send(format!("> {}", summary)).ok();
            let mut cmd = Command::new(&command_parts[0]);
            for arg in command_parts.iter().skip(1) {
                cmd.arg(arg);
            }
            match cmd.output() {
                Ok(output) => {
                    for line in String::from_utf8_lossy(&output.stdout).lines() {
                        sender.send(line.to_string()).ok();
                    }
                    for line in String::from_utf8_lossy(&output.stderr).lines() {
                        sender.send(line.to_string()).ok();
                    }
                    sender
                        .send(format!("Live run exited with {}", output.status))
                        .ok();
                }
                Err(err) => {
                    sender.send(format!("Live run failed: {}", err)).ok();
                }
            }
            drop(sender);
        });
        self.live_run_handle = Some(LiveRunHandle {
            receiver,
            join_handle: Some(handle),
        });
        self.live_run_status = Some("Running...".into());
    }

    fn poll_live_run(&mut self) {
        if let Some(handle) = &mut self.live_run_handle {
            let (lines, finished) = handle.poll();
            for line in lines {
                self.push_log(&line);
                self.live_run_status = Some(line);
            }
            if finished {
                self.live_run_handle = None;
                if self.live_run_status.is_none() {
                    self.live_run_status = Some("Live run complete".into());
                }
            }
        }
    }

    fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        self.push_log(if self.show_help {
            "Help drawer opened"
        } else {
            "Help drawer closed"
        });
    }

    fn toggle_config_explorer(&mut self) {
        self.config_explorer_active = !self.config_explorer_active;
        self.push_log(if self.config_explorer_active {
            "Config explorer active (Use Enter to load selection)"
        } else {
            "Config explorer hidden"
        });
    }

    fn control_command_summary(&self) -> String {
        self.control.command.join(" ")
    }

    fn preset_label(&self) -> &str {
        if self.custom_override {
            "Custom"
        } else {
            &self.presets[self.active_preset].name
        }
    }

    fn preset_description(&self) -> &str {
        if self.custom_override {
            "Manual overrides applied"
        } else {
            &self.presets[self.active_preset].description
        }
    }

    fn live_run_summary(&self) -> &str {
        if let Some(status) = &self.live_run_status {
            status
        } else {
            "Ready for live runs"
        }
    }

    fn config_rows(&self, max: usize) -> Vec<(String, String)> {
        self.config_snapshot
            .entries
            .iter()
            .take(max)
            .cloned()
            .collect()
    }

    fn open_command_editor(&mut self) {
        let text = self.control.command.join(" ");
        self.command_editor.set_content(&text);
        self.command_editor_visible = true;
        self.command_editor_area = None;
        self.push_log("Command editor open (Ctrl+S to save)");
    }

    fn apply_command_editor(&mut self) {
        let content = self.command_editor.get_content();
        match split(&content) {
            Some(parts) if !parts.is_empty() => {
                self.control.command = parts;
                self.mark_custom();
                self.push_log("Command updated from editor");
            }
            Some(_) => {
                self.push_log("Command editor is empty; no change");
            }
            None => {
                self.push_log("Failed to parse command text");
            }
        }
        self.command_editor_visible = false;
        self.command_editor_area = None;
    }

    fn cancel_command_editor(&mut self) {
        self.command_editor_visible = false;
        self.command_editor_area = None;
        self.push_log("Command editor closed without saving");
    }
}

impl Default for App {
    fn default() -> Self {
        App::new()
    }
}

pub fn run_tui<B, I>(terminal: &mut Terminal<B>, event_source: &mut I, app: &mut App) -> Result<()>
where
    B: Backend,
    I: EventSource,
{
    let mut last_tick = Instant::now();
    loop {
        app.poll_live_run();
        terminal.draw(|f| draw_ui(f, app))?;
        let tick_rate = Duration::from_secs(app.control.poll_secs.max(1));
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event_source.poll(timeout)? {
            let event = event_source.read()?;
            if let Event::Key(key) = event {
                let cloned_event = Event::Key(key.clone());
                let input_event = Input::from(&cloned_event);
                if app.command_editor_visible {
                    match key.code {
                        KeyCode::Esc => {
                            app.cancel_command_editor();
                        }
                        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.apply_command_editor();
                        }
                        _ => {
                            if let Some(area) = app.command_editor_area {
                                let _ = app.command_editor.input(key, &area);
                            }
                        }
                    }
                    continue;
                }
                if app.config_explorer_active {
                    if let Err(err) = app.config_explorer.handle(input_event) {
                        app.push_log(&format!("Explorer error: {}", err));
                    }
                    if key.code == KeyCode::Enter {
                        let current = app.config_explorer.current();
                        if !current.is_dir() {
                            let path = current.path().to_path_buf();
                            match app.reload_config(Some(path.clone())) {
                                Ok(_) => app.push_log(&format!(
                                    "Loaded config {}",
                                    path.display()
                                )),
                                Err(err) => app.push_log(&format!(
                                    "Failed to load config {}: {}",
                                    path.display(),
                                    err
                                )),
                            }
                            app.toggle_config_explorer();
                        }
                    }
                    if key.code == KeyCode::Esc {
                        app.toggle_config_explorer();
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    KeyCode::Char('l') => app.push_log("Manual refresh triggered."),
                    KeyCode::Char('[') => app.adjust_poll_rate(-1),
                    KeyCode::Char(']') => app.adjust_poll_rate(1),
                    KeyCode::Char('s') => app.cycle_solver(),
                    KeyCode::Char('v') => app.toggle_verbose(),
                    KeyCode::Char('r') => app.start_live_run(),
                    KeyCode::Char('p') => app.cycle_preset(true),
                    KeyCode::Char('P') => app.cycle_preset(false),
                    KeyCode::Char('h') => app.toggle_help(),
                    KeyCode::Char('L') => app.reload_config(None),
                    KeyCode::Char('c') => app.open_command_editor(),
                    KeyCode::Char('e') => app.toggle_config_explorer(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.tick();
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn draw_ui(f: &mut Frame, app: &mut App) {
    app.command_editor_area = None;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(f, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(35),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);

    let workflow_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(7)])
        .split(body_chunks[0]);
    render_workflow_table(f, workflow_chunks[0], app);
    render_stage_section(f, workflow_chunks[1], app);

    let mid_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(9),
        ])
        .split(body_chunks[1]);
    render_demo_snapshot(f, mid_chunks[0], app);
    render_demo_summary(f, mid_chunks[1], app);
    render_layout_canvas(f, mid_chunks[2], &app.layout_preview);
    render_demo_chart(f, mid_chunks[3], app);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Min(6),
        ])
        .split(body_chunks[2]);
    render_control_panel(f, right_chunks[0], app);
    render_config_preview(f, right_chunks[1], app);
    render_live_run_status(f, right_chunks[2], app);
    render_key_help(f, right_chunks[3]);

    render_logs(f, chunks[2], app);

    if app.show_help {
        render_help_overlay(f, f.area(), app);
    }

    if app.command_editor_visible {
        render_command_editor_overlay(f, app);
    }
}

fn render_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new(Text::from(Line::from(vec![Span::styled(
        "GAT TUI | workflow + demo monitor",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )])))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_workflow_table(f: &mut Frame, area: Rect, app: &App) {
    let rows: Vec<Row> = app
        .workflows
        .iter()
        .enumerate()
        .map(|(idx, wf)| {
            let style = if idx == app.selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(wf.name),
                Cell::from(wf.stage),
                Cell::from(wf.status),
                Cell::from(wf.detail),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(30),
        ],
    )
    .header(Row::new(vec!["Workflow", "Stage", "Status", "Details"]))
    .block(Block::default().borders(Borders::ALL).title("Workflows"));
    f.render_widget(table, area);
}

fn render_demo_snapshot(f: &mut Frame, area: Rect, app: &App) {
    let demo_rows: Vec<Row> = app
        .demo_stats
        .records()
        .iter()
        .take(4)
        .map(|row| {
            Row::new(vec![
                Cell::from(row.n_firms.to_string()),
                Cell::from(format!("{:.1}", row.price)),
                Cell::from(format!("{:.1}", row.eens)),
            ])
        })
        .collect();

    let demo_table = Table::new(
        demo_rows,
        [
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(Row::new(vec!["Firms", "Price", "EENS"]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Demo snapshot"),
    )
    .widths(&[
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(10),
    ]);
    f.render_widget(demo_table, area);
}

fn render_demo_summary(f: &mut Frame, area: Rect, app: &App) {
    let summary = Paragraph::new(app.demo_summary())
        .block(Block::default().borders(Borders::ALL).title("Demo summary"));
    f.render_widget(summary, area);
}

fn render_layout_canvas(f: &mut Frame, area: Rect, layout: &LayoutPreview) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    let (x_bounds, y_bounds) = layout.bounds();
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Layout preview"),
        )
        .x_bounds(x_bounds)
        .y_bounds(y_bounds)
        .paint(|ctx| {
            for ((x1, y1), (x2, y2)) in layout.edge_lines() {
                ctx.draw(&CanvasLine {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: Color::White,
                });
            }
            let points: Vec<(f64, f64)> = layout.points();
            if !points.is_empty() {
                ctx.draw(&Points {
                    coords: &points,
                    color: Color::LightGreen,
                });
            }
        });
    f.render_widget(canvas, area);
}

fn render_demo_chart(f: &mut Frame, area: Rect, app: &App) {
    let (price_points, eens_points) = app.demo_stats.chart_points();
    let x_min = price_points.first().map(|(x, _)| *x).unwrap_or(0.0);
    let mut x_max = price_points.last().map(|(x, _)| *x).unwrap_or(x_min + 1.0);
    if (x_max - x_min).abs() < std::f64::EPSILON {
        x_max = x_min + 1.0;
    }

    let chart = Chart::new(vec![
        Dataset::default()
            .name("Price")
            .marker(Marker::Braille)
            .style(Style::default().fg(Color::LightGreen))
            .data(&price_points),
        Dataset::default()
            .name("EENS")
            .marker(Marker::Dot)
            .style(Style::default().fg(Color::LightMagenta))
            .data(&eens_points),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Demo trend (Price vs EENS)"),
    )
    .x_axis(Axis::default().title("N firms").bounds([x_min, x_max]))
    .y_axis(Axis::default().title("Value"));
    f.render_widget(chart, area);
}

fn render_stage_graph(f: &mut Frame, area: Rect, app: &App) {
    let graph = Paragraph::new(Text::from(app.stage_graph())).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Workflow graph"),
    );
    f.render_widget(graph, area);
}

fn render_stage_gauges(f: &mut Frame, area: Rect, app: &App) {
    let gauges = app.gauge_metrics();
    let gauge_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            gauges
                .iter()
                .map(|_| Constraint::Length(3))
                .collect::<Vec<_>>(),
        )
        .split(area);
    for ((label, value), area) in gauges.iter().zip(gauge_chunks.iter()) {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(*label))
            .gauge_style(Style::default().fg(Color::LightBlue))
            .ratio((value / 200.0).min(1.0));
        f.render_widget(gauge, *area);
    }
}

fn render_logs(f: &mut Frame, area: Rect, app: &App) {
    let log_lines: Vec<Line<'static>> = app
        .logs
        .iter()
        .map(|line| Line::from(Span::raw(line.clone())))
        .collect();
    let logs = Paragraph::new(Text::from(log_lines))
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });
    f.render_widget(logs, area);
}

fn render_stage_section(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);
    render_stage_graph(f, chunks[0], app);
    render_stage_gauges(f, chunks[1], app);
}

fn render_control_panel(f: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Preset:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::raw(app.preset_label()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Description:",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(app.preset_description()),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Poll interval:",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {}s", app.control.poll_secs)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Solver:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::raw(app.control.solver.as_str()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Verbose:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::raw(if app.control.verbose { "on" } else { "off" }),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Command:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::raw(app.control_command_summary()),
    ]));
    lines.push(Line::from(vec![Span::styled(
        "Keys: [ ] poll, s solver, v verbose, p/P presets, r run, c edit cmd, L load config, h help",
        Style::default().fg(Color::LightCyan),
    )]));
    lines.push(Line::from(Span::raw(
        "Command editor: Ctrl+S save, Esc cancel",
    )));
    let panel = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Control panel"),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(panel, area);
}

fn render_config_preview(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(6)])
        .split(area);
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(
            "Config source:",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(&app.config_snapshot.source),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Status:", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::raw(&app.config_snapshot.status),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Config explorer key:",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw("press `e` to dive into files, Enter to load selection, Esc to exit"),
    ]));
    for (key, value) in app.config_rows(3) {
        lines.push(Line::from(vec![
            Span::styled(format!("{}:", key), Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::raw(value),
        ]));
    }
    if app.config_snapshot.entries.len() > 3 {
        lines.push(Line::from(Span::styled(
            "...",
            Style::default().add_modifier(Modifier::ITALIC),
        )));
    }
    let block = Paragraph::new(Text::from(lines))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Config preview (tap L to reload)"),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(block, chunks[0]);
    f.render_widget(&app.config_explorer.widget(), chunks[1]);
}

fn render_live_run_status(f: &mut Frame, area: Rect, app: &App) {
    let lines = vec![
        Line::from(vec![
            Span::styled("Preset cmd:", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::raw(app.control_command_summary()),
        ]),
        Line::from(vec![
            Span::styled(
                "Live status:",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::raw(app.live_run_summary()),
        ]),
    ];
    let block = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Live run"))
        .wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

fn render_key_help(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::raw(
            "Keys: ↑/↓ select, l manual log, q quit, c edit cmd",
        )),
        Line::from(Span::raw(
            "Live run: r, config reload: L, help: h, config explorer: e, Enter loads",
        )),
        Line::from(Span::raw("Use presets to pre-pack nice defaults")),
    ];
    let block = Paragraph::new(Text::from(lines))
        .block(Block::default().borders(Borders::ALL).title("Key hints"))
        .wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

fn render_help_overlay(f: &mut Frame, area: Rect, app: &App) {
    let overlay = centered_rect(70, 60, area);
    f.render_widget(Clear, overlay);
    let block = Paragraph::new(Text::from(help_text(app)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Help & config tips"),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(block, overlay);
}

fn help_text(app: &App) -> Vec<Line<'_>> {
    vec![
        Line::from(Span::styled(
            "gat-tui helps you peek at workflows, tweak parameters, and launch commands in-place.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::raw(
            "Use [ ] to tweak polling, s to rotate solvers, and v to toggle verbose logs.",
        )),
        Line::from(Span::raw(
            "Press p/P to cycle presets, or adjust values manually to produce a ‘Custom’ flag.",
        )),
        Line::from(Span::raw(
            "Live runs spawn cargo commands (default: gat-cli --help) and stream output to logs.",
        )),
        Line::from(Span::raw(
            "Config preview loads ~/.config/gat-tui/config.toml (press L to reload).",
        )),
        Line::from(Span::raw(
            "Press e to open the file explorer, Enter to load a config, Esc to cancel.",
        )),
        Line::from(Span::raw("Press h to dismiss this help dialog.")),
        Line::from(Span::raw(app.preset_description())),
    ]
}

fn render_command_editor_overlay(f: &mut Frame, app: &mut App) {
    let overlay = centered_rect(80, 50, f.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Command editor (Ctrl+S save, Esc cancel)");
    f.render_widget(block, overlay);
    let inner = if overlay.width > 2 && overlay.height > 2 {
        Rect::new(
            overlay.x + 1,
            overlay.y + 1,
            overlay.width - 2,
            overlay.height - 2,
        )
    } else {
        overlay
    };
    f.render_widget(&app.command_editor, inner);
    app.command_editor_area = Some(inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

struct LayoutPreview {
    nodes: Vec<(f64, f64, String)>,
    edges: Vec<((f64, f64), (f64, f64))>,
}

impl LayoutPreview {
    fn from_network(network: Network) -> Self {
        let layout = layout_network(&network, 160);
        let mut coord_map = HashMap::new();
        let nodes = layout
            .nodes
            .iter()
            .map(|node| {
                let coords = (node.x as f64, node.y as f64);
                coord_map.insert(node.id, coords);
                (coords.0, coords.1, node.label.clone())
            })
            .collect::<Vec<_>>();
        let edges = layout
            .edges
            .iter()
            .filter_map(|edge| {
                if let (Some(from), Some(to)) = (coord_map.get(&edge.from), coord_map.get(&edge.to))
                {
                    Some((*from, *to))
                } else {
                    None
                }
            })
            .collect();
        LayoutPreview { nodes, edges }
    }

    fn points(&self) -> Vec<(f64, f64)> {
        self.nodes.iter().map(|(x, y, _)| (*x, *y)).collect()
    }

    fn edge_lines(&self) -> Vec<((f64, f64), (f64, f64))> {
        self.edges.clone()
    }

    fn bounds(&self) -> ([f64; 2], [f64; 2]) {
        let margin = 5.0;
        if self.nodes.is_empty() {
            return ([-10.0, 10.0], [-10.0, 10.0]);
        }
        let xs: Vec<f64> = self.nodes.iter().map(|(x, _, _)| *x).collect();
        let ys: Vec<f64> = self.nodes.iter().map(|(_, y, _)| *y).collect();
        let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min) - margin;
        let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + margin;
        let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min) - margin;
        let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + margin;
        ([min_x, max_x], [min_y, max_y])
    }
}

fn build_demo_network() -> Network {
    let mut network = Network::new();
    let labels = ["North", "East", "South", "West", "Center"];
    let mut nodes = Vec::new();
    for (idx, name) in labels.iter().enumerate() {
        let bus_id = BusId::new(idx);
        let node = network.graph.add_node(CoreNode::Bus(Bus {
            id: bus_id,
            name: name.to_string(),
            voltage_kv: 120.0,
        }));
        nodes.push((bus_id, node));
    }
    let mut branch_id = 0usize;
    let connections = &[(0, 1), (1, 2), (2, 3), (3, 0), (0, 4), (2, 4)];
    for &(from_idx, to_idx) in connections {
        let (from_bus, from_node) = nodes[from_idx];
        let (to_bus, to_node) = nodes[to_idx];
        network.graph.add_edge(
            from_node,
            to_node,
            CoreEdge::Branch(Branch {
                id: BranchId::new(branch_id),
                name: format!("{}-{}", labels[from_idx], labels[to_idx]),
                from_bus,
                to_bus,
                resistance: 0.02,
                reactance: 0.1,
            }),
        );
        branch_id += 1;
    }
    network
}
