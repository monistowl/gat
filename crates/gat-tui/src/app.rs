use crate::models::AppState;
use crate::events::{AppEvent, KeyEvent};

pub struct Application {
    state: AppState,
    should_quit: bool,
}

impl Application {
    pub fn new() -> Self {
        Application {
            state: AppState::new(),
            should_quit: false,
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn handle_event(&mut self, event: AppEvent) {
        if matches!(event, AppEvent::Quit) {
            self.should_quit = true;
            return;
        }

        self.state = crate::events::reduce(self.state.clone(), event);
    }

    pub fn handle_key_input(&mut self, c: char) {
        let key_event = match c {
            'q' => KeyEvent::Hotkey('q'),
            '1'..='5' => KeyEvent::Hotkey(c),
            'h' => KeyEvent::Hotkey('h'),
            '\u{1b}' => KeyEvent::Escape,
            '\r' => KeyEvent::Enter,
            '\t' => KeyEvent::Tab,
            _ => KeyEvent::Hotkey(c),
        };

        match key_event {
            KeyEvent::Hotkey('q') => {
                self.should_quit = true;
            }
            _ => {
                self.handle_event(AppEvent::KeyPress(key_event));
            }
        }
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
