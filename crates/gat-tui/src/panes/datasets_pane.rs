/// Datasets Pane - Data catalog browsing, upload, and management
///
/// The datasets pane provides:
/// - Data catalog with public datasets
/// - Dataset preview and inspection
/// - Upload and download management
/// - Metadata and retention policies

use crate::components::*;
use crate::ui::{GridBrowserState, GridInfo, GridLoadState, GridStatus};

/// Dataset entry in catalog
#[derive(Clone, Debug)]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub size_mb: f64,
    pub rows: usize,
    pub format: String,
    pub updated: String,
    pub status: DatasetStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatasetStatus {
    Available,
    Importing,
    Processing,
    Error,
}

impl DatasetStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            DatasetStatus::Available => "✓",
            DatasetStatus::Importing => "⟳",
            DatasetStatus::Processing => "◐",
            DatasetStatus::Error => "✗",
        }
    }
}

/// Upload job tracking
#[derive(Clone, Debug)]
pub struct UploadJob {
    pub id: String,
    pub filename: String,
    pub progress_percent: u32,
    pub status: UploadStatus,
    pub timestamp: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadStatus {
    Pending,
    Uploading,
    Processing,
    Complete,
    Failed,
}

impl UploadStatus {
    pub fn label(&self) -> &'static str {
        match self {
            UploadStatus::Pending => "Pending",
            UploadStatus::Uploading => "Uploading",
            UploadStatus::Processing => "Processing",
            UploadStatus::Complete => "Complete",
            UploadStatus::Failed => "Failed",
        }
    }
}

/// Datasets pane state
#[derive(Clone, Debug)]
pub struct DatasetsPaneState {
    // Catalog
    pub datasets: Vec<Dataset>,
    pub selected_dataset: usize,

    // Uploads
    pub uploads: Vec<UploadJob>,
    pub selected_upload: usize,

    // Metadata
    pub metadata: DatasetMetadata,

    // Grid management (Phase 3)
    pub grid_browser: GridBrowserState,
    pub grid_load: GridLoadState,
    pub show_grid_browser: bool,

    // Component states
    pub datasets_table: TableWidget,
    pub uploads_list: ListWidget,
    pub search_input: InputWidget,
    pub metadata_text: TextWidget,

    // UI state
    pub search_filter: String,
    pub show_uploads: bool,
}

#[derive(Clone, Debug)]
pub struct DatasetMetadata {
    pub retention_days: u32,
    pub backup_schedule: String,
    pub total_size_gb: f64,
    pub available_size_gb: f64,
}

impl Default for DatasetsPaneState {
    fn default() -> Self {
        let datasets = vec![
            Dataset {
                id: "opsd-001".into(),
                name: "OPSD Europe".into(),
                size_mb: 1250.5,
                rows: 8760000,
                format: "CSV".into(),
                updated: "2024-11-21 14:30".into(),
                status: DatasetStatus::Available,
            },
            Dataset {
                id: "airtravel-001".into(),
                name: "Airtravel Tutorial".into(),
                size_mb: 45.2,
                rows: 144000,
                format: "Parquet".into(),
                updated: "2024-11-20 10:15".into(),
                status: DatasetStatus::Available,
            },
            Dataset {
                id: "synthetic-002".into(),
                name: "Synthetic Grid Data".into(),
                size_mb: 890.0,
                rows: 5256000,
                format: "CSV".into(),
                updated: "2024-11-19 08:45".into(),
                status: DatasetStatus::Processing,
            },
        ];

        let mut datasets_table = TableWidget::new("datasets_catalog");
        datasets_table.columns = vec![
            Column { header: "Name".into(), width: 25 },
            Column { header: "Size (MB)".into(), width: 12 },
            Column { header: "Rows".into(), width: 12 },
            Column { header: "Status".into(), width: 10 },
        ];

        let mut uploads_list = ListWidget::new("datasets_uploads");

        let uploads = vec![
            UploadJob {
                id: "up_001".into(),
                filename: "custom_grid_model.csv".into(),
                progress_percent: 100,
                status: UploadStatus::Complete,
                timestamp: "2024-11-21 13:20".into(),
            },
            UploadJob {
                id: "up_002".into(),
                filename: "demand_forecast.parquet".into(),
                progress_percent: 45,
                status: UploadStatus::Uploading,
                timestamp: "2024-11-21 14:45".into(),
            },
        ];

        for upload in &uploads {
            uploads_list.add_item(
                format!("{} ({}%)", upload.filename, upload.progress_percent),
                upload.id.clone(),
            );
        }

        let metadata = DatasetMetadata {
            retention_days: 30,
            backup_schedule: "Nightly".into(),
            total_size_gb: 500.0,
            available_size_gb: 125.5,
        };

        DatasetsPaneState {
            datasets,
            selected_dataset: 0,
            uploads,
            selected_upload: 0,
            metadata,
            grid_browser: GridBrowserState::new(Vec::new()),
            grid_load: GridLoadState::new(),
            show_grid_browser: false,
            datasets_table,
            uploads_list,
            search_input: InputWidget::new("dataset_search")
                .with_placeholder("Search datasets..."),
            metadata_text: TextWidget::new("dataset_metadata", ""),
            search_filter: String::new(),
            show_uploads: false,
        }
    }
}

impl DatasetsPaneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select_next_dataset(&mut self) {
        if self.selected_dataset < self.datasets.len().saturating_sub(1) {
            self.selected_dataset += 1;
        }
    }

    pub fn select_prev_dataset(&mut self) {
        if self.selected_dataset > 0 {
            self.selected_dataset -= 1;
        }
    }

    pub fn selected_dataset(&self) -> Option<&Dataset> {
        self.datasets.get(self.selected_dataset)
    }

    pub fn filter_datasets(&mut self, query: String) {
        self.search_filter = query;
    }

    pub fn filtered_datasets(&self) -> Vec<&Dataset> {
        if self.search_filter.is_empty() {
            self.datasets.iter().collect()
        } else {
            let query = self.search_filter.to_lowercase();
            self.datasets
                .iter()
                .filter(|d| d.name.to_lowercase().contains(&query) || d.format.to_lowercase().contains(&query))
                .collect()
        }
    }

    pub fn select_next_upload(&mut self) {
        if self.selected_upload < self.uploads.len().saturating_sub(1) {
            self.selected_upload += 1;
        }
    }

    pub fn select_prev_upload(&mut self) {
        if self.selected_upload > 0 {
            self.selected_upload -= 1;
        }
    }

    pub fn selected_upload(&self) -> Option<&UploadJob> {
        self.uploads.get(self.selected_upload)
    }

    pub fn add_upload(&mut self, job: UploadJob) {
        self.uploads.insert(0, job.clone());
        self.uploads_list.add_item(
            format!("{} ({}%)", job.filename, job.progress_percent),
            job.id,
        );
    }

    pub fn update_upload_progress(&mut self, index: usize, progress: u32) {
        if let Some(upload) = self.uploads.get_mut(index) {
            upload.progress_percent = progress.min(100);
        }
    }

    pub fn toggle_view(&mut self) {
        self.show_uploads = !self.show_uploads;
    }

    pub fn dataset_count(&self) -> usize {
        self.datasets.len()
    }

    pub fn upload_count(&self) -> usize {
        self.uploads.len()
    }

    pub fn format_metadata(&mut self) {
        self.metadata_text.set_content(format!(
            "Retention: {} days\nBackup: {}\nUsed: {:.1} GB / {:.1} GB\nAvailable: {:.1} GB",
            self.metadata.retention_days,
            self.metadata.backup_schedule,
            self.metadata.total_size_gb - self.metadata.available_size_gb,
            self.metadata.total_size_gb,
            self.metadata.available_size_gb,
        ));
    }

    // Grid management methods (Phase 3)

    /// Update the grid browser with loaded grids from AppState
    pub fn update_grids(&mut self, grids: Vec<GridInfo>) {
        self.grid_browser = GridBrowserState::new(grids);
    }

    /// Get currently selected grid from grid browser
    pub fn selected_grid(&self) -> Option<&GridInfo> {
        self.grid_browser.selected_grid()
    }

    /// Navigate to next grid in browser
    pub fn select_next_grid(&mut self) {
        self.grid_browser.select_next();
    }

    /// Navigate to previous grid in browser
    pub fn select_prev_grid(&mut self) {
        self.grid_browser.select_previous();
    }

    /// Add character to grid load file path
    pub fn add_grid_path_char(&mut self, c: char) {
        self.grid_load.add_char(c);
    }

    /// Remove character from grid load file path
    pub fn backspace_grid_path(&mut self) {
        self.grid_load.backspace();
    }

    /// Move cursor left in grid load path
    pub fn grid_path_cursor_left(&mut self) {
        self.grid_load.cursor_left();
    }

    /// Move cursor right in grid load path
    pub fn grid_path_cursor_right(&mut self) {
        self.grid_load.cursor_right();
    }

    /// Get the grid load file path
    pub fn grid_load_path(&self) -> String {
        self.grid_load.get_path()
    }

    /// Check if grid load path is valid
    pub fn is_grid_path_valid(&self) -> bool {
        self.grid_load.is_valid()
    }

    /// Reset grid load state
    pub fn reset_grid_load(&mut self) {
        self.grid_load.reset();
    }

    /// Toggle grid browser visibility
    pub fn toggle_grid_browser(&mut self) {
        self.show_grid_browser = !self.show_grid_browser;
    }

    /// Add character to grid browser search
    pub fn add_grid_search_char(&mut self, c: char) {
        self.grid_browser.add_char(c);
    }

    /// Remove character from grid browser search
    pub fn backspace_grid_search(&mut self) {
        self.grid_browser.backspace();
    }

    /// Clear grid browser search
    pub fn clear_grid_search(&mut self) {
        self.grid_browser.clear_search();
    }

    /// Get filtered grids from grid browser
    pub fn filtered_grids(&self) -> Vec<&GridInfo> {
        self.grid_browser.filtered_grids()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datasets_init() {
        let state = DatasetsPaneState::new();
        assert_eq!(state.dataset_count(), 3);
        assert_eq!(state.upload_count(), 2);
    }

    #[test]
    fn test_dataset_selection() {
        let mut state = DatasetsPaneState::new();
        state.select_next_dataset();
        assert_eq!(state.selected_dataset, 1);
        state.select_prev_dataset();
        assert_eq!(state.selected_dataset, 0);
    }

    #[test]
    fn test_filter_datasets() {
        let mut state = DatasetsPaneState::new();
        state.filter_datasets("OPSD".into());
        let filtered = state.filtered_datasets();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_upload_management() {
        let mut state = DatasetsPaneState::new();
        let initial = state.upload_count();
        let job = UploadJob {
            id: "test_001".into(),
            filename: "test.csv".into(),
            progress_percent: 0,
            status: UploadStatus::Pending,
            timestamp: "2024-11-21".into(),
        };
        state.add_upload(job);
        assert_eq!(state.upload_count(), initial + 1);
    }

    #[test]
    fn test_upload_progress() {
        let mut state = DatasetsPaneState::new();
        state.update_upload_progress(0, 75);
        assert_eq!(state.uploads[0].progress_percent, 75);
    }

    #[test]
    fn test_dataset_status_symbol() {
        assert_eq!(DatasetStatus::Available.symbol(), "✓");
        assert_eq!(DatasetStatus::Processing.symbol(), "◐");
        assert_eq!(DatasetStatus::Error.symbol(), "✗");
    }

    #[test]
    fn test_upload_status_label() {
        assert_eq!(UploadStatus::Complete.label(), "Complete");
        assert_eq!(UploadStatus::Uploading.label(), "Uploading");
    }

    #[test]
    fn test_view_toggle() {
        let mut state = DatasetsPaneState::new();
        assert!(!state.show_uploads);
        state.toggle_view();
        assert!(state.show_uploads);
    }

    // Grid management tests (Phase 3)

    #[test]
    fn test_grid_browser_initialization() {
        let state = DatasetsPaneState::new();
        assert!(!state.show_grid_browser);
        assert!(state.grid_browser.filtered_grids().is_empty());
    }

    #[test]
    fn test_update_grids() {
        let mut state = DatasetsPaneState::new();
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
        state.update_grids(grids);
        assert_eq!(state.filtered_grids().len(), 2);
    }

    #[test]
    fn test_grid_selection_navigation() {
        let mut state = DatasetsPaneState::new();
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
        state.update_grids(grids);
        assert_eq!(state.selected_grid().unwrap().id, "grid1");

        state.select_next_grid();
        assert_eq!(state.selected_grid().unwrap().id, "grid2");

        state.select_prev_grid();
        assert_eq!(state.selected_grid().unwrap().id, "grid1");
    }

    #[test]
    fn test_grid_load_path_input() {
        let mut state = DatasetsPaneState::new();
        assert!(state.grid_load_path().is_empty());

        state.add_grid_path_char('/');
        state.add_grid_path_char('t');
        state.add_grid_path_char('e');
        state.add_grid_path_char('s');
        state.add_grid_path_char('t');
        state.add_grid_path_char('.');
        state.add_grid_path_char('a');
        state.add_grid_path_char('r');
        state.add_grid_path_char('r');
        state.add_grid_path_char('o');
        state.add_grid_path_char('w');

        assert_eq!(state.grid_load_path(), "/test.arrow");
        assert!(state.is_grid_path_valid());
    }

    #[test]
    fn test_grid_load_backspace() {
        let mut state = DatasetsPaneState::new();
        state.grid_load.file_path = "test.arrow".to_string();
        state.grid_load.cursor_position = state.grid_load.file_path.len();

        for _ in 0..6 {
            state.backspace_grid_path();
        }

        assert_eq!(state.grid_load_path(), "test");
        assert!(!state.is_grid_path_valid());
    }

    #[test]
    fn test_grid_search_filtering() {
        let mut state = DatasetsPaneState::new();
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
        state.update_grids(grids);
        assert_eq!(state.filtered_grids().len(), 2);

        state.add_grid_search_char('1');
        state.add_grid_search_char('4');
        assert_eq!(state.filtered_grids().len(), 1);
        assert_eq!(state.filtered_grids()[0].id, "ieee14");
    }

    #[test]
    fn test_grid_browser_toggle() {
        let mut state = DatasetsPaneState::new();
        assert!(!state.show_grid_browser);
        state.toggle_grid_browser();
        assert!(state.show_grid_browser);
        state.toggle_grid_browser();
        assert!(!state.show_grid_browser);
    }

    #[test]
    fn test_grid_load_reset() {
        let mut state = DatasetsPaneState::new();
        state.add_grid_path_char('/');
        state.add_grid_path_char('t');
        state.add_grid_path_char('e');
        state.add_grid_path_char('s');
        state.add_grid_path_char('t');

        assert_eq!(state.grid_load_path(), "/test");
        state.reset_grid_load();
        assert!(state.grid_load_path().is_empty());
    }
}
