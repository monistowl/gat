use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode};
use gat_core::{Branch, BranchId, Bus, BusId, Edge as CoreEdge, Network, Node as CoreNode};
use gat_viz::layout::layout_network;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Span, Spans};
use ratatui::widgets::canvas::{Canvas, Line, Points};
use ratatui::widgets::{
    Axis, Block, Borders, Cell, Chart, Dataset, Gauge, Paragraph, Row, Table, Wrap,
};
use ratatui::{backend::Backend, Frame, Terminal};
use std::collections::HashMap;
use std::collections::VecDeque;
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

pub struct App {
    workflows: Vec<Workflow>,
    selected: usize,
    logs: VecDeque<String>,
    demo_stats: DemoStats,
    layout_preview: LayoutPreview,
}

pub trait EventSource {
    fn poll(&mut self, timeout: Duration) -> crossterm::Result<bool>;
    fn read(&mut self) -> crossterm::Result<Event>;
}

pub struct CrosstermEventSource;

impl EventSource for CrosstermEventSource {
    fn poll(&mut self, timeout: Duration) -> crossterm::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> crossterm::Result<Event> {
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
        Self {
            workflows,
            selected: 0,
            logs,
            demo_stats: DemoStats::load_default(),
            layout_preview,
        }
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

    fn stage_graph(&self) -> Vec<Spans<'_>> {
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
            spans.push(Spans::from(vec![Span::styled(stage, style)]));
        }
        spans
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
    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw_ui(f, app))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event_source.poll(timeout)? {
            if let Event::Key(key) = event_source.read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => app.next(),
                    KeyCode::Up => app.previous(),
                    KeyCode::Char('l') => app.push_log("Manual refresh triggered."),
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

fn draw_ui<B: ratatui::backend::Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());

    render_header(f, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);
    render_workflow_table(f, body_chunks[0], app);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(9),
            Constraint::Min(6),
        ])
        .split(body_chunks[1]);
    render_demo_snapshot(f, right_chunks[0], app);
    render_demo_summary(f, right_chunks[1], app);
    render_layout_canvas(f, right_chunks[2], &app.layout_preview);
    render_demo_chart(f, right_chunks[3], app);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(right_chunks[4]);
    render_stage_graph(f, bottom_chunks[0], app);
    render_stage_gauges(f, bottom_chunks[1], app);

    render_logs(f, chunks[2], app);
}

fn render_header<B: Backend>(f: &mut Frame<B>, area: Rect) {
    let header = Paragraph::new(Spans::from(vec![Span::styled(
        "GAT TUI | workflow + demo monitor",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_workflow_table<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
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

    let table = Table::new(rows)
        .header(Row::new(vec!["Workflow", "Stage", "Status", "Details"]))
        .block(Block::default().borders(Borders::ALL).title("Workflows"))
        .widths(&[
            Constraint::Length(20),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(30),
        ]);
    f.render_widget(table, area);
}

fn render_demo_snapshot<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
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

    let demo_table = Table::new(demo_rows)
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

fn render_demo_summary<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let summary = Paragraph::new(app.demo_summary())
        .block(Block::default().borders(Borders::ALL).title("Demo summary"));
    f.render_widget(summary, area);
}

fn render_layout_canvas<B: Backend>(f: &mut Frame<B>, area: Rect, layout: &LayoutPreview) {
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
                ctx.draw(&Line {
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

fn render_demo_chart<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
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

fn render_stage_graph<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let graph = Paragraph::new(app.stage_graph()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Workflow graph"),
    );
    f.render_widget(graph, area);
}

fn render_stage_gauges<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
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

fn render_logs<B: Backend>(f: &mut Frame<B>, area: Rect, app: &App) {
    let log_text: Vec<Spans> = app
        .logs
        .iter()
        .map(|line| Spans::from(line.clone()))
        .collect();
    let logs = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });
    f.render_widget(logs, area);
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
