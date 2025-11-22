/// Grid management UI components (Phase 3)
///
/// Provides reusable components for:
/// - Grid browser (list of loaded grids)
/// - Grid load modal (file path input)
/// - Grid info display (stats for current grid)

use crate::data::DatasetEntry;
use crate::DatasetStatus;

/// Display information about a single loaded grid
#[derive(Clone, Debug)]
pub struct GridInfo {
    pub id: String,
    pub node_count: usize,
    pub branch_count: usize,
    pub density: f64,
    pub status: GridStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridStatus {
    Active,
    Inactive,
    Loading,
    Error,
}

impl GridStatus {
    pub fn display(&self) -> &'static str {
        match self {
            GridStatus::Active => "●",
            GridStatus::Inactive => "○",
            GridStatus::Loading => "◐",
            GridStatus::Error => "✗",
        }
    }
}

impl GridInfo {
    /// Create GridInfo from a DatasetEntry
    pub fn from_dataset(entry: &DatasetEntry, is_active: bool) -> Self {
        Self {
            id: entry.id.clone(),
            node_count: entry.row_count,
            branch_count: (entry.row_count as f64 * 1.5) as usize, // Rough estimate
            density: entry.row_count as f64 / 100.0, // Normalized
            status: match is_active {
                true => GridStatus::Active,
                false => GridStatus::Inactive,
            },
        }
    }

    /// Format grid info for display in a table row
    pub fn format_row(&self) -> [String; 5] {
        [
            self.status.display().to_string(),
            self.id.clone(),
            self.node_count.to_string(),
            self.branch_count.to_string(),
            format!("{:.3}", self.density),
        ]
    }
}

/// State for the grid browser modal
#[derive(Clone, Debug)]
pub struct GridBrowserState {
    pub grids: Vec<GridInfo>,
    pub selected_index: usize,
    pub search_query: String,
}

impl GridBrowserState {
    pub fn new(grids: Vec<GridInfo>) -> Self {
        Self {
            grids,
            selected_index: 0,
            search_query: String::new(),
        }
    }

    /// Filter grids by search query
    pub fn filtered_grids(&self) -> Vec<&GridInfo> {
        if self.search_query.is_empty() {
            self.grids.iter().collect()
        } else {
            let query = self.search_query.to_lowercase();
            self.grids
                .iter()
                .filter(|g| g.id.to_lowercase().contains(&query))
                .collect()
        }
    }

    /// Get the currently selected grid
    pub fn selected_grid(&self) -> Option<&GridInfo> {
        let filtered = self.filtered_grids();
        filtered.get(self.selected_index).copied()
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let max = self.filtered_grids().len();
        if self.selected_index + 1 < max {
            self.selected_index += 1;
        }
    }

    /// Add character to search query
    pub fn add_char(&mut self, c: char) {
        self.search_query.push(c);
        self.selected_index = 0; // Reset selection on new search
    }

    /// Remove last character from search query
    pub fn backspace(&mut self) {
        self.search_query.pop();
        self.selected_index = 0;
    }

    /// Clear search query
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.selected_index = 0;
    }
}

/// State for the grid load modal
#[derive(Clone, Debug)]
pub struct GridLoadState {
    pub file_path: String,
    pub cursor_position: usize,
    pub loading: bool,
    pub error_message: Option<String>,
}

impl GridLoadState {
    pub fn new() -> Self {
        Self {
            file_path: String::new(),
            cursor_position: 0,
            loading: false,
            error_message: None,
        }
    }

    /// Add character to file path
    pub fn add_char(&mut self, c: char) {
        self.file_path.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.error_message = None;
    }

    /// Remove character before cursor
    pub fn backspace(&mut self) {
        if self.cursor_position > 0 {
            self.file_path.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
            self.error_message = None;
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor_position < self.file_path.len() {
            self.cursor_position += 1;
        }
    }

    /// Check if path is valid (basic validation)
    pub fn is_valid(&self) -> bool {
        !self.file_path.is_empty() && (self.file_path.ends_with(".arrow") || self.file_path.ends_with(".m"))
    }

    /// Get the file path
    pub fn get_path(&self) -> String {
        self.file_path.clone()
    }

    /// Clear all state
    pub fn reset(&mut self) {
        self.file_path.clear();
        self.cursor_position = 0;
        self.loading = false;
        self.error_message = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_status_display() {
        assert_eq!(GridStatus::Active.display(), "●");
        assert_eq!(GridStatus::Inactive.display(), "○");
        assert_eq!(GridStatus::Loading.display(), "◐");
        assert_eq!(GridStatus::Error.display(), "✗");
    }

    #[test]
    fn test_grid_info_format_row() {
        let grid = GridInfo {
            id: "ieee14".to_string(),
            node_count: 14,
            branch_count: 20,
            density: 0.14,
            status: GridStatus::Active,
        };

        let row = grid.format_row();
        assert_eq!(row[0], "●");
        assert_eq!(row[1], "ieee14");
        assert_eq!(row[2], "14");
        assert_eq!(row[3], "20");
        assert!(row[4].starts_with("0.14"));
    }

    #[test]
    fn test_grid_browser_state_creation() {
        let grids = vec![
            GridInfo {
                id: "grid1".to_string(),
                node_count: 10,
                branch_count: 15,
                density: 0.1,
                status: GridStatus::Active,
            },
            GridInfo {
                id: "grid2".to_string(),
                node_count: 20,
                branch_count: 30,
                density: 0.2,
                status: GridStatus::Inactive,
            },
        ];

        let state = GridBrowserState::new(grids);
        assert_eq!(state.grids.len(), 2);
        assert_eq!(state.selected_index, 0);
        assert!(state.search_query.is_empty());
    }

    #[test]
    fn test_grid_browser_selection() {
        let grids = vec![
            GridInfo {
                id: "grid1".to_string(),
                node_count: 10,
                branch_count: 15,
                density: 0.1,
                status: GridStatus::Active,
            },
            GridInfo {
                id: "grid2".to_string(),
                node_count: 20,
                branch_count: 30,
                density: 0.2,
                status: GridStatus::Inactive,
            },
        ];

        let mut state = GridBrowserState::new(grids);
        assert_eq!(state.selected_grid().unwrap().id, "grid1");

        state.select_next();
        assert_eq!(state.selected_grid().unwrap().id, "grid2");

        state.select_previous();
        assert_eq!(state.selected_grid().unwrap().id, "grid1");
    }

    #[test]
    fn test_grid_browser_search() {
        let grids = vec![
            GridInfo {
                id: "ieee14".to_string(),
                node_count: 14,
                branch_count: 20,
                density: 0.14,
                status: GridStatus::Active,
            },
            GridInfo {
                id: "ieee33".to_string(),
                node_count: 33,
                branch_count: 50,
                density: 0.33,
                status: GridStatus::Inactive,
            },
        ];

        let mut state = GridBrowserState::new(grids);
        assert_eq!(state.filtered_grids().len(), 2);

        state.add_char('1');
        state.add_char('4');
        let filtered = state.filtered_grids();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "ieee14");
    }

    #[test]
    fn test_grid_load_state_path_input() {
        let mut state = GridLoadState::new();
        assert!(state.file_path.is_empty());

        state.add_char('/');
        state.add_char('t');
        state.add_char('e');
        state.add_char('s');
        state.add_char('t');
        state.add_char('.');
        state.add_char('a');
        state.add_char('r');
        state.add_char('r');
        state.add_char('o');
        state.add_char('w');

        assert_eq!(state.file_path, "/test.arrow");
        assert!(state.is_valid());
    }

    #[test]
    fn test_grid_load_state_backspace() {
        let mut state = GridLoadState::new();
        state.file_path = "test.arrow".to_string();
        state.cursor_position = state.file_path.len();

        for _ in 0..6 {
            state.backspace();
        }

        assert_eq!(state.file_path, "test");
        assert!(!state.is_valid()); // .arrow removed
    }

    #[test]
    fn test_grid_load_state_cursor() {
        let mut state = GridLoadState::new();
        state.file_path = "test.arrow".to_string();

        state.cursor_position = 5;
        state.cursor_left();
        assert_eq!(state.cursor_position, 4);

        state.cursor_right();
        assert_eq!(state.cursor_position, 5);
    }

    #[test]
    fn test_grid_load_state_validation() {
        let mut state = GridLoadState::new();

        state.file_path = "test.txt".to_string();
        assert!(!state.is_valid());

        state.file_path = "test.arrow".to_string();
        assert!(state.is_valid());

        state.file_path = "test.m".to_string();
        assert!(state.is_valid());
    }
}
