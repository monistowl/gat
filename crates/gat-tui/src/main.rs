use anyhow::Result;
use atty::Stream;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use gat_tui::{run_tui, App, CrosstermEventSource};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

fn main() -> Result<()> {
    if !atty::is(Stream::Stdout) {
        eprintln!("gat-tui requires an interactive terminal (stdout is not a TTY).");
        eprintln!("Run `gat-gui` if you only need a rendered summary without a terminal.");
        return Ok(());
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    let mut input_source = CrosstermEventSource;
    let res = run_tui(&mut terminal, &mut input_source, &mut app);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    res
}
