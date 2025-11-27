+++
title = "Geo-Spatial Analysis"
description = "GIS integration and spatial feature aggregation for power systems"
weight = 61
+++

# Geo-Spatial Analysis

GAT's `gat geo` commands enable GIS integration for spatial analysis, connecting power grid topology to geographic polygons like census tracts, zip codes, and planning areas.

## Overview

| Command | Purpose | Output |
|---------|---------|--------|
| `geo join` | Map buses to spatial polygons | Bus-to-polygon mapping |
| `geo featurize` | Aggregate time-series by polygon | Spatial feature tables |

## Spatial Joins

### `gat geo join`

Maps buses/feeders to GIS polygons using spatial join methods.

```bash
gat geo join \
  --grid-file grid.arrow \
  --polygons census_tracts.geojson \
  --out bus_mapping.parquet
```

**Required Arguments:**
- `--grid-file` — Grid topology (Arrow format with `bus_id`, `lat`, `lon`)
- `--polygons` — GIS file (GeoParquet, Shapefile, or GeoJSON)
- `--out` — Output mapping table

**Options:**
- `--method` — Spatial join method:
  - `point_in_polygon` (default) — Direct containment test
  - `voronoi` — Voronoi tessellation assignment
  - `knn` — K-nearest-neighbor to polygon centroids
- `--k` — For KNN: number of nearest polygons (default: 1)
- `--out-partitions` — Partition output by columns

**Output Schema:**
```
bus_id    | polygon_id | distance
----------|------------|----------
1         | tract_001  | 0.0
2         | tract_001  | 0.0
3         | tract_002  | 0.0
```

### Example: Census Tract Mapping

```bash
# Download census tract boundaries
# (from census.gov or your GIS data source)

# Join buses to tracts
gat geo join \
  --grid-file feeder.arrow \
  --polygons tracts.geojson \
  --method point_in_polygon \
  --out bus_to_tract.parquet
```

**Reference:** [Spatial Joins in Energy GIS](https://doi.org/10.3390/ijgi9020102)

## Spatial Feature Aggregation

### `gat geo featurize`

Aggregates time-series grid metrics to spatial polygons for forecasting models.

```bash
gat geo featurize \
  --mapping bus_to_tract.parquet \
  --timeseries hourly_loads.parquet \
  --out polygon_features.parquet \
  --lags 1,24,168 \
  --windows 24,168 \
  --seasonal
```

**Required Arguments:**
- `--mapping` — Bus-to-polygon mapping (from `geo join`)
- `--timeseries` — Time-series metrics (Parquet with `bus_id`, `time`, values)
- `--out` — Output feature table

**Options:**
- `--lags` — Lag periods to compute (comma-separated)
- `--windows` — Rolling window sizes (comma-separated)
- `--seasonal` — Add day-of-week, hour-of-day, month-of-year flags
- `--out-partitions` — Partition output

**Output Features:**
- Aggregated load/generation by polygon
- Lag features (1h, 24h, 168h back)
- Rolling statistics (mean, max, min over windows)
- Seasonal indicators (if `--seasonal`)

### Example: Demand Forecasting Pipeline

```bash
# 1. Create bus-to-polygon mapping
gat geo join \
  --grid-file grid.arrow \
  --polygons planning_areas.geojson \
  --out mapping.parquet

# 2. Aggregate load time-series
gat geo featurize \
  --mapping mapping.parquet \
  --timeseries loads_2024.parquet \
  --out features.parquet \
  --lags 1,7,24,168 \
  --windows 24,168,720 \
  --seasonal

# 3. Train forecasting model (Python)
import polars as pl
features = pl.read_parquet("features.parquet")
# ... train spatial demand forecasting model
```

**Reference:** [Spatial-Temporal Load Forecasting](https://doi.org/10.1016/j.energy.2020.117515)

## Data Requirements

### Grid File

The grid topology must include coordinates:

```
bus_id | lat      | lon
-------|----------|----------
1      | 40.7128  | -74.0060
2      | 40.7580  | -73.9855
```

### Polygon File

Supported formats:
- **GeoParquet** — Recommended for large datasets
- **GeoJSON** — Human-readable, web-compatible
- **Shapefile** — Legacy GIS format

Polygons must have:
- Geometry column with polygon/multipolygon shapes
- Unique identifier column (auto-detected or specify)

## Use Cases

### Utility Planning
- Aggregate load forecasts to service territories
- Map reliability metrics to planning areas
- Identify high-growth zones for capacity planning

### Regulatory Reporting
- Aggregate reliability indices by geographic region
- Map outages to census tracts for equity analysis
- Generate reports by administrative boundaries

### Spatial ML Models
- Build geo-aware demand forecasting models
- Incorporate spatial features in reliability prediction
- Enable transfer learning across regions

## Related Commands

- [ML Features](/guide/ml-features/) — Feature extraction for ML
- [Reliability](/guide/reliability/) — Reliability metrics
- [Time Series](/guide/ts/) — Time-series analysis
