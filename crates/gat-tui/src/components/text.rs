/// Reusable text and paragraph component wrapper

#[derive(Clone, Debug)]
pub struct TextWidget {
    pub content: String,
    pub id: String,
}

impl TextWidget {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        TextWidget {
            content: content.into(),
            id: id.into(),
        }
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    pub fn append(&mut self, text: impl Into<String>) {
        self.content.push_str(&text.into());
    }

    pub fn clear(&mut self) {
        self.content.clear();
    }
}

#[derive(Clone, Debug)]
pub struct ParagraphWidget {
    pub lines: Vec<String>,
    pub id: String,
    pub scroll_offset: usize,
}

impl ParagraphWidget {
    pub fn new(id: impl Into<String>) -> Self {
        ParagraphWidget {
            lines: Vec::new(),
            id: id.into(),
            scroll_offset: 0,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.lines = text
            .into()
            .lines()
            .map(|s| s.to_string())
            .collect();
        self
    }

    pub fn set_content(&mut self, content: impl Into<String>) {
        let text = content.into();
        self.lines = text
            .lines()
            .map(|s| s.to_string())
            .collect();
        self.scroll_offset = 0;
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(
            self.lines.len().saturating_sub(1)
        );
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }
}
