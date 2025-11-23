/// A simple label component for displaying static text
pub struct Label {
    pub text: String,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Label { text: text.into() }
    }
}
