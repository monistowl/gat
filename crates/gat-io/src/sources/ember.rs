use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use polars::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a single carbon intensity and renewable data point from Ember
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmberDataPoint {
    /// ISO 8601 timestamp of the data point
    pub timestamp: DateTime<Utc>,
    /// Geographic region code (e.g., "US-West", "GB", "DE")
    pub region: String,
    /// Carbon intensity in grams of CO2 per kWh
    pub carbon_intensity_g_per_kwh: f64,
    /// Renewable energy percentage (0-100)
    pub renewable_pct: f64,
    /// Fossil fuel percentage (0-100)
    pub fossil_pct: f64,
}

/// Ember Climate Data Fetcher - retrieves real-time carbon and renewable data
pub struct EmberDataFetcher {
    /// Base URL for Ember API
    base_url: String,
}

impl EmberDataFetcher {
    /// Create new Ember fetcher with default endpoint
    pub fn new() -> Self {
        Self {
            base_url: "https://api.ember-climate.org/v1".to_string(),
        }
    }

    /// Fetch carbon intensity data for a region and date range
    ///
    /// # Arguments
    ///
    /// * `region` - Region code (e.g., "US-West", "GB")
    /// * `start_date` - Start date in format "YYYY-MM-DD"
    /// * `end_date` - End date in format "YYYY-MM-DD"
    ///
    /// # Returns
    ///
    /// Vector of EmberDataPoint structs with hourly carbon intensity and renewable data
    pub fn fetch_carbon_intensity(
        &self,
        region: &str,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<EmberDataPoint>> {
        let url = format!(
            "{}/carbon-intensity?region={}&start_date={}&end_date={}",
            self.base_url, region, start_date, end_date
        );

        let response = ureq::get(&url)
            .call()
            .context("Failed to call Ember API")?;

        if response.status() != 200 {
            return Err(anyhow!("Ember API returned status {}", response.status()));
        }

        let body: serde_json::Value = response
            .into_json()
            .context("Failed to parse Ember JSON")?;

        let mut points = Vec::new();

        if let Some(data_array) = body["data"].as_array() {
            for item in data_array {
                let timestamp_str = item["datetime"]
                    .as_str()
                    .context("Missing datetime field")?;

                let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                    .context("Failed to parse timestamp")?
                    .with_timezone(&Utc);

                let point = EmberDataPoint {
                    timestamp,
                    region: region.to_string(),
                    carbon_intensity_g_per_kwh: item["carbonIntensity"]
                        .as_f64()
                        .unwrap_or(0.0),
                    renewable_pct: item["renewablePercentage"]
                        .as_f64()
                        .unwrap_or(0.0),
                    fossil_pct: item["fossilPercentage"]
                        .as_f64()
                        .unwrap_or(0.0),
                };
                points.push(point);
            }
        }

        Ok(points)
    }

    /// Convert Ember data points to Arrow/Polars DataFrame
    ///
    /// Creates a time-series DataFrame with 5 columns:
    /// - timestamp (string in ISO 8601 format)
    /// - region (string)
    /// - carbon_intensity_g_per_kwh (f64)
    /// - renewable_pct (f64)
    /// - fossil_pct (f64)
    pub fn to_arrow(&self, points: Vec<EmberDataPoint>) -> Result<DataFrame> {
        let timestamps: Vec<String> = points.iter()
            .map(|p| p.timestamp.to_rfc3339())
            .collect();
        let regions: Vec<String> = points.iter()
            .map(|p| p.region.clone())
            .collect();
        let carbon_intensity: Vec<f64> = points.iter()
            .map(|p| p.carbon_intensity_g_per_kwh)
            .collect();
        let renewable: Vec<f64> = points.iter()
            .map(|p| p.renewable_pct)
            .collect();
        let fossil: Vec<f64> = points.iter()
            .map(|p| p.fossil_pct)
            .collect();

        let df = DataFrame::new(vec![
            Series::new("timestamp", timestamps),
            Series::new("region", regions),
            Series::new("carbon_intensity_g_per_kwh", carbon_intensity),
            Series::new("renewable_pct", renewable),
            Series::new("fossil_pct", fossil),
        ])?;

        Ok(df)
    }
}

impl Default for EmberDataFetcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ember_data_fetcher_init() {
        let fetcher = EmberDataFetcher::new();
        assert!(!fetcher.base_url.is_empty());
    }

    #[test]
    fn test_ember_to_arrow() {
        let fetcher = EmberDataFetcher::new();

        let points = vec![
            EmberDataPoint {
                timestamp: Utc::now(),
                region: "US-West".to_string(),
                carbon_intensity_g_per_kwh: 150.0,
                renewable_pct: 50.0,
                fossil_pct: 50.0,
            }
        ];

        let df = fetcher.to_arrow(points).unwrap();
        assert_eq!(df.height(), 1);
        assert_eq!(df.width(), 5);
    }

    #[test]
    fn test_ember_to_arrow_multiple() {
        let fetcher = EmberDataFetcher::new();

        let points = vec![
            EmberDataPoint {
                timestamp: Utc::now(),
                region: "US-West".to_string(),
                carbon_intensity_g_per_kwh: 150.0,
                renewable_pct: 50.0,
                fossil_pct: 50.0,
            },
            EmberDataPoint {
                timestamp: Utc::now(),
                region: "US-East".to_string(),
                carbon_intensity_g_per_kwh: 200.0,
                renewable_pct: 40.0,
                fossil_pct: 60.0,
            }
        ];

        let df = fetcher.to_arrow(points).unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 5);
    }

    #[test]
    fn test_ember_data_point_creation() {
        let point = EmberDataPoint {
            timestamp: Utc::now(),
            region: "GB".to_string(),
            carbon_intensity_g_per_kwh: 120.0,
            renewable_pct: 60.0,
            fossil_pct: 40.0,
        };

        assert_eq!(point.region, "GB");
        assert_eq!(point.carbon_intensity_g_per_kwh, 120.0);
        assert!(point.renewable_pct > 50.0);
    }
}
