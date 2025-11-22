/// Reusable list component wrapper
///
/// Scrollable selectable list for displaying items

#[derive(Clone, Debug)]
pub struct ListItem {
    pub label: String,
    pub data: String,
}

/// Wrapper around list component
#[derive(Clone, Debug)]
pub struct ListWidget {
    pub items: Vec<ListItem>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub id: String,
}

impl ListWidget {
    pub fn new(id: impl Into<String>) -> Self {
        ListWidget {
            items: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            id: id.into(),
        }
    }

    pub fn with_items(mut self, items: Vec<ListItem>) -> Self {
        self.items = items;
        self
    }

    pub fn add_item(&mut self, label: String, data: String) {
        self.items.push(ListItem { label, data });
    }

    pub fn select_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = index;
        }
    }

    pub fn select_next(&mut self) {
        if self.selected_index < self.items.len().saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn selected_item(&self) -> Option<&ListItem> {
        self.items.get(self.selected_index)
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
