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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TerminalSize {
    width: u16,
    height: u16,
}

impl TerminalSize {
    fn current() -> Result<Self> {
        let (width, height) = crossterm::terminal::size()?;
        Ok(Self { width, height })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal (gracefully continue if no TTY)
    let _raw_mode = enable_raw_mode().ok();
    let mut stdout = io::stdout();
    let _alternate_screen = execute!(stdout, EnterAlternateScreen, EnableMouseCapture).ok();

    let mut app = Application::new();

    // Track previous active pane to detect changes
    let mut last_pane = app.state().active_pane;

    let render_shell = || {
        let context = PaneContext::new().with_modal(CommandModal::new("Command", "Enter command", 'x'));
        PanelRegistry::new(context)
            .register(DashboardPane)
            .register(OperationsPane)
            .register(DatasetsPane)
            .register(PipelinePane)
            .register(CommandsPane)
            .into_shell("GAT TUI")
    };

    let mut shell = render_shell();
    let mut last_terminal_size = TerminalSize::current()?;

    // Render initial screen once
    execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    )?;
    let rendered = shell.render();
    stdout.write_all(rendered.as_bytes())?;
    stdout.flush()?;

    // Main event loop - render on state changes or resize
    loop {
        // Check for terminal resize
        let current_size = TerminalSize::current()?;
        let size_changed = current_size != last_terminal_size;

        // Handle input events with timeout
        let mut should_render = size_changed;
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
                    Ok(Event::Resize(width, height)) => {
                        // Crossterm will give us resize events; update our tracking
                        last_terminal_size = TerminalSize { width, height };
                        should_render = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Re-render if state changed (e.g., pane switched) or terminal resized
        if should_render || app.state().active_pane != last_pane {
            last_pane = app.state().active_pane;
            last_terminal_size = current_size;

            // Recreate shell with new viewport dimensions
            shell = render_shell();
            shell = shell.with_viewport(current_size.width, current_size.height);
            shell.select_menu_item(last_pane.hotkey());

            execute!(
                stdout,
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                crossterm::cursor::MoveTo(0, 0)
            )?;
            let rendered = shell.render();
            stdout.write_all(rendered.as_bytes())?;
            stdout.flush()?;
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
