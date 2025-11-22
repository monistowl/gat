use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initial draw
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Length(5),
                    Constraint::Min(0),
                ]
                .as_ref(),
            )
            .split(f.size());

        let title = Paragraph::new("Hello World from tuirealm!")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Tuirealm Test"));
        f.render_widget(title, chunks[0]);

        let instructions = Paragraph::new(
            "Press ESC to exit\n\nIf you see this with proper colors,\nlines, and alignment, tuirealm/ratatui\nis working correctly.",
        )
        .style(Style::default().fg(Color::Green))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Instructions"),
        );
        f.render_widget(instructions, chunks[1]);

        let status = Paragraph::new("✓ Rendering system is functional")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM))
            .alignment(Alignment::Center);
        f.render_widget(status, chunks[2]);
    })?;

    // Event loop
    loop {
        if let Ok(true) = event::poll(std::time::Duration::from_millis(100)) {
            if let Ok(Event::Key(key)) = event::read() {
                if matches!(key.code, KeyCode::Esc) {
                    break;
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
