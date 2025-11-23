use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct FeaturizeGeoSummary {
    pub num_polygons: usize,
    pub num_time_periods: usize,
    pub total_rows: usize,
    pub num_base_features: usize,
    pub num_total_features: usize,
}

pub fn featurize_spatial_timeseries(
    _mapping: &Path,
    _timeseries: &Path,
    _lags: &[usize],
    _windows: &[usize],
    _seasonal: bool,
    _out: &Path,
    _partitions: &[String],
) -> Result<FeaturizeGeoSummary> {
    println!("Performing spatial-temporal featurization (placeholder)...");
    Ok(FeaturizeGeoSummary {
        num_polygons: 0,
        num_time_periods: 0,
        total_rows: 0,
        num_base_features: 0,
        num_total_features: 0,
    })
}
