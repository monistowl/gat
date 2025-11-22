use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gat_tui::{Application, events::{AppEvent, KeyEvent}, ui::{PanelRegistry, PaneContext, CommandModal}};
use gat_tui::panes::dashboard::DashboardPane;
use gat_tui::panes::operations::OperationsPane;
use gat_tui::panes::datasets::DatasetsPane;
use gat_tui::panes::pipeline::PipelinePane;
use gat_tui::panes::commands::CommandsPane;
use std::io;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal (gracefully continue if no TTY)
    let _raw_mode = enable_raw_mode().ok();
    let mut stdout = io::stdout();
    let _alternate_screen = execute!(stdout, EnterAlternateScreen, EnableMouseCapture).ok();

    let mut app = Application::new();

    // Build the UI shell with all panes
    let context = PaneContext::new().with_modal(CommandModal::new("Command", "Enter command", 'x'));
    let shell = PanelRegistry::new(context)
        .register(DashboardPane)
        .register(OperationsPane)
        .register(DatasetsPane)
        .register(PipelinePane)
        .register(CommandsPane)
        .into_shell("GAT TUI");

    // Main event loop
    loop {
        // Clear screen and render UI
        execute!(
            stdout,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
            crossterm::cursor::MoveTo(0, 0)
        )?;

        let rendered = shell.render();
        stdout.write_all(rendered.as_bytes())?;
        stdout.flush()?;

        // Handle input events with timeout
        match event::poll(std::time::Duration::from_millis(250)) {
            Ok(true) => {
                match event::read() {
                    Ok(Event::Key(key)) => {
                        match key.code {
                            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.handle_event(AppEvent::Quit);
                            }
                            KeyCode::Char('q') => {
                                app.handle_key_input('q');
                            }
                            KeyCode::Char(c) => {
                                app.handle_key_input(c);
                            }
                            KeyCode::Esc => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Escape));
                            }
                            KeyCode::Enter => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Enter));
                            }
                            KeyCode::Tab => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Tab));
                            }
                            KeyCode::BackTab => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::ShiftTab));
                            }
                            KeyCode::Up => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Up));
                            }
                            KeyCode::Down => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Down));
                            }
                            KeyCode::Left => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Left));
                            }
                            KeyCode::Right => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Right));
                            }
                            KeyCode::PageUp => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::PageUp));
                            }
                            KeyCode::PageDown => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::PageDown));
                            }
                            KeyCode::Home => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Home));
                            }
                            KeyCode::End => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::End));
                            }
                            KeyCode::Backspace => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Backspace));
                            }
                            KeyCode::Delete => {
                                app.handle_event(AppEvent::KeyPress(KeyEvent::Delete));
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if app.should_quit() {
            break;
        }
    }

    // Cleanup terminal
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture
    );

    Ok(())
}
