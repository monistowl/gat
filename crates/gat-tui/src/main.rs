use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use csv::ReaderBuilder;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Span, Spans};
use ratatui::widgets::{Axis, Block, Borders, Cell, Chart, Dataset, Paragraph, Row, Table, Wrap};
use ratatui::{Frame, Terminal};
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

/// Represents one of the key GAT workflow stages shown by the UI.
struct Workflow {
    name: &'static str,
    stage: &'static str,
    status: &'static str,
    detail: &'static str,
}

#[derive(Debug, Deserialize)]
struct DemoRecord {
    #[serde(rename = "N_firms")]
    n_firms: usize,
    #[serde(rename = "Price_MWh")]
    price: f64,
    #[serde(rename = "EENS_MWh")]
    eens: f64,
}

struct App {
    workflows: Vec<Workflow>,
    selected: usize,
    logs: VecDeque<String>,
    demo_records: Vec<DemoRecord>,
}

impl App {
    fn new() -> Self {
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
        Self {
            workflows,
            selected: 0,
            logs,
            demo_records: load_demo_records(),
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
        if !self.demo_records.is_empty() {
            let avg_price = self.demo_records.iter().map(|row| row.price).sum::<f64>()
                / self.demo_records.len() as f64;
            self.push_log(&format!("demo avg price {avg_price:.1} $/MWh"));
        }
    }

    fn push_log(&mut self, entry: &str) {
        if self.logs.len() == 5 {
            self.logs.pop_front();
        }
        self.logs.push_back(entry.to_string());
    }

    fn demo_summary(&self) -> String {
        if self.demo_records.is_empty() {
            return "demo data unavailable".into();
        }
        let avg_price = self
            .demo_records
            .iter()
            .map(|row| row.price)
            .sum::<f64>()
            / self.demo_records.len() as f64;
        let avg_eens = self
            .demo_records
            .iter()
            .map(|row| row.eens)
            .sum::<f64>()
            / self.demo_records.len() as f64;
        format!("Avg price {:.1} $/MWh | Avg EENS {:.1} MWh", avg_price, avg_eens)
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let tick_rate = Duration::from_secs(1);
    let mut last_tick = Instant::now();
    let mut app = App::new();

    'outer: loop {
        terminal.draw(|f| draw_ui(f, &app))?;
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break 'outer,
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

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
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

    let header = Paragraph::new(Spans::from(vec![Span::styled(
        "GAT TUI | workflow + demo monitor",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

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
    f.render_widget(table, body_chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Min(6),
        ])
        .split(body_chunks[1]);

    let demo_rows: Vec<Row> = app
        .demo_records
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
    f.render_widget(demo_table, right_chunks[0]);

    let summary = Paragraph::new(app.demo_summary())
        .block(Block::default().borders(Borders::ALL).title("Demo summary"));
    f.render_widget(summary, right_chunks[1]);

    let price_points: Vec<(f64, f64)> = app
        .demo_records
        .iter()
        .map(|row| (row.n_firms as f64, row.price))
        .collect();
    let eens_points: Vec<(f64, f64)> = app
        .demo_records
        .iter()
        .map(|row| (row.n_firms as f64, row.eens))
        .collect();

    let x_min = price_points.first().map(|(x, _)| *x).unwrap_or(0.0);
    let mut x_max = price_points.last().map(|(x, _)| *x).unwrap_or(10.0);
    if x_max <= x_min {
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
    f.render_widget(chart, right_chunks[1]);

    let log_text: Vec<Spans> = app
        .logs
        .iter()
        .map(|line| Spans::from(line.clone()))
        .collect();
    let logs = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });
    f.render_widget(logs, chunks[2]);
}

fn load_demo_records() -> Vec<DemoRecord> {
    let path = Path::new("out/demos/cournot/cournot_results.csv");
    if !path.exists() {
        return fallback_demo_records();
    }
    if let Ok(file) = fs::File::open(path) {
        let mut reader = ReaderBuilder::new().has_headers(true).from_reader(file);
        let mut out = Vec::new();
        for record in reader.deserialize().flatten() {
            out.push(record);
        }
        if !out.is_empty() {
            return out;
        }
    }
    fallback_demo_records()
}

fn fallback_demo_records() -> Vec<DemoRecord> {
    vec![
        DemoRecord {
            n_firms: 1,
            price: 180.0,
            eens: 15.1,
        },
        DemoRecord {
            n_firms: 4,
            price: 150.5,
            eens: 8.2,
        },
        DemoRecord {
            n_firms: 7,
            price: 130.2,
            eens: 4.6,
        },
        DemoRecord {
            n_firms: 10,
            price: 118.7,
            eens: 2.1,
        },
    ]
}
