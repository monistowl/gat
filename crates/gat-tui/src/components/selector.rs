/// Selection component for radio/checkbox-like widgets

#[derive(Clone, Debug)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct SelectWidget {
    pub options: Vec<SelectOption>,
    pub selected_index: usize,
    pub id: String,
}

impl SelectWidget {
    pub fn new(id: impl Into<String>) -> Self {
        SelectWidget {
            options: Vec::new(),
            selected_index: 0,
            id: id.into(),
        }
    }

    pub fn add_option(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.options.push(SelectOption {
            label: label.into(),
            value: value.into(),
        });
    }

    pub fn with_options(mut self, options: Vec<(String, String)>) -> Self {
        self.options = options
            .into_iter()
            .map(|(label, value)| SelectOption { label, value })
            .collect();
        self
    }

    pub fn select_index(&mut self, index: usize) {
        if index < self.options.len() {
            self.selected_index = index;
        }
    }

    pub fn next(&mut self) {
        if self.selected_index < self.options.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn selected(&self) -> Option<&SelectOption> {
        self.options.get(self.selected_index)
    }

    pub fn selected_value(&self) -> Option<String> {
        self.selected().map(|opt| opt.value.clone())
    }

    pub fn option_count(&self) -> usize {
        self.options.len()
    }
}
