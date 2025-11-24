use anyhow::{anyhow, Context, Result};
use polars::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a U.S. generator from EIA data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EiaGeneratorData {
    /// Unique generator identifier
    pub id: String,
    /// Generator name/plant name
    pub name: String,
    /// Primary fuel type (e.g., "Natural Gas", "Solar", "Wind")
    pub fuel_type: String,
    /// Nameplate capacity in MW
    pub capacity_mw: f64,
    /// Geographic latitude
    pub latitude: f64,
    /// Geographic longitude
    pub longitude: f64,
}

/// Represents a load/demand point from EIA data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EiaLoadData {
    /// Bus/zone identifier
    pub bus_id: String,
    /// Demand in MW
    pub demand_mw: f64,
    /// Geographic region
    pub region: String,
}

/// EIA Data Fetcher - retrieves U.S. grid data from EIA API
pub struct EiaDataFetcher {
    /// EIA API key (from https://www.eia.gov/opendata/)
    api_key: String,
    /// Base URL for EIA API
    base_url: String,
}

impl EiaDataFetcher {
    /// Create new EIA fetcher with API key
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.eia.gov/v2".to_string(),
        }
    }

    /// Fetch U.S. generator data from EIA API
    ///
    /// Returns a vector of EiaGeneratorData structs containing nameplate capacity
    /// and geographic location for all registered generators.
    pub fn fetch_generators(&self) -> Result<Vec<EiaGeneratorData>> {
        let url = format!(
            "{}/electricity/operating-generator-capacity/data/?api_key={}&data[0]=capacity-mw&sort[0][column]=capacity-mw&sort[0][direction]=desc&limit=10000",
            self.base_url, self.api_key
        );

        let response = ureq::get(&url).call().context("Failed to call EIA API")?;

        if response.status() != 200 {
            return Err(anyhow!("EIA API returned status {}", response.status()));
        }

        let body: serde_json::Value = response
            .into_json()
            .context("Failed to parse EIA JSON response")?;

        let mut generators = Vec::new();

        if let Some(data_array) = body["response"]["data"].as_array() {
            for (idx, item) in data_array.iter().enumerate() {
                let gen = EiaGeneratorData {
                    id: format!("GEN-{}", idx),
                    name: item["plantName"].as_str().unwrap_or("Unknown").to_string(),
                    fuel_type: item["fuelType"].as_str().unwrap_or("Other").to_string(),
                    capacity_mw: item["capacityMw"].as_f64().unwrap_or(0.0),
                    latitude: item["latitude"].as_f64().unwrap_or(0.0),
                    longitude: item["longitude"].as_f64().unwrap_or(0.0),
                };
                generators.push(gen);
            }
        }

        Ok(generators)
    }

    /// Convert generators to Arrow/Polars DataFrame
    pub fn generators_to_arrow(&self, generators: Vec<EiaGeneratorData>) -> Result<DataFrame> {
        let ids: Vec<String> = generators.iter().map(|g| g.id.clone()).collect();
        let names: Vec<String> = generators.iter().map(|g| g.name.clone()).collect();
        let fuel_types: Vec<String> = generators.iter().map(|g| g.fuel_type.clone()).collect();
        let capacities: Vec<f64> = generators.iter().map(|g| g.capacity_mw).collect();
        let lats: Vec<f64> = generators.iter().map(|g| g.latitude).collect();
        let lons: Vec<f64> = generators.iter().map(|g| g.longitude).collect();

        let df = DataFrame::new(vec![
            Series::new("gen_id", ids),
            Series::new("gen_name", names),
            Series::new("fuel_type", fuel_types),
            Series::new("capacity_mw", capacities),
            Series::new("latitude", lats),
            Series::new("longitude", lons),
        ])?;

        Ok(df)
    }
}

impl Default for EiaDataFetcher {
    fn default() -> Self {
        Self::new("".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eia_data_fetcher_init() {
        let fetcher = EiaDataFetcher::new("test_key".to_string());
        assert_eq!(fetcher.api_key, "test_key");
    }

    #[test]
    fn test_generators_to_arrow() {
        let fetcher = EiaDataFetcher::new("test".to_string());
        let gens = vec![EiaGeneratorData {
            id: "gen1".to_string(),
            name: "Plant A".to_string(),
            fuel_type: "Natural Gas".to_string(),
            capacity_mw: 500.0,
            latitude: 40.0,
            longitude: -75.0,
        }];

        let df = fetcher.generators_to_arrow(gens).unwrap();
        assert_eq!(df.height(), 1);
        assert_eq!(df.width(), 6);
    }

    #[test]
    fn test_generators_to_arrow_multiple() {
        let fetcher = EiaDataFetcher::new("test".to_string());
        let gens = vec![
            EiaGeneratorData {
                id: "gen1".to_string(),
                name: "Plant A".to_string(),
                fuel_type: "Natural Gas".to_string(),
                capacity_mw: 500.0,
                latitude: 40.0,
                longitude: -75.0,
            },
            EiaGeneratorData {
                id: "gen2".to_string(),
                name: "Plant B".to_string(),
                fuel_type: "Coal".to_string(),
                capacity_mw: 800.0,
                latitude: 41.0,
                longitude: -76.0,
            },
        ];

        let df = fetcher.generators_to_arrow(gens).unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 6);
    }
}
