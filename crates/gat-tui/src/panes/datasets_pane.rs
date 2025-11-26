/// Datasets Pane - Data catalog browsing, upload, and management
///
/// The datasets pane provides:
/// - Data catalog with public datasets
/// - Dataset preview and inspection
/// - Upload and download management
/// - Metadata and retention policies
/// - Scenario templates and loading
use crate::components::*;
use crate::ui::{GridBrowserState, GridInfo, GridLoadState};

/// Active tab in the Datasets pane
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatasetTab {
    Catalog,
    Uploads,
    Scenarios,
    Geo,
}

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

/// Scenario template entry
#[derive(Clone, Debug)]
pub struct ScenarioTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub variable_count: usize,
    pub last_modified: String,
    pub validation_status: ScenarioStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioStatus {
    Valid,
    Invalid,
    Untested,
}

impl ScenarioStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            ScenarioStatus::Valid => "✓",
            ScenarioStatus::Invalid => "✗",
            ScenarioStatus::Untested => "?",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ScenarioStatus::Valid => "Valid",
            ScenarioStatus::Invalid => "Invalid",
            ScenarioStatus::Untested => "Untested",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScenarioMetadata {
    pub total_scenarios: usize,
    pub loaded_count: usize,
    pub last_loaded: String,
    pub validation_status: String,
}

impl Default for ScenarioMetadata {
    fn default() -> Self {
        Self {
            total_scenarios: 0,
            loaded_count: 0,
            last_loaded: "Never".into(),
            validation_status: "No scenarios loaded".into(),
        }
    }
}

// ============================================================================
// Geo (GIS) types
// ============================================================================

/// Type of GIS layer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeoLayerType {
    /// Boundary polygons (zones, regions, states)
    Boundary,
    /// Transmission corridors and lines
    Transmission,
    /// Weather zones for load correlation
    WeatherZone,
    /// Custom user-defined layer
    Custom,
}

impl GeoLayerType {
    pub fn label(&self) -> &'static str {
        match self {
            GeoLayerType::Boundary => "Boundary",
            GeoLayerType::Transmission => "Transmission",
            GeoLayerType::WeatherZone => "Weather Zone",
            GeoLayerType::Custom => "Custom",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            GeoLayerType::Boundary => "◻",
            GeoLayerType::Transmission => "─",
            GeoLayerType::WeatherZone => "☁",
            GeoLayerType::Custom => "◆",
        }
    }
}

/// Status of a GIS layer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeoLayerStatus {
    Loaded,
    Loading,
    Error,
    NotLoaded,
}

impl GeoLayerStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            GeoLayerStatus::Loaded => "✓",
            GeoLayerStatus::Loading => "⟳",
            GeoLayerStatus::Error => "✗",
            GeoLayerStatus::NotLoaded => "○",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            GeoLayerStatus::Loaded => "Loaded",
            GeoLayerStatus::Loading => "Loading",
            GeoLayerStatus::Error => "Error",
            GeoLayerStatus::NotLoaded => "Not Loaded",
        }
    }
}

/// A GIS layer entry for the Geo browser
#[derive(Clone, Debug)]
pub struct GeoLayer {
    pub id: String,
    pub name: String,
    pub layer_type: GeoLayerType,
    pub source_path: String,
    pub feature_count: usize,
    pub crs: String,
    pub status: GeoLayerStatus,
}

impl GeoLayer {
    pub fn new(id: &str, name: &str, layer_type: GeoLayerType) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            layer_type,
            source_path: String::new(),
            feature_count: 0,
            crs: "EPSG:4326".to_string(),
            status: GeoLayerStatus::NotLoaded,
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.source_path = path.to_string();
        self
    }

    pub fn with_features(mut self, count: usize) -> Self {
        self.feature_count = count;
        self
    }

    pub fn with_crs(mut self, crs: &str) -> Self {
        self.crs = crs.to_string();
        self
    }

    pub fn with_status(mut self, status: GeoLayerStatus) -> Self {
        self.status = status;
        self
    }

    pub fn display_line(&self) -> String {
        format!(
            "{} {} {} ({})",
            self.layer_type.symbol(),
            self.name,
            self.status.symbol(),
            self.feature_count
        )
    }
}

/// Spatial join configuration for linking grid nodes to geo features
#[derive(Clone, Debug)]
pub struct SpatialJoinConfig {
    pub target_layer_id: String,
    pub join_type: SpatialJoinType,
    pub distance_threshold_km: f64,
    pub attribute_mapping: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialJoinType {
    /// Point in polygon containment
    Contains,
    /// Nearest neighbor within threshold
    Nearest,
    /// Intersection with buffer
    Intersects,
}

impl SpatialJoinType {
    pub fn label(&self) -> &'static str {
        match self {
            SpatialJoinType::Contains => "Contains",
            SpatialJoinType::Nearest => "Nearest",
            SpatialJoinType::Intersects => "Intersects",
        }
    }
}

impl Default for SpatialJoinConfig {
    fn default() -> Self {
        Self {
            target_layer_id: String::new(),
            join_type: SpatialJoinType::Contains,
            distance_threshold_km: 50.0,
            attribute_mapping: Vec::new(),
        }
    }
}

/// Spatial lag configuration for spatial regression models
#[derive(Clone, Debug)]
pub struct LagConfig {
    pub weight_matrix_type: WeightMatrixType,
    pub k_neighbors: usize,
    pub distance_decay: f64,
    pub row_standardize: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightMatrixType {
    /// k-nearest neighbors
    KNN,
    /// Distance-based with threshold
    Distance,
    /// Queen contiguity (shared edge or vertex)
    Queen,
    /// Rook contiguity (shared edge only)
    Rook,
}

impl WeightMatrixType {
    pub fn label(&self) -> &'static str {
        match self {
            WeightMatrixType::KNN => "K-Nearest Neighbors",
            WeightMatrixType::Distance => "Distance-Based",
            WeightMatrixType::Queen => "Queen Contiguity",
            WeightMatrixType::Rook => "Rook Contiguity",
        }
    }
}

impl Default for LagConfig {
    fn default() -> Self {
        Self {
            weight_matrix_type: WeightMatrixType::KNN,
            k_neighbors: 5,
            distance_decay: 1.0,
            row_standardize: true,
        }
    }
}

/// Datasets pane state
#[derive(Clone, Debug)]
pub struct DatasetsPaneState {
    // Tab navigation
    pub active_tab: DatasetTab,

    // Catalog
    pub datasets: Vec<Dataset>,
    pub selected_dataset: usize,

    // Uploads
    pub uploads: Vec<UploadJob>,
    pub selected_upload: usize,

    // Scenarios
    pub scenarios: Vec<ScenarioTemplate>,
    pub selected_scenario: usize,
    pub scenario_metadata: ScenarioMetadata,

    // Geo (GIS) layers
    pub geo_layers: Vec<GeoLayer>,
    pub selected_geo_layer: usize,
    pub spatial_join: SpatialJoinConfig,
    pub lag_config: LagConfig,

    // Metadata
    pub metadata: DatasetMetadata,

    // Grid management (Phase 3)
    pub grid_browser: GridBrowserState,
    pub grid_load: GridLoadState,
    pub show_grid_browser: bool,

    // Component states
    pub datasets_table: TableWidget,
    pub uploads_list: ListWidget,
    pub scenarios_list: ListWidget,
    pub geo_layers_list: ListWidget,
    pub search_input: InputWidget,
    pub metadata_text: TextWidget,
    pub scenario_details: TextWidget,
    pub geo_details: TextWidget,

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
            Column {
                header: "Name".into(),
                width: 25,
            },
            Column {
                header: "Size (MB)".into(),
                width: 12,
            },
            Column {
                header: "Rows".into(),
                width: 12,
            },
            Column {
                header: "Status".into(),
                width: 10,
            },
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

        let scenarios = vec![
            ScenarioTemplate {
                id: "scn_001".into(),
                name: "Summer Peak Load".into(),
                description: "High demand scenario with 15% load increase".into(),
                path: "/scenarios/summer_peak.yaml".into(),
                variable_count: 3,
                last_modified: "2024-11-20 09:30".into(),
                validation_status: ScenarioStatus::Valid,
            },
            ScenarioTemplate {
                id: "scn_002".into(),
                name: "High Renewable Penetration".into(),
                description: "80% renewable energy scenario with dispatch optimization".into(),
                path: "/scenarios/high_renewable.yaml".into(),
                variable_count: 5,
                last_modified: "2024-11-19 14:15".into(),
                validation_status: ScenarioStatus::Valid,
            },
            ScenarioTemplate {
                id: "scn_003".into(),
                name: "Generator Outage".into(),
                description: "Critical generator unavailable during peak hours".into(),
                path: "/scenarios/gen_outage.yaml".into(),
                variable_count: 2,
                last_modified: "2024-11-21 11:45".into(),
                validation_status: ScenarioStatus::Untested,
            },
        ];

        let mut scenarios_list = ListWidget::new("datasets_scenarios");
        for scenario in &scenarios {
            scenarios_list.add_item(
                format!("{} {}", scenario.name, scenario.validation_status.symbol()),
                scenario.id.clone(),
            );
        }

        // Geo layers sample data
        let geo_layers = vec![
            GeoLayer::new("geo_001", "US State Boundaries", GeoLayerType::Boundary)
                .with_path("/data/geo/us_states.shp")
                .with_features(51)
                .with_status(GeoLayerStatus::Loaded),
            GeoLayer::new(
                "geo_002",
                "Transmission Corridors",
                GeoLayerType::Transmission,
            )
            .with_path("/data/geo/transmission.geojson")
            .with_features(1250)
            .with_status(GeoLayerStatus::Loaded),
            GeoLayer::new("geo_003", "NOAA Weather Zones", GeoLayerType::WeatherZone)
                .with_path("/data/geo/weather_zones.shp")
                .with_features(122)
                .with_status(GeoLayerStatus::NotLoaded),
        ];

        let mut geo_layers_list = ListWidget::new("datasets_geo_layers");
        for layer in &geo_layers {
            geo_layers_list.add_item(layer.display_line(), layer.id.clone());
        }

        let metadata = DatasetMetadata {
            retention_days: 30,
            backup_schedule: "Nightly".into(),
            total_size_gb: 500.0,
            available_size_gb: 125.5,
        };

        let scenario_metadata = ScenarioMetadata {
            total_scenarios: scenarios.len(),
            loaded_count: 2,
            last_loaded: "2024-11-21 10:00".into(),
            validation_status: "2 valid, 1 untested".into(),
        };

        DatasetsPaneState {
            active_tab: DatasetTab::Catalog,
            datasets,
            selected_dataset: 0,
            uploads,
            selected_upload: 0,
            scenarios,
            selected_scenario: 0,
            scenario_metadata,
            geo_layers,
            selected_geo_layer: 0,
            spatial_join: SpatialJoinConfig::default(),
            lag_config: LagConfig::default(),
            metadata,
            grid_browser: GridBrowserState::new(Vec::new()),
            grid_load: GridLoadState::new(),
            show_grid_browser: false,
            datasets_table,
            uploads_list,
            scenarios_list,
            geo_layers_list,
            search_input: InputWidget::new("dataset_search").with_placeholder("Search datasets..."),
            metadata_text: TextWidget::new("dataset_metadata", ""),
            scenario_details: TextWidget::new("scenario_details", ""),
            geo_details: TextWidget::new("geo_details", ""),
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
                .filter(|d| {
                    d.name.to_lowercase().contains(&query)
                        || d.format.to_lowercase().contains(&query)
                })
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

    // Tab navigation methods (Phase 5)

    pub fn switch_tab(&mut self, tab: DatasetTab) {
        self.active_tab = tab;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            DatasetTab::Catalog => DatasetTab::Uploads,
            DatasetTab::Uploads => DatasetTab::Scenarios,
            DatasetTab::Scenarios => DatasetTab::Geo,
            DatasetTab::Geo => DatasetTab::Catalog,
        };
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = match self.active_tab {
            DatasetTab::Catalog => DatasetTab::Geo,
            DatasetTab::Uploads => DatasetTab::Catalog,
            DatasetTab::Scenarios => DatasetTab::Uploads,
            DatasetTab::Geo => DatasetTab::Scenarios,
        };
    }

    pub fn is_catalog_tab(&self) -> bool {
        self.active_tab == DatasetTab::Catalog
    }

    pub fn is_uploads_tab(&self) -> bool {
        self.active_tab == DatasetTab::Uploads
    }

    pub fn is_scenarios_tab(&self) -> bool {
        self.active_tab == DatasetTab::Scenarios
    }

    pub fn is_geo_tab(&self) -> bool {
        self.active_tab == DatasetTab::Geo
    }

    // Scenario management methods

    pub fn select_next_scenario(&mut self) {
        if self.selected_scenario < self.scenarios.len().saturating_sub(1) {
            self.selected_scenario += 1;
        }
    }

    pub fn select_prev_scenario(&mut self) {
        if self.selected_scenario > 0 {
            self.selected_scenario -= 1;
        }
    }

    pub fn selected_scenario(&self) -> Option<&ScenarioTemplate> {
        self.scenarios.get(self.selected_scenario)
    }

    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    pub fn load_scenarios(&mut self, scenarios: Vec<ScenarioTemplate>) {
        self.scenarios = scenarios;
        self.selected_scenario = 0;
        self.scenario_metadata.loaded_count = self.scenarios.len();
        self.scenario_metadata.last_loaded =
            chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    }

    pub fn get_scenario_details(&mut self) -> String {
        if let Some(scenario) = self.selected_scenario() {
            format!(
                "Name: {}\nDescription: {}\nPath: {}\nVariables: {}\nStatus: {}\nModified: {}",
                scenario.name,
                scenario.description,
                scenario.path,
                scenario.variable_count,
                scenario.validation_status.label(),
                scenario.last_modified,
            )
        } else {
            "No scenario selected".into()
        }
    }

    pub fn validate_scenario(&self) -> bool {
        if let Some(scenario) = self.selected_scenario() {
            scenario.validation_status == ScenarioStatus::Valid
        } else {
            false
        }
    }

    pub fn format_scenario_metadata(&mut self) {
        let status = &self.scenario_metadata;
        self.scenario_details.set_content(format!(
            "Total Scenarios: {}\nLoaded: {}\nLast Loaded: {}\nValidation: {}",
            status.total_scenarios,
            status.loaded_count,
            status.last_loaded,
            status.validation_status,
        ));
    }

    // ============================================================================
    // Geo (GIS) management methods
    // ============================================================================

    /// Navigate to next geo layer
    pub fn select_next_geo_layer(&mut self) {
        if self.selected_geo_layer < self.geo_layers.len().saturating_sub(1) {
            self.selected_geo_layer += 1;
        }
    }

    /// Navigate to previous geo layer
    pub fn select_prev_geo_layer(&mut self) {
        if self.selected_geo_layer > 0 {
            self.selected_geo_layer -= 1;
        }
    }

    /// Get the currently selected geo layer
    pub fn selected_geo_layer(&self) -> Option<&GeoLayer> {
        self.geo_layers.get(self.selected_geo_layer)
    }

    /// Get geo layer count
    pub fn geo_layer_count(&self) -> usize {
        self.geo_layers.len()
    }

    /// Add a new geo layer
    pub fn add_geo_layer(&mut self, layer: GeoLayer) {
        self.geo_layers_list
            .add_item(layer.display_line(), layer.id.clone());
        self.geo_layers.push(layer);
    }

    /// Remove a geo layer by id
    pub fn remove_geo_layer(&mut self, id: &str) -> bool {
        if let Some(pos) = self.geo_layers.iter().position(|l| l.id == id) {
            self.geo_layers.remove(pos);
            if self.selected_geo_layer >= self.geo_layers.len() && self.selected_geo_layer > 0 {
                self.selected_geo_layer -= 1;
            }
            true
        } else {
            false
        }
    }

    /// Get formatted details for the selected geo layer
    pub fn get_geo_layer_details(&self) -> String {
        if let Some(layer) = self.selected_geo_layer() {
            format!(
                "Name: {}\nType: {}\nPath: {}\nFeatures: {}\nCRS: {}\nStatus: {}",
                layer.name,
                layer.layer_type.label(),
                layer.source_path,
                layer.feature_count,
                layer.crs,
                layer.status.label(),
            )
        } else {
            "No layer selected".into()
        }
    }

    /// Update geo details text widget
    pub fn format_geo_details(&mut self) {
        let details = self.get_geo_layer_details();
        self.geo_details.set_content(details);
    }

    /// Get layers filtered by type
    pub fn geo_layers_by_type(&self, layer_type: GeoLayerType) -> Vec<&GeoLayer> {
        self.geo_layers
            .iter()
            .filter(|l| l.layer_type == layer_type)
            .collect()
    }

    /// Get loaded layers only
    pub fn loaded_geo_layers(&self) -> Vec<&GeoLayer> {
        self.geo_layers
            .iter()
            .filter(|l| l.status == GeoLayerStatus::Loaded)
            .collect()
    }

    // Spatial join configuration

    /// Set the target layer for spatial join
    pub fn set_spatial_join_target(&mut self, layer_id: &str) {
        self.spatial_join.target_layer_id = layer_id.to_string();
    }

    /// Set the spatial join type
    pub fn set_spatial_join_type(&mut self, join_type: SpatialJoinType) {
        self.spatial_join.join_type = join_type;
    }

    /// Set the distance threshold for spatial join (in km)
    pub fn set_spatial_join_distance(&mut self, distance_km: f64) {
        self.spatial_join.distance_threshold_km = distance_km;
    }

    /// Add attribute mapping for spatial join
    pub fn add_spatial_join_mapping(&mut self, source: &str, target: &str) {
        self.spatial_join
            .attribute_mapping
            .push((source.to_string(), target.to_string()));
    }

    /// Clear spatial join configuration
    pub fn clear_spatial_join(&mut self) {
        self.spatial_join = SpatialJoinConfig::default();
    }

    /// Get spatial join summary
    pub fn get_spatial_join_summary(&self) -> String {
        format!(
            "Target: {}\nType: {}\nDistance: {:.1} km\nMappings: {}",
            if self.spatial_join.target_layer_id.is_empty() {
                "None"
            } else {
                &self.spatial_join.target_layer_id
            },
            self.spatial_join.join_type.label(),
            self.spatial_join.distance_threshold_km,
            self.spatial_join.attribute_mapping.len(),
        )
    }

    // Spatial lag configuration

    /// Set weight matrix type for spatial lag
    pub fn set_lag_weight_type(&mut self, weight_type: WeightMatrixType) {
        self.lag_config.weight_matrix_type = weight_type;
    }

    /// Set number of neighbors for KNN
    pub fn set_lag_k_neighbors(&mut self, k: usize) {
        self.lag_config.k_neighbors = k;
    }

    /// Set distance decay parameter
    pub fn set_lag_distance_decay(&mut self, decay: f64) {
        self.lag_config.distance_decay = decay;
    }

    /// Toggle row standardization
    pub fn toggle_lag_row_standardize(&mut self) {
        self.lag_config.row_standardize = !self.lag_config.row_standardize;
    }

    /// Get lag config summary
    pub fn get_lag_config_summary(&self) -> String {
        format!(
            "Type: {}\nK Neighbors: {}\nDecay: {:.2}\nRow Standardized: {}",
            self.lag_config.weight_matrix_type.label(),
            self.lag_config.k_neighbors,
            self.lag_config.distance_decay,
            if self.lag_config.row_standardize {
                "Yes"
            } else {
                "No"
            },
        )
    }

    /// Reset lag config to defaults
    pub fn reset_lag_config(&mut self) {
        self.lag_config = LagConfig::default();
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

    // Tab navigation tests (Phase 5)

    #[test]
    fn test_tab_initialization() {
        let state = DatasetsPaneState::new();
        assert_eq!(state.active_tab, DatasetTab::Catalog);
    }

    #[test]
    fn test_next_tab_cycle() {
        let mut state = DatasetsPaneState::new();
        assert_eq!(state.active_tab, DatasetTab::Catalog);

        state.next_tab();
        assert_eq!(state.active_tab, DatasetTab::Uploads);

        state.next_tab();
        assert_eq!(state.active_tab, DatasetTab::Scenarios);

        state.next_tab();
        assert_eq!(state.active_tab, DatasetTab::Geo);

        state.next_tab();
        assert_eq!(state.active_tab, DatasetTab::Catalog);
    }

    #[test]
    fn test_prev_tab_cycle() {
        let mut state = DatasetsPaneState::new();
        state.active_tab = DatasetTab::Catalog;

        state.prev_tab();
        assert_eq!(state.active_tab, DatasetTab::Geo);

        state.prev_tab();
        assert_eq!(state.active_tab, DatasetTab::Scenarios);

        state.prev_tab();
        assert_eq!(state.active_tab, DatasetTab::Uploads);

        state.prev_tab();
        assert_eq!(state.active_tab, DatasetTab::Catalog);
    }

    #[test]
    fn test_switch_tab() {
        let mut state = DatasetsPaneState::new();
        state.switch_tab(DatasetTab::Scenarios);
        assert_eq!(state.active_tab, DatasetTab::Scenarios);

        state.switch_tab(DatasetTab::Uploads);
        assert_eq!(state.active_tab, DatasetTab::Uploads);
    }

    #[test]
    fn test_tab_query_methods() {
        let mut state = DatasetsPaneState::new();
        assert!(state.is_catalog_tab());
        assert!(!state.is_uploads_tab());
        assert!(!state.is_scenarios_tab());

        state.next_tab();
        assert!(!state.is_catalog_tab());
        assert!(state.is_uploads_tab());
        assert!(!state.is_scenarios_tab());

        state.next_tab();
        assert!(!state.is_catalog_tab());
        assert!(!state.is_uploads_tab());
        assert!(state.is_scenarios_tab());
    }

    // Scenario management tests

    #[test]
    fn test_scenarios_init() {
        let state = DatasetsPaneState::new();
        assert_eq!(state.scenario_count(), 3);
        assert_eq!(state.selected_scenario, 0);
    }

    #[test]
    fn test_scenario_selection_navigation() {
        let mut state = DatasetsPaneState::new();
        assert_eq!(state.selected_scenario, 0);

        state.select_next_scenario();
        assert_eq!(state.selected_scenario, 1);

        state.select_next_scenario();
        assert_eq!(state.selected_scenario, 2);

        state.select_next_scenario();
        assert_eq!(state.selected_scenario, 2); // Bounds check

        state.select_prev_scenario();
        assert_eq!(state.selected_scenario, 1);

        state.select_prev_scenario();
        assert_eq!(state.selected_scenario, 0);
    }

    #[test]
    fn test_selected_scenario() {
        let state = DatasetsPaneState::new();
        let scenario = state.selected_scenario().unwrap();
        assert_eq!(scenario.id, "scn_001");
        assert_eq!(scenario.name, "Summer Peak Load");
    }

    #[test]
    fn test_scenario_validation() {
        let state = DatasetsPaneState::new();
        assert!(state.validate_scenario()); // First scenario is valid

        let mut state = state.clone();
        state.select_next_scenario();
        state.select_next_scenario(); // Third scenario
        assert!(!state.validate_scenario()); // Third is untested
    }

    #[test]
    fn test_scenario_details_formatting() {
        let mut state = DatasetsPaneState::new();
        let details = state.get_scenario_details();
        assert!(details.contains("Summer Peak Load"));
        assert!(details.contains("High demand scenario"));
        assert!(details.contains("Valid"));
    }

    #[test]
    fn test_scenario_metadata_formatting() {
        let mut state = DatasetsPaneState::new();
        state.format_scenario_metadata();
        assert!(state
            .scenario_details
            .content
            .contains("Total Scenarios: 3"));
        assert!(state.scenario_details.content.contains("Loaded: 2"));
    }

    #[test]
    fn test_scenario_status_symbol() {
        assert_eq!(ScenarioStatus::Valid.symbol(), "✓");
        assert_eq!(ScenarioStatus::Invalid.symbol(), "✗");
        assert_eq!(ScenarioStatus::Untested.symbol(), "?");
    }

    #[test]
    fn test_scenario_status_label() {
        assert_eq!(ScenarioStatus::Valid.label(), "Valid");
        assert_eq!(ScenarioStatus::Invalid.label(), "Invalid");
        assert_eq!(ScenarioStatus::Untested.label(), "Untested");
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

    // ========================================================================
    // Geo (GIS) management tests
    // ========================================================================

    #[test]
    fn test_geo_layers_init() {
        let state = DatasetsPaneState::new();
        assert_eq!(state.geo_layer_count(), 3);
        assert_eq!(state.selected_geo_layer, 0);
    }

    #[test]
    fn test_geo_layer_selection_navigation() {
        let mut state = DatasetsPaneState::new();
        assert_eq!(state.selected_geo_layer, 0);

        state.select_next_geo_layer();
        assert_eq!(state.selected_geo_layer, 1);

        state.select_next_geo_layer();
        assert_eq!(state.selected_geo_layer, 2);

        state.select_next_geo_layer();
        assert_eq!(state.selected_geo_layer, 2); // Bounds check

        state.select_prev_geo_layer();
        assert_eq!(state.selected_geo_layer, 1);

        state.select_prev_geo_layer();
        assert_eq!(state.selected_geo_layer, 0);

        state.select_prev_geo_layer();
        assert_eq!(state.selected_geo_layer, 0); // Bounds check
    }

    #[test]
    fn test_selected_geo_layer() {
        let state = DatasetsPaneState::new();
        let layer = state.selected_geo_layer().unwrap();
        assert_eq!(layer.id, "geo_001");
        assert_eq!(layer.name, "US State Boundaries");
        assert_eq!(layer.layer_type, GeoLayerType::Boundary);
    }

    #[test]
    fn test_geo_layer_type_labels() {
        assert_eq!(GeoLayerType::Boundary.label(), "Boundary");
        assert_eq!(GeoLayerType::Transmission.label(), "Transmission");
        assert_eq!(GeoLayerType::WeatherZone.label(), "Weather Zone");
        assert_eq!(GeoLayerType::Custom.label(), "Custom");
    }

    #[test]
    fn test_geo_layer_type_symbols() {
        assert_eq!(GeoLayerType::Boundary.symbol(), "◻");
        assert_eq!(GeoLayerType::Transmission.symbol(), "─");
        assert_eq!(GeoLayerType::WeatherZone.symbol(), "☁");
        assert_eq!(GeoLayerType::Custom.symbol(), "◆");
    }

    #[test]
    fn test_geo_layer_status_labels() {
        assert_eq!(GeoLayerStatus::Loaded.label(), "Loaded");
        assert_eq!(GeoLayerStatus::Loading.label(), "Loading");
        assert_eq!(GeoLayerStatus::Error.label(), "Error");
        assert_eq!(GeoLayerStatus::NotLoaded.label(), "Not Loaded");
    }

    #[test]
    fn test_geo_layer_status_symbols() {
        assert_eq!(GeoLayerStatus::Loaded.symbol(), "✓");
        assert_eq!(GeoLayerStatus::Loading.symbol(), "⟳");
        assert_eq!(GeoLayerStatus::Error.symbol(), "✗");
        assert_eq!(GeoLayerStatus::NotLoaded.symbol(), "○");
    }

    #[test]
    fn test_geo_layer_builder() {
        let layer = GeoLayer::new("test_001", "Test Layer", GeoLayerType::Custom)
            .with_path("/data/test.shp")
            .with_features(100)
            .with_crs("EPSG:3857")
            .with_status(GeoLayerStatus::Loaded);

        assert_eq!(layer.id, "test_001");
        assert_eq!(layer.name, "Test Layer");
        assert_eq!(layer.source_path, "/data/test.shp");
        assert_eq!(layer.feature_count, 100);
        assert_eq!(layer.crs, "EPSG:3857");
        assert_eq!(layer.status, GeoLayerStatus::Loaded);
    }

    #[test]
    fn test_add_geo_layer() {
        let mut state = DatasetsPaneState::new();
        let initial_count = state.geo_layer_count();

        let new_layer = GeoLayer::new("new_001", "New Layer", GeoLayerType::Custom)
            .with_features(50)
            .with_status(GeoLayerStatus::NotLoaded);

        state.add_geo_layer(new_layer);
        assert_eq!(state.geo_layer_count(), initial_count + 1);
    }

    #[test]
    fn test_remove_geo_layer() {
        let mut state = DatasetsPaneState::new();
        let initial_count = state.geo_layer_count();

        assert!(state.remove_geo_layer("geo_001"));
        assert_eq!(state.geo_layer_count(), initial_count - 1);

        // Try to remove non-existent layer
        assert!(!state.remove_geo_layer("nonexistent"));
    }

    #[test]
    fn test_geo_layer_details() {
        let state = DatasetsPaneState::new();
        let details = state.get_geo_layer_details();
        assert!(details.contains("US State Boundaries"));
        assert!(details.contains("Boundary"));
        assert!(details.contains("EPSG:4326"));
    }

    #[test]
    fn test_geo_layers_by_type() {
        let state = DatasetsPaneState::new();
        let boundaries = state.geo_layers_by_type(GeoLayerType::Boundary);
        assert_eq!(boundaries.len(), 1);
        assert_eq!(boundaries[0].name, "US State Boundaries");
    }

    #[test]
    fn test_loaded_geo_layers() {
        let state = DatasetsPaneState::new();
        let loaded = state.loaded_geo_layers();
        assert_eq!(loaded.len(), 2); // Two are loaded in sample data
    }

    #[test]
    fn test_is_geo_tab() {
        let mut state = DatasetsPaneState::new();
        assert!(!state.is_geo_tab());

        state.switch_tab(DatasetTab::Geo);
        assert!(state.is_geo_tab());
    }

    // Spatial join config tests

    #[test]
    fn test_spatial_join_config_default() {
        let state = DatasetsPaneState::new();
        assert!(state.spatial_join.target_layer_id.is_empty());
        assert_eq!(state.spatial_join.join_type, SpatialJoinType::Contains);
        assert_eq!(state.spatial_join.distance_threshold_km, 50.0);
    }

    #[test]
    fn test_spatial_join_type_labels() {
        assert_eq!(SpatialJoinType::Contains.label(), "Contains");
        assert_eq!(SpatialJoinType::Nearest.label(), "Nearest");
        assert_eq!(SpatialJoinType::Intersects.label(), "Intersects");
    }

    #[test]
    fn test_set_spatial_join_config() {
        let mut state = DatasetsPaneState::new();

        state.set_spatial_join_target("geo_001");
        assert_eq!(state.spatial_join.target_layer_id, "geo_001");

        state.set_spatial_join_type(SpatialJoinType::Nearest);
        assert_eq!(state.spatial_join.join_type, SpatialJoinType::Nearest);

        state.set_spatial_join_distance(100.0);
        assert_eq!(state.spatial_join.distance_threshold_km, 100.0);

        state.add_spatial_join_mapping("zone_id", "node_zone");
        assert_eq!(state.spatial_join.attribute_mapping.len(), 1);
    }

    #[test]
    fn test_clear_spatial_join() {
        let mut state = DatasetsPaneState::new();
        state.set_spatial_join_target("geo_001");
        state.set_spatial_join_type(SpatialJoinType::Nearest);

        state.clear_spatial_join();
        assert!(state.spatial_join.target_layer_id.is_empty());
        assert_eq!(state.spatial_join.join_type, SpatialJoinType::Contains);
    }

    #[test]
    fn test_spatial_join_summary() {
        let mut state = DatasetsPaneState::new();
        state.set_spatial_join_target("geo_001");
        state.set_spatial_join_type(SpatialJoinType::Nearest);

        let summary = state.get_spatial_join_summary();
        assert!(summary.contains("geo_001"));
        assert!(summary.contains("Nearest"));
    }

    // Lag config tests

    #[test]
    fn test_lag_config_default() {
        let state = DatasetsPaneState::new();
        assert_eq!(state.lag_config.weight_matrix_type, WeightMatrixType::KNN);
        assert_eq!(state.lag_config.k_neighbors, 5);
        assert_eq!(state.lag_config.distance_decay, 1.0);
        assert!(state.lag_config.row_standardize);
    }

    #[test]
    fn test_weight_matrix_type_labels() {
        assert_eq!(WeightMatrixType::KNN.label(), "K-Nearest Neighbors");
        assert_eq!(WeightMatrixType::Distance.label(), "Distance-Based");
        assert_eq!(WeightMatrixType::Queen.label(), "Queen Contiguity");
        assert_eq!(WeightMatrixType::Rook.label(), "Rook Contiguity");
    }

    #[test]
    fn test_set_lag_config() {
        let mut state = DatasetsPaneState::new();

        state.set_lag_weight_type(WeightMatrixType::Queen);
        assert_eq!(state.lag_config.weight_matrix_type, WeightMatrixType::Queen);

        state.set_lag_k_neighbors(10);
        assert_eq!(state.lag_config.k_neighbors, 10);

        state.set_lag_distance_decay(2.0);
        assert_eq!(state.lag_config.distance_decay, 2.0);

        state.toggle_lag_row_standardize();
        assert!(!state.lag_config.row_standardize);

        state.toggle_lag_row_standardize();
        assert!(state.lag_config.row_standardize);
    }

    #[test]
    fn test_lag_config_summary() {
        let state = DatasetsPaneState::new();
        let summary = state.get_lag_config_summary();
        assert!(summary.contains("K-Nearest Neighbors"));
        assert!(summary.contains("5"));
        assert!(summary.contains("Yes"));
    }

    #[test]
    fn test_reset_lag_config() {
        let mut state = DatasetsPaneState::new();
        state.set_lag_weight_type(WeightMatrixType::Queen);
        state.set_lag_k_neighbors(10);

        state.reset_lag_config();
        assert_eq!(state.lag_config.weight_matrix_type, WeightMatrixType::KNN);
        assert_eq!(state.lag_config.k_neighbors, 5);
    }

    #[test]
    fn test_geo_layer_display_line() {
        let layer = GeoLayer::new("test", "Test Layer", GeoLayerType::Boundary)
            .with_features(100)
            .with_status(GeoLayerStatus::Loaded);

        let display = layer.display_line();
        assert!(display.contains("◻")); // Boundary symbol
        assert!(display.contains("Test Layer"));
        assert!(display.contains("✓")); // Loaded symbol
        assert!(display.contains("100"));
    }
}
