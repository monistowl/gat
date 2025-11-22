/// Reusable text input component wrapper

#[derive(Clone, Debug)]
pub struct InputWidget {
    pub value: String,
    pub placeholder: String,
    pub cursor_pos: usize,
    pub id: String,
    pub is_focused: bool,
    pub max_length: Option<usize>,
}

impl InputWidget {
    pub fn new(id: impl Into<String>) -> Self {
        InputWidget {
            value: String::new(),
            placeholder: String::new(),
            cursor_pos: 0,
            id: id.into(),
            is_focused: false,
            max_length: None,
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    pub fn set_value(&mut self, value: String) {
        if let Some(max) = self.max_length {
            self.value = value.chars().take(max).collect();
        } else {
            self.value = value;
        }
        self.cursor_pos = self.value.len();
    }

    pub fn push_char(&mut self, ch: char) {
        if let Some(max) = self.max_length {
            if self.value.len() < max {
                self.value.insert(self.cursor_pos, ch);
                self.cursor_pos += 1;
            }
        } else {
            self.value.insert(self.cursor_pos, ch);
            self.cursor_pos += 1;
        }
    }

    pub fn pop_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.value.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.value.len() {
            self.cursor_pos += 1;
        }
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_pos = 0;
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }
}

impl Default for InputWidget {
    fn default() -> Self {
        Self::new("input")
    }
}
