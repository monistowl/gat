use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use gat_tui::{run_tui, App, EventSource};
use ratatui::backend::TestBackend;
use ratatui::terminal::Terminal;
use std::collections::VecDeque;
use std::time::Duration;

struct MockInput {
    events: VecDeque<Event>,
}

impl MockInput {
    fn new(events: Vec<Event>) -> Self {
        Self {
            events: events.into_iter().collect(),
        }
    }
}

impl EventSource for MockInput {
    fn poll(&mut self, _timeout: Duration) -> crossterm::Result<bool> {
        Ok(!self.events.is_empty())
    }

    fn read(&mut self) -> crossterm::Result<Event> {
        Ok(self.events.pop_front().unwrap())
    }
}

#[test]
fn run_tui_quits_on_q() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut input = MockInput::new(vec![Event::Key(KeyEvent::new(
        KeyCode::Char('q'),
        KeyModifiers::NONE,
    ))]);
    let mut app = App::new();
    run_tui(&mut terminal, &mut input, &mut app).unwrap();
}
