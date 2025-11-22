/// Datasets Pane - Data catalog browsing, upload, and management
///
/// The datasets pane provides:
/// - Data catalog with public datasets
/// - Dataset preview and inspection
/// - Upload and download management
/// - Metadata and retention policies

use crate::components::*;

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
}
