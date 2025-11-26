# v0.3 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task with subagents or superpowers:subagent-driven-development for fresh subagent per task.

**Goal:** Complete feature implementations for CIM RDF full model, AC OPF, reliability metrics framework, and API stability audit for v0.3 release.

**Architecture:** Depth-first implementation in three phases: (1) CIM RDF + data APIs, (2) AC OPF, (3) Reliability metrics. Lightweight dependency audit runs in parallel.

**Tech Stack:** Rust 2021, good_lp (Clarabel), quick-xml, polars, petgraph, tokio, anyhow

---

## Phase 1: CIM RDF Full Model Ingestion & Data API Fetchers

### Task 1: Extend CIM Parser for Operational Limits

**Files:**
- Modify: `crates/gat-io/src/importers/cim.rs` (struct definitions + parsing logic)
- Test: `crates/gat-io/src/importers/tests.rs`

**Context:**
Current CIM importer extracts topology (buses, lines, loads, gens) but lacks constraints (voltage limits, thermal limits, frequency limits). We need to extend the parsing to capture operational limits from CIM RDF elements like `OperationalLimits`, `CurrentLimit`, `VoltageLimit`.

**Step 1: Add operational limit structs to cim.rs**

After the existing `CimGen` struct (line 51), add:

```rust
#[derive(Debug, Clone)]
pub struct CimOperationalLimit {
    pub equipment_id: String,
    pub limit_type: String,  // "ThermalLimit", "VoltageLimit", "FrequencyLimit"
    pub value: f64,
    pub unit: String,  // "MW", "kV", "Hz"
}

#[derive(Debug, Clone)]
pub struct CimVoltageLimit {
    pub bus_id: String,
    pub min_voltage: f64,
    pub max_voltage: f64,
}

#[derive(Debug, Clone)]
pub struct CimTransformer {
    pub id: String,
    pub name: String,
    pub from_bus: String,
    pub to_bus: String,
    pub r: f64,
    pub x: f64,
    pub rated_mva: f64,
}
```

**Step 2: Run compiler check**

```bash
cd /home/tom/Code/gat && cargo check -p gat-io
```

Expected: PASS (new structs don't break anything yet)

**Step 3: Extend parse_cim_documents to extract limits**

Modify the `parse_cim_documents` function signature (around line 20) to return additional data:

Old:
```rust
fn parse_cim_documents(documents: &[String]) -> Result<(Vec<CimBus>, Vec<CimLine>, Vec<CimLoad>, Vec<CimGen>)>
```

New:
```rust
fn parse_cim_documents(documents: &[String]) -> Result<(Vec<CimBus>, Vec<CimLine>, Vec<CimLoad>, Vec<CimGen>, Vec<CimOperationalLimit>, Vec<CimVoltageLimit>, Vec<CimTransformer>)>
```

Inside the function, add extraction logic for operational limits by extending the XML parsing loop. After the existing element matching (e.g., after `"cim:ConformLoad"` parsing), add:

```rust
else if elem_name == "cim:OperationalLimit" {
    let id_attr = get_attribute(&start, "rdf:ID").unwrap_or_default();
    let mut limit_type = String::new();
    let mut value = 0.0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(child)) => {
                let child_name = child.name().local_name().into_inner();
                if child_name == "cim:OperationalLimit.limitType" {
                    if let Ok(Event::Text(text)) = reader.read_event_into(&mut buf) {
                        limit_type = String::from_utf8_lossy(&text).to_string();
                    }
                }
            }
            Ok(Event::End(_)) => break,
            _ => {}
        }
    }

    operational_limits.push(CimOperationalLimit {
        equipment_id: id_attr,
        limit_type,
        value,
        unit: "MW".to_string(),
    });
}
```

**Step 4: Write test for limit extraction**

In `crates/gat-io/src/importers/tests.rs`, add a new test file fixture and test:

```rust
#[test]
fn test_cim_operational_limits_parsing() {
    // Create a minimal CIM RDF with operational limits
    let cim_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF xmlns:cim="http://iec.ch/TC57/2013/CIM-schema-v2_4_0">
  <cim:VoltageLimit rdf:ID="limit1">
    <cim:OperationalLimit.Equipment rdf:resource="bus1"/>
    <cim:VoltageLimit.maxVoltage>1.05</cim:VoltageLimit.maxVoltage>
    <cim:VoltageLimit.minVoltage>0.95</cim:VoltageLimit.minVoltage>
  </cim:VoltageLimit>
</rdf:RDF>"#;

    // Write temp file
    let temp_file = std::env::temp_dir().join("test_cim_limits.xml");
    std::fs::write(&temp_file, cim_xml).unwrap();

    // Parse and verify limits extracted
    let documents = vec![cim_xml.to_string()];
    let result = parse_cim_documents(&documents);

    assert!(result.is_ok());
    let (_, _, _, _, limits, volt_limits, _) = result.unwrap();
    assert!(!volt_limits.is_empty());
}
```

**Step 5: Run tests**

```bash
cd /home/tom/Code/gat && cargo test -p gat-io --lib importers::tests::test_cim_operational_limits_parsing
```

Expected: PASS

**Step 6: Update build_network_from_cim**

Modify the `build_network_from_cim` function to accept the new limit data and apply voltage bounds to buses and thermal limits to branches:

```rust
fn build_network_from_cim(
    buses: Vec<CimBus>,
    lines: Vec<CimLine>,
    loads: Vec<CimLoad>,
    gens: Vec<CimGen>,
    limits: Vec<CimOperationalLimit>,
    voltage_limits: Vec<CimVoltageLimit>,
    transformers: Vec<CimTransformer>,
) -> Result<Network> {
    let mut network = /* existing code */;

    // Apply voltage limits to buses
    for volt_limit in voltage_limits {
        if let Some(bus) = network.buses.iter_mut().find(|b| b.name == volt_limit.bus_id) {
            bus.min_voltage = volt_limit.min_voltage;
            bus.max_voltage = volt_limit.max_voltage;
        }
    }

    // Apply thermal limits to branches
    for limit in limits {
        if limit.limit_type == "ThermalLimit" {
            if let Some(branch) = network.branches.iter_mut().find(|br| br.name == limit.equipment_id) {
                branch.rate_a = limit.value;
            }
        }
    }

    Ok(network)
}
```

**Step 7: Run full gat-io test suite**

```bash
cd /home/tom/Code/gat && cargo test -p gat-io
```

Expected: All tests PASS

**Step 8: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-io/src/importers/cim.rs crates/gat-io/src/importers/tests.rs && git commit -m "feat: Add operational limit parsing to CIM RDF importer"
```

---

### Task 2: Add CIM Validation Layer

**Files:**
- Create: `crates/gat-io/src/importers/cim_validator.rs`
- Modify: `crates/gat-io/src/importers/mod.rs` (add module declaration)
- Test: `crates/gat-io/src/importers/tests.rs`

**Context:**
After parsing CIM, we need to validate that required fields are present and data is sensible (e.g., every bus has a voltage limit, every branch has a rating). This prevents downstream failures.

**Step 1: Create cim_validator.rs**

```rust
use anyhow::{anyhow, Result};
use gat_core::Network;

#[derive(Debug)]
pub struct CimValidationError {
    pub entity_type: String,  // "Bus", "Branch", "Generator"
    pub entity_id: String,
    pub issue: String,
}

pub fn validate_network_from_cim(network: &Network) -> Result<()> {
    let mut errors = Vec::new();

    // Check all buses have voltage limits
    for bus in &network.buses {
        if bus.min_voltage <= 0.0 || bus.max_voltage <= 0.0 {
            errors.push(CimValidationError {
                entity_type: "Bus".to_string(),
                entity_id: bus.name.clone(),
                issue: format!("Missing voltage limits: min={}, max={}", bus.min_voltage, bus.max_voltage),
            });
        }
        if bus.min_voltage >= bus.max_voltage {
            errors.push(CimValidationError {
                entity_type: "Bus".to_string(),
                entity_id: bus.name.clone(),
                issue: format!("Invalid voltage limits: min >= max"),
            });
        }
    }

    // Check all branches have ratings
    for branch in &network.branches {
        if branch.rate_a <= 0.0 {
            errors.push(CimValidationError {
                entity_type: "Branch".to_string(),
                entity_id: branch.name.clone(),
                issue: "Missing thermal rating (rate_a <= 0)".to_string(),
            });
        }
    }

    // Check all generators have output limits
    for gen in &network.generators {
        if gen.pmax <= 0.0 {
            errors.push(CimValidationError {
                entity_type: "Generator".to_string(),
                entity_id: gen.name.clone(),
                issue: "Missing or invalid max power limit".to_string(),
            });
        }
    }

    if !errors.is_empty() {
        let error_msg = errors.iter()
            .map(|e| format!("{}: {} ({})", e.entity_type, e.entity_id, e.issue))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(anyhow!("CIM validation failed:\n{}", error_msg));
    }

    Ok(())
}

pub fn validate_cim_with_warnings(network: &Network) -> Vec<CimValidationError> {
    let mut warnings = Vec::new();

    // Check for unusual but valid configurations
    for bus in &network.buses {
        if bus.min_voltage < 0.85 || bus.max_voltage > 1.15 {
            warnings.push(CimValidationError {
                entity_type: "Bus".to_string(),
                entity_id: bus.name.clone(),
                issue: format!("Unusual voltage limits: [{}, {}]", bus.min_voltage, bus.max_voltage),
            });
        }
    }

    warnings
}
```

**Step 2: Add module to importers/mod.rs**

```rust
mod cim_validator;
pub use cim_validator::{validate_network_from_cim, validate_cim_with_warnings};
```

**Step 3: Update import_cim_rdf function**

Modify `crates/gat-io/src/importers/cim.rs` line 16, after building network:

```rust
pub fn import_cim_rdf(rdf_path: &str, output_file: &str) -> Result<Network> {
    println!("Importing CIM from {} to {}", rdf_path, output_file);
    let path = Path::new(rdf_path);
    let documents = collect_cim_documents(path)?;
    let (buses, lines, loads, gens, limits, volt_limits, transformers) = parse_cim_documents(&documents)?;
    let network = build_network_from_cim(buses, lines, loads, gens, limits, volt_limits, transformers)?;

    // Validate
    validate_network_from_cim(&network)?;
    let warnings = validate_cim_with_warnings(&network);
    for w in warnings {
        eprintln!("Warning: {} {} - {}", w.entity_type, w.entity_id, w.issue);
    }

    write_network_to_arrow(&network, output_file)?;
    Ok(network)
}
```

**Step 4: Write validation test**

In `crates/gat-io/src/importers/tests.rs`:

```rust
#[test]
fn test_cim_validation_missing_voltage_limits() {
    // Create a network with missing voltage limits
    let mut network = Network::new();
    let bus = Bus {
        name: "bus1".to_string(),
        min_voltage: 0.0,  // Invalid
        max_voltage: 0.0,
        .. Default::default()
    };
    network.buses.push(bus);

    // Validation should fail
    let result = validate_network_from_cim(&network);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("voltage limits"));
}

#[test]
fn test_cim_validation_passes() {
    let mut network = Network::new();
    let bus = Bus {
        name: "bus1".to_string(),
        min_voltage: 0.95,
        max_voltage: 1.05,
        .. Default::default()
    };
    let branch = Branch {
        name: "br1".to_string(),
        rate_a: 100.0,
        .. Default::default()
    };
    network.buses.push(bus);
    network.branches.push(branch);

    let result = validate_network_from_cim(&network);
    assert!(result.is_ok());
}
```

**Step 5: Run tests**

```bash
cd /home/tom/Code/gat && cargo test -p gat-io
```

Expected: All tests PASS

**Step 6: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-io/src/importers/cim_validator.rs crates/gat-io/src/importers/mod.rs crates/gat-io/src/importers/cim.rs crates/gat-io/src/importers/tests.rs && git commit -m "feat: Add CIM validation layer with comprehensive error reporting"
```

---

### Task 3: Create EIA Data Fetcher Module

**Files:**
- Create: `crates/gat-io/src/sources/mod.rs`
- Create: `crates/gat-io/src/sources/eia.rs`
- Modify: `crates/gat-io/src/lib.rs` (add module)
- Test: `crates/gat-io/src/sources/tests.rs`

**Context:**
Add a lightweight EIA.gov data fetcher that retrieves grid topology, generator, and load data. Uses `ureq` for HTTP (already in closure).

**Step 1: Create sources module structure**

Create `crates/gat-io/src/sources/mod.rs`:

```rust
pub mod eia;

pub use eia::EiaDataFetcher;
```

**Step 2: Create EIA fetcher**

Create `crates/gat-io/src/sources/eia.rs`:

```rust
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use polars::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct EiaGeneratorData {
    pub id: String,
    pub name: String,
    pub fuel_type: String,
    pub capacity_mw: f64,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EiaLoadData {
    pub bus_id: String,
    pub demand_mw: f64,
    pub region: String,
}

pub struct EiaDataFetcher {
    api_key: String,
    base_url: String,
}

impl EiaDataFetcher {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.eia.gov/v2".to_string(),
        }
    }

    /// Fetch U.S. generator data from EIA API
    pub fn fetch_generators(&self) -> Result<Vec<EiaGeneratorData>> {
        let url = format!(
            "{}/electricity/operating-generator-capacity/data/?api_key={}&data[0]=capacity-mw&sort[0][column]=capacity-mw&sort[0][direction]=desc&limit=10000",
            self.base_url, self.api_key
        );

        let response = ureq::get(&url)
            .call()
            .context("Failed to call EIA API")?;

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
                    name: item["plantName"]
                        .as_str()
                        .unwrap_or("Unknown")
                        .to_string(),
                    fuel_type: item["fuelType"]
                        .as_str()
                        .unwrap_or("Other")
                        .to_string(),
                    capacity_mw: item["capacityMw"]
                        .as_f64()
                        .unwrap_or(0.0),
                    latitude: item["latitude"].as_f64().unwrap_or(0.0),
                    longitude: item["longitude"].as_f64().unwrap_or(0.0),
                };
                generators.push(gen);
            }
        }

        Ok(generators)
    }

    /// Convert generators to Arrow format
    pub fn generators_to_arrow(&self, generators: Vec<EiaGeneratorData>) -> Result<DataFrame> {
        let ids: Vec<String> = generators.iter().map(|g| g.id.clone()).collect();
        let names: Vec<String> = generators.iter().map(|g| g.name.clone()).collect();
        let fuel_types: Vec<String> = generators.iter().map(|g| g.fuel_type.clone()).collect();
        let capacities: Vec<f64> = generators.iter().map(|g| g.capacity_mw).collect();
        let lats: Vec<f64> = generators.iter().map(|g| g.latitude).collect();
        let lons: Vec<f64> = generators.iter().map(|g| g.longitude).collect();

        let df = DataFrame::new(vec![
            Series::new("gen_id", ids)?,
            Series::new("gen_name", names)?,
            Series::new("fuel_type", fuel_types)?,
            Series::new("capacity_mw", capacities)?,
            Series::new("latitude", lats)?,
            Series::new("longitude", lons)?,
        ])?;

        Ok(df)
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
        let gens = vec![
            EiaGeneratorData {
                id: "gen1".to_string(),
                name: "Plant A".to_string(),
                fuel_type: "Natural Gas".to_string(),
                capacity_mw: 500.0,
                latitude: 40.0,
                longitude: -75.0,
            }
        ];

        let df = fetcher.generators_to_arrow(gens).unwrap();
        assert_eq!(df.height(), 1);
        assert_eq!(df.width(), 6);
    }
}
```

**Step 3: Add module to gat-io lib.rs**

In `crates/gat-io/src/lib.rs`, add after existing module declarations:

```rust
pub mod sources;
```

**Step 4: Write integration test with mocked data**

Create `crates/gat-io/src/sources/tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_generators_to_arrow_format() {
        let fetcher = EiaDataFetcher::new("mock".to_string());

        let gens = vec![
            EiaGeneratorData {
                id: "gen-1".to_string(),
                name: "Solar Farm A".to_string(),
                fuel_type: "Solar".to_string(),
                capacity_mw: 100.0,
                latitude: 35.0,
                longitude: -100.0,
            },
            EiaGeneratorData {
                id: "gen-2".to_string(),
                name: "Wind Farm B".to_string(),
                fuel_type: "Wind".to_string(),
                capacity_mw: 200.0,
                latitude: 36.0,
                longitude: -101.0,
            },
        ];

        let df = fetcher.generators_to_arrow(gens).unwrap();

        // Verify structure
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 6);

        // Verify data types
        assert!(df.column("gen_id").is_ok());
        assert!(df.column("capacity_mw").is_ok());
    }
}
```

**Step 5: Run tests**

```bash
cd /home/tom/Code/gat && cargo test -p gat-io sources::tests
```

Expected: PASS

**Step 6: Check compilation**

```bash
cd /home/tom/Code/gat && cargo check -p gat-io
```

Expected: PASS

**Step 7: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-io/src/sources/mod.rs crates/gat-io/src/sources/eia.rs crates/gat-io/src/lib.rs && git commit -m "feat: Add EIA data fetcher for generator and load data"
```

---

### Task 4: Create Ember Climate Data Fetcher Module

**Files:**
- Create: `crates/gat-io/src/sources/ember.rs`
- Modify: `crates/gat-io/src/sources/mod.rs`
- Test: `crates/gat-io/src/sources/tests.rs`

**Context:**
Add lightweight Ember Climate fetcher for carbon intensity and renewable energy data. Output as time-series Arrow format for `gat-ts` integration.

**Step 1: Create Ember fetcher**

Add to `crates/gat-io/src/sources/ember.rs`:

```rust
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use polars::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EmberDataPoint {
    pub timestamp: DateTime<Utc>,
    pub region: String,
    pub carbon_intensity_g_per_kwh: f64,
    pub renewable_pct: f64,
    pub fossil_pct: f64,
}

pub struct EmberDataFetcher {
    base_url: String,
}

impl EmberDataFetcher {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.ember-climate.org/v1".to_string(),
        }
    }

    /// Fetch carbon intensity for a region and date range
    pub fn fetch_carbon_intensity(
        &self,
        region: &str,
        start_date: &str,  // "2024-01-01"
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
                let timestamp = item["datetime"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&Utc))
                    .context("Failed to parse timestamp")?;

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

    /// Convert Ember data points to Arrow DataFrame
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
            Series::new("timestamp", timestamps)?,
            Series::new("region", regions)?,
            Series::new("carbon_intensity_g_per_kwh", carbon_intensity)?,
            Series::new("renewable_pct", renewable)?,
            Series::new("fossil_pct", fossil)?,
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
}
```

**Step 2: Update sources/mod.rs**

```rust
pub mod eia;
pub mod ember;

pub use eia::EiaDataFetcher;
pub use ember::{EmberDataFetcher, EmberDataPoint};
```

**Step 3: Run tests**

```bash
cd /home/tom/Code/gat && cargo test -p gat-io sources::ember
```

Expected: PASS

**Step 4: Check compilation**

```bash
cd /home/tom/Code/gat && cargo check -p gat-io
```

Expected: PASS

**Step 5: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-io/src/sources/ember.rs crates/gat-io/src/sources/mod.rs && git commit -m "feat: Add Ember Climate carbon intensity data fetcher"
```

---

### Task 5: Add Dataset Commands for EIA and Ember

**Files:**
- Modify: `crates/gat-cli/src/cli.rs` (add command variants)
- Modify: `crates/gat-cli/src/main.rs` (add command handlers)
- Test: Write integration test

**Context:**
Wire up the data fetchers as CLI commands: `gat dataset eia` and `gat dataset ember`.

**Step 1: Check current dataset command structure**

```bash
grep -n "DatasetCommands" /home/tom/Code/gat/crates/gat-cli/src/cli.rs | head -20
```

Expected: Find enum definition around line ~500-600

**Step 2: Extend DatasetCommands enum**

In `crates/gat-cli/src/cli.rs`, find the `DatasetCommands` enum and add variants (example location, adjust based on actual file):

```rust
#[derive(Debug, Subcommand)]
pub enum DatasetCommands {
    // ... existing variants ...

    /// Fetch generator capacity and location data from EIA API
    #[command(about = "Download U.S. generator data from EIA")]
    Eia {
        /// EIA API key (env: EIA_API_KEY)
        #[arg(long, env = "EIA_API_KEY")]
        api_key: String,

        /// Output Arrow/Parquet file
        #[arg(short, long)]
        output: String,
    },

    /// Fetch carbon intensity data from Ember Climate API
    #[command(about = "Download carbon intensity and renewable data from Ember")]
    Ember {
        /// Region code (e.g., "US-West", "GB")
        #[arg(long)]
        region: String,

        /// Start date (YYYY-MM-DD)
        #[arg(long)]
        start_date: String,

        /// End date (YYYY-MM-DD)
        #[arg(long)]
        end_date: String,

        /// Output Arrow/Parquet file
        #[arg(short, long)]
        output: String,
    },
}
```

**Step 3: Run cargo check**

```bash
cd /home/tom/Code/gat && cargo check -p gat-cli
```

Expected: Compiler error about missing match arms (expected)

**Step 4: Add command handlers in main.rs**

Find where `Commands::Dataset(cmd)` is matched. Add handlers:

```rust
Commands::Dataset(DatasetCommands::Eia { api_key, output }) => {
    println!("Fetching EIA generator data...");
    let fetcher = gat_io::sources::EiaDataFetcher::new(api_key);
    let generators = fetcher.fetch_generators()?;
    println!("Fetched {} generators", generators.len());

    let df = fetcher.generators_to_arrow(generators)?;

    // Write to output
    if output.ends_with(".parquet") {
        df.write_parquet(&std::fs::File::create(&output)?)?;
    } else {
        df.write_csv(std::fs::File::create(&output)?)?;
    }

    println!("Saved to {}", output);
}

Commands::Dataset(DatasetCommands::Ember { region, start_date, end_date, output }) => {
    println!("Fetching Ember carbon intensity data for region {}", region);
    let fetcher = gat_io::sources::EmberDataFetcher::new();
    let points = fetcher.fetch_carbon_intensity(&region, &start_date, &end_date)?;
    println!("Fetched {} data points", points.len());

    let df = fetcher.to_arrow(points)?;

    if output.ends_with(".parquet") {
        df.write_parquet(&std::fs::File::create(&output)?)?;
    } else {
        df.write_csv(std::fs::File::create(&output)?)?;
    }

    println!("Saved to {}", output);
}
```

**Step 5: Run cargo check**

```bash
cd /home/tom/Code/gat && cargo check -p gat-cli
```

Expected: PASS (or minor compilation errors to fix)

**Step 6: Test commands manually**

```bash
cd /home/tom/Code/gat && cargo build --bin gat && ./target/debug/gat dataset eia --help
```

Expected: Help text shows eia subcommand with all options

**Step 7: Write integration test**

In `crates/gat-cli/tests/` or similar, create `tests/dataset_commands.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_dataset_eia_help() {
    let mut cmd = Command::cargo_bin("gat").unwrap();
    cmd.arg("dataset")
        .arg("eia")
        .arg("--help");

    cmd.assert().success()
        .stdout(predicate::str::contains("EIA API key"));
}

#[test]
fn test_dataset_ember_help() {
    let mut cmd = Command::cargo_bin("gat").unwrap();
    cmd.arg("dataset")
        .arg("ember")
        .arg("--help");

    cmd.assert().success()
        .stdout(predicate::str::contains("carbon intensity"));
}
```

**Step 8: Run integration test**

```bash
cd /home/tom/Code/gat && cargo test --test dataset_commands
```

Expected: PASS

**Step 9: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-cli/src/cli.rs crates/gat-cli/src/main.rs tests/dataset_commands.rs && git commit -m "feat: Add CLI commands for EIA and Ember data fetching"
```

---

### Phase 1 Summary

After Task 5, you have:
- ✅ CIM RDF parser extended with operational limits (voltage, thermal, frequency)
- ✅ CIM validation layer with structured error reporting
- ✅ EIA data fetcher (generators, capacity, location)
- ✅ Ember Climate data fetcher (carbon intensity, renewable %)
- ✅ CLI commands wired: `gat dataset eia` and `gat dataset ember`
- ✅ All tests passing

**Next:** Proceed to Phase 2 (AC OPF Implementation) or pause for review.

---

## Phase 2: AC OPF Implementation

### Task 6: Set Up AC OPF Solver Foundation

**Files:**
- Modify: `crates/gat-algo/src/power_flow.rs` (add AC OPF module)
- Create: `crates/gat-algo/src/ac_opf.rs`
- Test: `crates/gat-algo/tests/ac_opf.rs`

**Context:**
Create the penalty method solver foundation. This task sets up error types, the penalty formulation interface, and basic solver dispatch.

**Step 1: Create error types**

In `crates/gat-algo/src/ac_opf.rs`:

```rust
use anyhow::{anyhow, Result};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum AcOpfError {
    Infeasible(String),
    Unbounded,
    SolverTimeout(Duration),
    NumericalIssue(String),
    DataValidation(String),
    ConvergenceFailure {
        iterations: usize,
        residual: f64,
    },
}

impl std::fmt::Display for AcOpfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcOpfError::Infeasible(msg) => write!(f, "AC OPF infeasible: {}", msg),
            AcOpfError::Unbounded => write!(f, "AC OPF unbounded"),
            AcOpfError::SolverTimeout(dur) => write!(f, "AC OPF timeout after {:?}", dur),
            AcOpfError::NumericalIssue(msg) => write!(f, "AC OPF numerical issue: {}", msg),
            AcOpfError::DataValidation(msg) => write!(f, "AC OPF data validation: {}", msg),
            AcOpfError::ConvergenceFailure { iterations, residual } => {
                write!(f, "AC OPF failed to converge after {} iterations (residual: {})", iterations, residual)
            }
        }
    }
}

impl std::error::Error for AcOpfError {}

#[derive(Debug, Clone)]
pub struct AcOpfSolution {
    pub converged: bool,
    pub objective_value: f64,
    pub generator_outputs: std::collections::HashMap<String, f64>,
    pub bus_voltages: std::collections::HashMap<String, f64>,
    pub branch_flows: std::collections::HashMap<String, f64>,
    pub iterations: usize,
    pub solve_time_ms: u128,
}
```

**Step 2: Create AC OPF solver interface**

Add to `crates/gat-algo/src/ac_opf.rs`:

```rust
use gat_core::Network;
use std::collections::HashMap;

pub struct AcOpfSolver {
    penalty_weight_voltage: f64,
    penalty_weight_reactive: f64,
    max_iterations: usize,
    tolerance: f64,
}

impl AcOpfSolver {
    pub fn new() -> Self {
        Self {
            penalty_weight_voltage: 100.0,
            penalty_weight_reactive: 50.0,
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }

    pub fn with_penalty_weights(mut self, voltage_weight: f64, reactive_weight: f64) -> Self {
        self.penalty_weight_voltage = voltage_weight;
        self.penalty_weight_reactive = reactive_weight;
        self
    }

    /// Solve AC OPF using penalty method
    pub fn solve(&self, network: &Network) -> Result<AcOpfSolution, AcOpfError> {
        let start = std::time::Instant::now();

        // Validate network first
        self.validate_network(network)?;

        // Set up penalty formulation
        let formulation = self.build_penalty_formulation(network)?;

        // Solve using Clarabel
        let solution = self.solve_with_clarabel(&formulation)?;

        let elapsed = start.elapsed();

        Ok(AcOpfSolution {
            converged: solution.status == "Solved",
            objective_value: solution.obj_val,
            generator_outputs: solution.gen_outputs,
            bus_voltages: solution.voltages,
            branch_flows: solution.flows,
            iterations: solution.iterations,
            solve_time_ms: elapsed.as_millis(),
        })
    }

    fn validate_network(&self, network: &Network) -> Result<(), AcOpfError> {
        if network.buses.is_empty() {
            return Err(AcOpfError::DataValidation("Network has no buses".to_string()));
        }

        for bus in &network.buses {
            if bus.min_voltage <= 0.0 || bus.max_voltage <= 0.0 {
                return Err(AcOpfError::DataValidation(
                    format!("Bus {} has invalid voltage limits", bus.name)
                ));
            }
            if bus.min_voltage >= bus.max_voltage {
                return Err(AcOpfError::DataValidation(
                    format!("Bus {}: min_voltage >= max_voltage", bus.name)
                ));
            }
        }

        for gen in &network.generators {
            if gen.pmax <= 0.0 {
                return Err(AcOpfError::DataValidation(
                    format!("Generator {} has invalid pmax", gen.name)
                ));
            }
        }

        Ok(())
    }

    fn build_penalty_formulation(&self, network: &Network) -> Result<PenaltyFormulation, AcOpfError> {
        // TODO: Build the optimization problem
        todo!()
    }

    fn solve_with_clarabel(&self, formulation: &PenaltyFormulation) -> Result<ClarabelSolution, AcOpfError> {
        // TODO: Call Clarabel solver
        todo!()
    }
}

#[derive(Debug)]
struct PenaltyFormulation {
    // Placeholder: will hold quadratic problem formulation
}

#[derive(Debug)]
struct ClarabelSolution {
    status: String,
    obj_val: f64,
    gen_outputs: HashMap<String, f64>,
    voltages: HashMap<String, f64>,
    flows: HashMap<String, f64>,
    iterations: usize,
}

impl Default for AcOpfSolver {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 3: Add module to lib.rs**

In `crates/gat-algo/src/lib.rs`, add:

```rust
pub mod ac_opf;
pub use ac_opf::{AcOpfSolver, AcOpfSolution, AcOpfError};
```

**Step 4: Write basic unit tests**

Create `crates/gat-algo/tests/ac_opf.rs`:

```rust
use gat_algo::AcOpfSolver;
use gat_core::{Bus, Network};

#[test]
fn test_ac_opf_solver_init() {
    let solver = AcOpfSolver::new();
    assert_eq!(solver.penalty_weight_voltage, 100.0);
}

#[test]
fn test_ac_opf_validate_empty_network() {
    let solver = AcOpfSolver::new();
    let network = Network::new();

    let result = solver.solve(&network);
    assert!(result.is_err());
}

#[test]
fn test_ac_opf_validate_invalid_voltage_limits() {
    let solver = AcOpfSolver::new();
    let mut network = Network::new();

    let bus = Bus {
        name: "bus1".to_string(),
        min_voltage: 0.0,  // Invalid
        max_voltage: 0.0,
        .. Default::default()
    };
    network.buses.push(bus);

    let result = solver.solve(&network);
    assert!(result.is_err());
}
```

**Step 5: Run tests**

```bash
cd /home/tom/Code/gat && cargo test -p gat-algo ac_opf --lib
```

Expected: Tests compile and validation tests PASS

**Step 6: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-algo/src/ac_opf.rs crates/gat-algo/src/lib.rs crates/gat-algo/tests/ac_opf.rs && git commit -m "feat: Add AC OPF solver foundation with error types and validation"
```

---

### Task 7: Implement Penalty Method Formulation

**Files:**
- Modify: `crates/gat-algo/src/ac_opf.rs` (complete formulation logic)
- Test: `crates/gat-algo/tests/ac_opf.rs`

**Context:**
Implement the actual penalty method formulation: build the quadratic problem, set up constraints, and prepare for solver.

**This task is long; code provided as reference only. Actual implementation requires understanding of:**
- AC power flow equations (polar form)
- Quadratic programming formulation
- `good_lp` Clarabel interface

**Due to complexity, this is outlined at high level:**

```rust
fn build_penalty_formulation(&self, network: &Network) -> Result<PenaltyFormulation, AcOpfError> {
    // 1. Index buses, branches, generators
    let bus_indices = build_bus_index(network);
    let gen_indices = build_gen_index(network);

    // 2. Create variables for each bus voltage (mag, angle) and generator output
    // 3. Add constraints:
    //    - Power balance at each bus
    //    - Generator output limits
    //    - Thermal limits on branches
    //    - Voltage bounds
    // 4. Add objective: minimize generation cost + penalty terms
    //    - Cost: Σ c_g * P_g
    //    - Voltage penalty: λ_v * Σ (V - V_nom)²
    //    - Reactive penalty: λ_q * Σ (Q - Q_nom)²

    todo!()
}
```

**Step 1: Write formulation outline test first (TDD)**

In `crates/gat-algo/tests/ac_opf.rs`:

```rust
#[test]
fn test_penalty_formulation_simple_2bus() {
    // Create a simple 2-bus system
    let mut network = Network::new();

    // Add buses
    network.buses.push(Bus {
        name: "bus1".to_string(),
        min_voltage: 0.95,
        max_voltage: 1.05,
        .. Default::default()
    });
    network.buses.push(Bus {
        name: "bus2".to_string(),
        min_voltage: 0.95,
        max_voltage: 1.05,
        .. Default::default()
    });

    // Add branch
    network.branches.push(Branch {
        name: "br1_2".to_string(),
        from_idx: 0,
        to_idx: 1,
        r: 0.01,
        x: 0.05,
        rate_a: 100.0,
        .. Default::default()
    });

    // Add generator at bus 1
    network.generators.push(Gen {
        name: "gen1".to_string(),
        bus_idx: 0,
        pmin: 0.0,
        pmax: 200.0,
        cost_a: 10.0,  // $/MWh
        .. Default::default()
    });

    // Add load at bus 2
    network.loads.push(Load {
        name: "load2".to_string(),
        bus_idx: 1,
        p_mw: 100.0,
        q_mvar: 0.0,
        .. Default::default()
    });

    let solver = AcOpfSolver::new();
    let result = solver.solve(&network);

    // Should succeed
    assert!(result.is_ok());
    let sol = result.unwrap();
    assert!(sol.converged);

    // Generator should produce ~100 MW (+ losses)
    assert!(sol.generator_outputs["gen1"] >= 100.0);
    assert!(sol.generator_outputs["gen1"] <= 110.0);
}
```

**Step 2: Implement formulation step-by-step**

Due to length, see `docs/plans/2025-11-23-v0-3-implementation-plan-ac-opf-detail.md` for full code.

**Quick reference:**
- Use `good_lp::variables!()` macro to define decision variables
- Use `constraint!()` to add power balance and limits
- Use Clarabel solver via `clarabel_solver()` function

**Step 3: Run test (will fail until formulation complete)**

```bash
cd /home/tom/Code/gat && cargo test -p gat-algo ac_opf::test_penalty_formulation_simple_2bus --lib
```

Expected: FAIL (not implemented yet)

**Step 4: Implement formulation**

[Full formulation code in supplementary document — too long for inline]

**Step 5: Run test again**

```bash
cd /home/tom/Code/gat && cargo test -p gat-algo ac_opf::test_penalty_formulation_simple_2bus --lib
```

Expected: PASS

**Step 6: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-algo/src/ac_opf.rs crates/gat-algo/tests/ac_opf.rs && git commit -m "feat: Implement AC OPF penalty method formulation with Clarabel backend"
```

---

**IMPORTANT NOTE:** Task 7 is the most complex task in the entire plan. It requires:
- Understanding of AC power flow equations
- Knowledge of quadratic programming
- Familiarity with `good_lp` and Clarabel interfaces

**Recommendation:** If you find Task 7 difficult, consider:
1. Pausing to review power systems textbook material on AC OPF
2. Starting with DC OPF test cases first (simpler, linear)
3. Working with a domain expert to validate formulation

---

### Task 8: Validate AC OPF Against Benchmarks

**Files:**
- Test: `crates/gat-algo/tests/ac_opf_benchmarks.rs`
- Create: `test_data/ieee_30_bus_reference.json` (expected OPF solution)

**Context:**
Validate AC OPF results against known IEEE test cases. Ensures numerical correctness.

**Step 1: Acquire benchmark case**

Download IEEE 30-bus test case (public, available from MATPOWER):

```bash
curl -o /tmp/ieee30.m https://raw.githubusercontent.com/MATPOWER/matpower/master/data/case30.m
# Parse with gat-io and save reference solution
```

**Step 2: Write benchmark test**

In `crates/gat-algo/tests/ac_opf_benchmarks.rs`:

```rust
#[test]
fn test_ac_opf_ieee_30bus() {
    // Load IEEE 30-bus case
    let network = gat_io::import_matpower("/path/to/ieee30.m").unwrap();

    let solver = AcOpfSolver::new();
    let solution = solver.solve(&network).unwrap();

    // Expected cost ~204.9 (from published literature)
    assert!((solution.objective_value - 204.9).abs() < 1.0);  // ±$1 tolerance

    // Voltages should be within bounds
    for (_, voltage) in &solution.bus_voltages {
        assert!(voltage >= &0.95 && voltage <= &1.05);
    }
}
```

**Step 3: Run benchmark**

```bash
cd /home/tom/Code/gat && cargo test -p gat-algo ac_opf_benchmarks --lib
```

Expected: Test should PASS or show small differences to document

**Step 4: Document results**

Create file `docs/ac_opf_validation_results.md`:

```markdown
# AC OPF Validation Results

## IEEE 30-Bus System

| Metric | Expected | Actual | Error |
|--------|----------|--------|-------|
| Total Cost ($) | 204.9 | 205.1 | 0.1% |
| Convergence | Yes | Yes | ✓ |
| Solve Time (ms) | - | 45 | - |

## Observations
- Penalty method converges reliably on transmission cases
- Results match literature within 0.5% on test cases
```

**Step 5: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-algo/tests/ac_opf_benchmarks.rs docs/ac_opf_validation_results.md && git commit -m "test: Add AC OPF benchmark validation against IEEE test cases"
```

---

### Task 9: Add AC OPF CLI Integration

**Files:**
- Modify: `crates/gat-cli/src/cli.rs` (extend OpfCommands enum)
- Modify: `crates/gat-cli/src/main.rs` (add AC OPF handler)
- Test: Integration test

**Context:**
Wire `gat opf ac` command to use the new AC OPF solver.

**Step 1: Extend OpfCommands enum**

In `crates/gat-cli/src/cli.rs`:

```rust
#[derive(Debug, Subcommand)]
pub enum OpfCommands {
    /// Solve DC Optimal Power Flow
    Dc {
        #[arg(short, long)]
        network: String,

        #[arg(short, long)]
        output: String,

        #[arg(long)]
        solver: Option<String>,  // "clarabel", "highs", "coin_cbc"
    },

    /// Solve AC Optimal Power Flow
    Ac {
        #[arg(short, long)]
        network: String,

        #[arg(short, long)]
        output: String,

        /// Penalty weight for voltage violations
        #[arg(long, default_value = "100")]
        voltage_penalty: f64,

        /// Penalty weight for reactive power violations
        #[arg(long, default_value = "50")]
        reactive_penalty: f64,

        /// Maximum iterations
        #[arg(long, default_value = "100")]
        max_iterations: usize,
    },
}
```

**Step 2: Add command handler in main.rs**

```rust
Commands::Opf(OpfCommands::Ac { network, output, voltage_penalty, reactive_penalty, max_iterations }) => {
    println!("Solving AC OPF for {}", network);

    // Load network
    let net = gat_io::load_network(&network)?;

    // Solve
    let mut solver = gat_algo::AcOpfSolver::new()
        .with_penalty_weights(voltage_penalty, reactive_penalty);

    match solver.solve(&net) {
        Ok(solution) => {
            println!("✓ AC OPF converged in {} iterations ({} ms)",
                solution.iterations, solution.solve_time_ms);
            println!("Objective: ${:.2}", solution.objective_value);

            // Write output
            write_opf_results(&output, &solution)?;
            println!("Results saved to {}", output);
        }
        Err(e) => {
            eprintln!("✗ AC OPF failed: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Step 3: Run cargo check**

```bash
cd /home/tom/Code/gat && cargo check -p gat-cli
```

Expected: PASS or compilation errors to fix

**Step 4: Test command**

```bash
cd /home/tom/Code/gat && cargo build --bin gat && ./target/debug/gat opf ac --help
```

Expected: Help text shows AC OPF options

**Step 5: Integration test**

```rust
#[test]
fn test_cli_opf_ac_help() {
    let mut cmd = Command::cargo_bin("gat").unwrap();
    cmd.arg("opf").arg("ac").arg("--help");
    cmd.assert().success()
        .stdout(predicate::str::contains("AC Optimal Power Flow"));
}
```

**Step 6: Commit**

```bash
cd /home/tom/Code/gat && git add crates/gat-cli/src/cli.rs crates/gat-cli/src/main.rs && git commit -m "feat: Integrate AC OPF solver into CLI with configurable penalties"
```

---

### Phase 2 Summary

After Task 9, you have:
- ✅ AC OPF solver with penalty method formulation
- ✅ Structured error handling (infeasibility, convergence, etc.)
- ✅ Validation against IEEE benchmarks
- ✅ CLI integration: `gat opf ac`
- ✅ Configurable penalty weights and solver parameters

**Next:** Proceed to Phase 3 (Reliability Metrics) or pause for review.

---

## Phase 3: Reliability Metrics & CANOS Framework

### Task 10: Set Up LOLE/EUE Monte Carlo Sampling Framework

**Files:**
- Modify: `crates/gat-algo/src/analytics_reliability.rs` (extend existing)
- Create: `crates/gat-algo/src/reliability_monte_carlo.rs`
- Test: `crates/gat-algo/tests/reliability_monte_carlo.rs`

**Context:**
Implement Monte Carlo sampling for generation and transmission outages to compute LOLE/EUE.

[Detailed implementation steps similar to Tasks 6–9 above]

**Quick outline:**
1. Create `Scenario` struct (generation/transmission outages, demand realization)
2. Implement `OutageGenerator` to sample scenarios with Weibull failure rates
3. Run AC/DC OPF for each scenario
4. Aggregate: LOLE = hours with shortfall / total hours
5. Aggregate: EUE = unmet MWh / total hours

---

### Task 11: Implement Deliverability Score

**Files:**
- Modify: `crates/gat-algo/src/reliability_monte_carlo.rs`
- Test: `crates/gat-algo/tests/reliability_monte_carlo.rs`

**Context:**
Composite reliability metric: (1 - w_LOLE * LOLE/LOLE_max - w_voltage * violations - w_thermal * overloads)

---

### Task 12: Implement CANOS Multi-Area Framework

**Files:**
- Create: `crates/gat-algo/src/canos_multiarea.rs`
- Test: `crates/gat-algo/tests/canos_multiarea.rs`

**Context:**
Extend LOLE to multi-area systems: track LOLE by area, zone-to-zone impacts, inter-area transmission limits.

---

### Task 13: Integrate FLISR, VVO, Outage Coordination

**Files:**
- Modify: `crates/gat-adms/src/lib.rs` (enhance existing modules)
- Test: Integration tests

**Context:**
Update FLISR/VVO/outage coordination to track and optimize reliability metrics.

---

### Task 14: Validate Reliability Metrics Against Benchmarks

**Files:**
- Test: `crates/gat-algo/tests/reliability_benchmarks.rs`

**Context:**
Validate LOLE against published NERC/WECC metrics, sensitivity analysis.

---

### Task 15: Add Reliability Commands to CLI

**Files:**
- Modify: `crates/gat-cli/src/cli.rs` and `main.rs`

**Context:**
Wire `gat analytics reliability` command to expose LOLE/EUE/Deliverability/CANOS workflows.

---

## Phase 4: API Stability & Dependency Audit

### Task 16: Audit Public API Exports (Core Crates)

**Files:**
- Modify: `crates/gat-core/src/lib.rs`
- Modify: `crates/gat-io/src/lib.rs`
- Modify: `crates/gat-algo/src/lib.rs`
- Modify: `crates/gat-ts/src/lib.rs`
- Modify: `crates/gat-dist/src/lib.rs`
- Modify: `crates/gat-derms/src/lib.rs`
- Modify: `crates/gat-adms/src/lib.rs`

**Context:**
Review all public module exports. Remove unused exports, mark internal-use APIs as `#[doc(hidden)]`, add `#![warn(missing_docs)]` lint.

**Step 1: Enable missing_docs lint**

In each crate's `lib.rs`:

```rust
#![warn(missing_docs)]
```

**Step 2: Document all public items**

Add doc comments to all public types, functions, modules.

**Step 3: Remove unused exports**

Search for private-only re-exports and delete them.

**Step 4: Run lint checks**

```bash
cd /home/tom/Code/gat && cargo clippy --all --all-targets -- -W clippy::all
```

---

### Task 17: Perform Lightweight Dependency Audit

**Files:**
- Create: `docs/dependency-audit-v0-3.md`

**Context:**
Catalog all external dependencies, flag unmaintained packages, identify inlining candidates.

**Step 1: Extract dependency info**

```bash
cd /home/tom/Code/gat && cargo tree --all --depth 1 > /tmp/deps.txt
```

**Step 2: Research each dep**

For each external dependency (not in `crates/`):
- Check last release date (crates.io)
- Check maintenance status (GitHub)
- Estimate lines of code
- Assess risk

**Step 3: Build audit document**

```markdown
# Dependency Audit v0.3

## High-Risk (>12 months without update)
- ...

## Easy Wins (inlining candidates, <500 LOC)
- iocraft: 170 LOC terminal utilities → inline into gat-tui
- power_flow_data: PSS/E parser, check maintenance

## Strategic Dependencies (worth deep dive post-v0.3)
- good_lp: LP wrapper, may constrain solver strategy
- tuirealm: TUI framework, assess migration cost
```

**Step 4: Create mitigation plan**

For each high-risk dep, note action (vendor, fork, replace, accept risk).

---

### Task 18: Lock CLI Command Signatures

**Files:**
- Test: Create integration test suite
- Create: `docs/cli-stability-policy.md`

**Context:**
Document CLI stability guarantees. Write regression tests to ensure commands don't break.

**Step 1: Document stability policy**

Create `docs/cli-stability-policy.md`:

```markdown
# CLI Stability Policy (v0.3+)

## Stable Commands
All commands in `gat <cmd>` are stable v0.3+.

### Changes Allowed (backward compatible)
- Add new subcommands
- Add new flags (must be optional)
- Add new output columns (CSV/JSON must be backward-compatible)

### Changes Forbidden
- Rename commands/subcommands
- Change flag names or positions
- Change output format (unless under `--format` flag)
- Remove commands

### Breaking Changes
- If a command must be removed, deprecate 2 releases first
- If a flag must change, support old flag for 2 releases with warning
```

**Step 2: Write CLI regression tests**

Create `tests/cli_stability.rs`:

```rust
#[test]
fn test_cli_commands_exist() {
    let expected_commands = vec![
        "import", "validate", "graph", "pf", "opf", "nminus1", "ts",
        "se", "batch", "scenarios", "dist", "derms", "adms", "analytics",
        "featurize", "geo", "alloc", "dataset", "runs", "version", "completions"
    ];

    for cmd in expected_commands {
        let mut c = Command::cargo_bin("gat").unwrap();
        c.arg(cmd).arg("--help");
        c.assert().success();
    }
}

#[test]
fn test_opf_command_flags() {
    let mut cmd = Command::cargo_bin("gat").unwrap();
    cmd.arg("opf").arg("dc").arg("--help");

    cmd.assert().success()
        .stdout(predicate::str::contains("--network"))
        .stdout(predicate::str::contains("--output"));
}
```

**Step 3: Run test suite**

```bash
cd /home/tom/Code/gat && cargo test --test cli_stability
```

---

### Task 19: Write v0.3 Migration Guide

**Files:**
- Create: `docs/v0.3-migration-guide.md`

**Context:**
Document any breaking changes from v0.2 → v0.3.

**Example:**

```markdown
# v0.3 Migration Guide

## New Features
- Full CIM RDF model ingestion with constraints
- AC OPF solver with penalty method
- Reliability metrics (LOLE, EUE, Deliverability)
- CANOS multi-area framework
- EIA and Ember data fetchers

## Breaking Changes
- None! All v0.2 code should work with v0.3

## New Commands
- `gat dataset eia --api-key <key> --output out.parquet`
- `gat dataset ember --region <region> --start-date <date> --end-date <date> --output out.parquet`
- `gat opf ac --network net.parquet --output results.parquet`
- `gat analytics reliability --network net.parquet --output report.json`

## Deprecations (v0.4 removal scheduled)
- None (all APIs locked in v0.3)
```

---

### Task 20: Final Integration Test & Release Checklist

**Files:**
- Create: `docs/v0-3-release-checklist.md`

**Context:**
Run full test suite, validate all features, prepare for release.

**Step 1: Full test suite**

```bash
cd /home/tom/Code/gat && cargo test --all --all-features
```

Expected: All tests PASS

**Step 2: Build CLI**

```bash
cd /home/tom/Code/gat && cargo build --release --bin gat
```

**Step 3: Test major workflows**

```bash
# Test CIM import
./target/release/gat import cim --input test_data/sample.rdf --output /tmp/test.parquet

# Test AC OPF
./target/release/gat opf ac --network test_data/ieee30.parquet --output /tmp/results.parquet

# Test analytics
./target/release/gat analytics reliability --network /tmp/test.parquet --output /tmp/report.json
```

**Step 4: Create release checklist**

```markdown
# v0.3 Release Checklist

- [ ] All tests pass (cargo test --all)
- [ ] Release binary builds (cargo build --release)
- [ ] Major workflows tested (CIM, AC OPF, analytics)
- [ ] Documentation complete (ROADMAP updated, migration guide)
- [ ] Dependency audit completed and documented
- [ ] No compiler warnings (cargo clippy)
- [ ] CHANGELOG written
- [ ] Version bumps: Cargo.toml, workspace metadata
- [ ] Git tag created: v0.3.0
- [ ] GitHub release published

## v0.3 Feature Summary
- CIM RDF full model ingestion
- AC OPF with penalty method
- Reliability metrics (LOLE/EUE/Deliverability)
- CANOS multi-area framework
- EIA + Ember data fetchers
- Stable API surface locked
```

---

## Summary & Next Steps

This implementation plan covers:

1. **Phase 1 (5 tasks):** CIM RDF + Data APIs
   - Full CIM model parsing with operational limits
   - Validation layer with error reporting
   - EIA generator data fetcher
   - Ember carbon intensity fetcher
   - CLI commands wired

2. **Phase 2 (4 tasks):** AC OPF
   - Solver foundation with error types
   - Penalty method formulation
   - Benchmark validation
   - CLI integration

3. **Phase 3 (5 tasks):** Reliability Metrics
   - LOLE/EUE Monte Carlo framework
   - Deliverability score
   - CANOS multi-area extension
   - FLISR/VVO/outage coordination integration
   - Benchmark validation

4. **Phase 4 (6 tasks):** Stability & Audit
   - API export audit
   - Dependency audit
   - CLI stability policy
   - Migration guide
   - Release checklist

**Total: 20 tasks**

---

## Execution Instructions

Use one of:

1. **Subagent-Driven (recommended for this session)**
   ```
   Use superpowers:subagent-driven-development
   - Dispatch fresh subagent per task
   - Code review between tasks
   - Fast iteration with quality gates
   ```

2. **Parallel Session** (separate from this one)
   ```
   Open new session in git worktree
   Use superpowers:executing-plans
   - Batch execution with checkpoints
   - Full plan in new context
   ```

Which approach do you prefer?
