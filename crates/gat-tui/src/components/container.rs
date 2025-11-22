/// A basic container component with borders
pub struct Container {
    pub title: Option<String>,
    pub content: String,
}

impl Container {
    pub fn new(content: impl Into<String>) -> Self {
        Container {
            title: None,
            content: content.into(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}
