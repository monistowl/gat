use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gat_tui::{Application, events::{AppEvent, KeyEvent}};
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    eprintln!("GAT TUI - Starting application");
    let mut app = Application::new();
    eprintln!("Active pane: {}", app.state().active_pane.label());

    // Try to setup terminal, but continue if it fails (e.g., no TTY)
    let _terminal_setup = enable_raw_mode().ok();
    let mut stdout = io::stdout();
    let _alternate_screen = execute!(stdout, EnterAlternateScreen, EnableMouseCapture).ok();

    // Main event loop
    loop {
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
    let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);

    eprintln!("GAT TUI - Shutting down");
    Ok(())
}
