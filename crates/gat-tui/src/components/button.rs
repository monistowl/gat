/// A clickable button component
pub struct Button {
    pub label: String,
    pub hotkey: Option<char>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Button {
            label: label.into(),
            hotkey: None,
        }
    }

    pub fn with_hotkey(mut self, hotkey: char) -> Self {
        self.hotkey = Some(hotkey);
        self
    }
}
