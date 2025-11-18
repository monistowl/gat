use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Spans};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::VecDeque;
use std::io;
use std::time::{Duration, Instant};

/// Represents one of the key GAT workflow stages that the TUI highlights.
struct Workflow {
    name: &'static str,
    stage: &'static str,
    status: &'static str,
    detail: &'static str,
}

struct App {
    workflows: Vec<Workflow>,
    selected: usize,
    logs: VecDeque<String>,
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
        logs.push_back("gat-tui ready. Hit ←↑/↓→ to navigate, q to quit.".into());
        Self {
            workflows,
            selected: 0,
            logs,
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
            timestamp,
            workflow.name,
            workflow.stage
        ));
    }

    fn push_log(&mut self, entry: &str) {
        if self.logs.len() == 5 {
            self.logs.pop_front();
        }
        self.logs.push_back(entry.to_string());
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn draw_ui<B: ratatui::backend::Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(f.size());

    let header = Paragraph::new(Spans::from(vec![Span::styled(
        "GAT TUI | workflow monitor",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
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

    let log_text: Vec<Spans> = app
        .logs
        .iter()
        .map(|line| Spans::from(line.clone()))
        .collect();
    let logs = Paragraph::new(log_text)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: true });
    f.render_widget(logs, body_chunks[1]);

    let footer = Paragraph::new(vec![Spans::from(vec![Span::raw(
        "q: quit | ↑↓: select | l: log",
    )])])
    .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(footer, chunks[2]);
}
