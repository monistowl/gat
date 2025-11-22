/// Reusable table component wrapper
///
/// Wraps tui-realm-stdlib Table for displaying tabular data
/// with selection, scrolling, and keyboard navigation

/// Table column definition
#[derive(Clone, Debug)]
pub struct Column {
    pub header: String,
    pub width: usize,
}

/// Table data row
#[derive(Clone, Debug)]
pub struct TableRow {
    pub cells: Vec<String>,
}

/// Wrapper around tuirealm Table component
#[derive(Clone, Debug)]
pub struct TableWidget {
    pub columns: Vec<Column>,
    pub rows: Vec<TableRow>,
    pub selected_row: usize,
    pub scroll_offset: usize,
    pub id: String,
}

impl TableWidget {
    pub fn new(id: impl Into<String>) -> Self {
        TableWidget {
            columns: Vec::new(),
            rows: Vec::new(),
            selected_row: 0,
            scroll_offset: 0,
            id: id.into(),
        }
    }

    pub fn with_columns(mut self, columns: Vec<Column>) -> Self {
        self.columns = columns;
        self
    }

    pub fn with_rows(mut self, rows: Vec<TableRow>) -> Self {
        self.rows = rows;
        self
    }

    pub fn set_selected(&mut self, row: usize) {
        if row < self.rows.len() {
            self.selected_row = row;
        }
    }

    pub fn select_next(&mut self) {
        if self.selected_row < self.rows.len().saturating_sub(1) {
            self.selected_row += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
        }
    }

    pub fn scroll_down(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(page_size);
        if self.scroll_offset > self.rows.len() {
            self.scroll_offset = self.rows.len().saturating_sub(page_size.min(self.rows.len()));
        }
    }

    pub fn scroll_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    pub fn selected_row_data(&self) -> Option<&TableRow> {
        self.rows.get(self.selected_row)
    }

    pub fn total_rows(&self) -> usize {
        self.rows.len()
    }
}
