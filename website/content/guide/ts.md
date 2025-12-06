+++
title = "Time Series"
description = "Time series processing for telemetry, load profiles, and multi-period analysis"
weight = 12
+++

# Time Series

The `gat ts` commands process time-indexed data: SCADA telemetry, load profiles, generation schedules, and renewable forecasts. Operations include resampling, joining, aggregation, and preparation for multi-period dispatch.

## Overview

| Command | Purpose |
|---------|---------|
| `gat ts resample` | Resample to fixed intervals |
| `gat ts join` | Join multiple time series |
| `gat ts agg` | Aggregate by groups |
| `gat ts fill` | Fill missing values |
| `gat ts validate` | Check data quality |
| `gat ts slice` | Extract time windows |

## Quick Start

### Resample Telemetry Data

```bash
# Resample irregular SCADA data to 5-minute intervals
gat ts resample scada.parquet \
  --rule 5min \
  --timestamp timestamp \
  --value power_mw \
  --out scada_5min.parquet
```

### Join Multiple Sources

```bash
# Align load and solar data on timestamp
gat ts join \
  load_data.parquet \
  solar_data.parquet \
  --on timestamp \
  --out combined.parquet
```

### Aggregate by Category

```bash
# Sum load by region
gat ts agg load.parquet \
  --group region \
  --value load_mw \
  --agg sum \
  --out regional_load.parquet
```

## Time Series File Format

### Expected Schema

GAT time series commands expect:

| Column | Type | Description |
|--------|------|-------------|
| `timestamp` | datetime | ISO 8601 timestamp (configurable name) |
| `value` | float | Measurement value (configurable name) |
| (optional) | any | Additional columns preserved |

### Example Data

```csv
timestamp,sensor_id,value,unit
2024-01-15T00:00:00Z,LOAD_001,125.3,MW
2024-01-15T00:05:00Z,LOAD_001,127.1,MW
2024-01-15T00:10:00Z,LOAD_001,126.8,MW
2024-01-15T00:00:00Z,GEN_001,250.0,MW
2024-01-15T00:05:00Z,GEN_001,248.5,MW
```

### Supported Formats

- **Parquet** (recommended) — Fast columnar storage with type preservation
- **CSV** — Human-readable, auto-parsed timestamps
- **Arrow IPC** — In-memory transfer format

## Resample

### `gat ts resample`

Convert irregular timestamps to fixed intervals:

```bash
gat ts resample input.parquet \
  --rule 1h \
  --timestamp timestamp \
  --value power_mw \
  --out output.parquet
```

### Resampling Rules

| Rule | Description | Example |
|------|-------------|---------|
| `1s` | 1 second | Real-time telemetry |
| `1min` | 1 minute | SCADA polling |
| `5min` | 5 minutes | Standard intervals |
| `15min` | 15 minutes | Market intervals |
| `1h` | 1 hour | Hourly dispatch |
| `1d` | 1 day | Daily summaries |

### Aggregation Methods

```bash
gat ts resample input.parquet \
  --rule 1h \
  --agg mean \          # Default: average
  --timestamp timestamp \
  --value power_mw \
  --out hourly.parquet
```

| Method | Description |
|--------|-------------|
| `mean` | Average value in bucket (default) |
| `sum` | Sum of values |
| `min` | Minimum value |
| `max` | Maximum value |
| `first` | First value in bucket |
| `last` | Last value in bucket |
| `count` | Number of values |

### Output Schema

Resampling produces:

| Column | Description |
|--------|-------------|
| `bucket_start` | Start of time bucket |
| `bucket_end` | End of time bucket |
| `count` | Number of raw values in bucket |
| `mean_value` | Average (if agg=mean) |
| `min_value` | Minimum value |
| `max_value` | Maximum value |

### Examples

**15-minute load profile:**
```bash
gat ts resample scada_load.parquet \
  --rule 15min \
  --agg mean \
  --timestamp recorded_at \
  --value load_mw \
  --out load_15min.parquet
```

**Daily peak extraction:**
```bash
gat ts resample scada_load.parquet \
  --rule 1d \
  --agg max \
  --timestamp recorded_at \
  --value load_mw \
  --out daily_peaks.parquet
```

**Hourly energy totals:**
```bash
gat ts resample generation.parquet \
  --rule 1h \
  --agg sum \
  --timestamp timestamp \
  --value energy_mwh \
  --out hourly_energy.parquet
```

## Join

### `gat ts join`

Align multiple time series on timestamp:

```bash
gat ts join \
  load.parquet \
  solar.parquet \
  wind.parquet \
  --on timestamp \
  --out combined.parquet
```

### Join Types

```bash
gat ts join file1.parquet file2.parquet \
  --on timestamp \
  --how outer \          # outer (default), inner, left
  --out joined.parquet
```

| Type | Description |
|------|-------------|
| `outer` | Keep all timestamps from both files |
| `inner` | Keep only matching timestamps |
| `left` | Keep all from first file |

### Handling Missing Values

After outer join, missing values appear as null. Fill them:

```bash
# Join then fill
gat ts join load.parquet solar.parquet --on timestamp --out temp.parquet
gat ts fill temp.parquet --method forward --out filled.parquet
```

### Examples

**Combine load and weather:**
```bash
gat ts join \
  load_forecast.parquet \
  weather_data.parquet \
  --on timestamp \
  --how inner \
  --out load_weather.parquet
```

**Merge multiple generators:**
```bash
gat ts join \
  gen_1.parquet \
  gen_2.parquet \
  gen_3.parquet \
  --on timestamp \
  --suffix _1,_2,_3 \
  --out all_generators.parquet
```

## Aggregate

### `gat ts agg`

Group and aggregate time series data:

```bash
gat ts agg input.parquet \
  --group sensor_id \
  --value power_mw \
  --agg mean \
  --out aggregated.parquet
```

### Aggregation Functions

| Function | Description |
|----------|-------------|
| `sum` | Total value |
| `mean` | Average |
| `min` | Minimum |
| `max` | Maximum |
| `count` | Number of records |
| `std` | Standard deviation |

### Multiple Aggregations

```bash
gat ts agg input.parquet \
  --group region,fuel_type \
  --value power_mw \
  --agg sum,mean,max \
  --out regional_stats.parquet
```

### Examples

**Regional load summary:**
```bash
gat ts agg load_data.parquet \
  --group region \
  --value load_mw \
  --agg sum \
  --out regional_load.parquet
```

**Hourly statistics by generator:**
```bash
# First resample to hourly
gat ts resample raw_data.parquet --rule 1h --out hourly.parquet

# Then aggregate by generator
gat ts agg hourly.parquet \
  --group generator_id \
  --value mean_value \
  --agg mean,min,max \
  --out generator_stats.parquet
```

**Fleet-level generation by fuel:**
```bash
gat ts agg generation.parquet \
  --group fuel_type,hour_of_day \
  --value power_mw \
  --agg sum \
  --out fuel_mix_hourly.parquet
```

## Fill Missing Values

### `gat ts fill`

Handle gaps in time series:

```bash
gat ts fill input.parquet \
  --timestamp timestamp \
  --value power_mw \
  --method forward \
  --out filled.parquet
```

### Fill Methods

| Method | Description |
|--------|-------------|
| `forward` | Fill with last known value |
| `backward` | Fill with next known value |
| `linear` | Linear interpolation |
| `zero` | Fill with zero |
| `mean` | Fill with column mean |

### Examples

**Forward fill SCADA dropouts:**
```bash
gat ts fill scada.parquet \
  --method forward \
  --max-gap 30min \
  --out filled.parquet
```

**Interpolate missing weather:**
```bash
gat ts fill weather.parquet \
  --method linear \
  --timestamp timestamp \
  --value temperature \
  --out interpolated.parquet
```

## Validate Data Quality

### `gat ts validate`

Check time series for issues:

```bash
gat ts validate input.parquet \
  --timestamp timestamp \
  --value power_mw
```

**Output:**
```
Time Series Validation
──────────────────────
Records:         8,760
Time range:      2024-01-01 to 2024-12-31
Interval:        1h (detected)

Issues found:
  Missing values:   24 (0.27%)
  Duplicate times:  0
  Out-of-order:     0
  Outliers (3σ):    12 (0.14%)

Statistics:
  Min:    85.2 MW
  Max:    425.8 MW
  Mean:   215.3 MW
  Std:    62.1 MW
```

### Options

```bash
gat ts validate input.parquet \
  --timestamp timestamp \
  --value power_mw \
  --outlier-threshold 3.0 \   # Standard deviations
  --expected-interval 1h \     # Flag irregular intervals
  --out validation_report.json
```

## Slice Time Windows

### `gat ts slice`

Extract specific time periods:

```bash
gat ts slice input.parquet \
  --from "2024-07-01T00:00:00Z" \
  --to "2024-07-31T23:59:59Z" \
  --out july_data.parquet
```

### Examples

**Peak summer week:**
```bash
gat ts slice load.parquet \
  --from "2024-07-15" \
  --to "2024-07-21" \
  --out peak_week.parquet
```

**Last 24 hours:**
```bash
gat ts slice realtime.parquet \
  --last 24h \
  --out recent.parquet
```

## Practical Workflows

### Multi-Period OPF Preparation

Prepare time series data for multi-period optimal power flow:

```bash
#!/bin/bash
# prepare_multiperiod.sh - Create dispatch inputs

# 1. Resample load to hourly
gat ts resample raw_load.parquet \
  --rule 1h \
  --agg mean \
  --out load_hourly.parquet

# 2. Resample renewable forecasts
gat ts resample solar_forecast.parquet \
  --rule 1h \
  --agg mean \
  --out solar_hourly.parquet

gat ts resample wind_forecast.parquet \
  --rule 1h \
  --agg mean \
  --out wind_hourly.parquet

# 3. Join into single timeline
gat ts join \
  load_hourly.parquet \
  solar_hourly.parquet \
  wind_hourly.parquet \
  --on timestamp \
  --out dispatch_inputs.parquet

# 4. Validate completeness
gat ts validate dispatch_inputs.parquet

# 5. Run multi-period OPF
gat opf dc grid.arrow \
  --time-series dispatch_inputs.parquet \
  --horizon 24h \
  --out dispatch_results.parquet
```

### State Estimation Pipeline

Prepare measurements for state estimation:

```bash
#!/bin/bash
# se_data_prep.sh - Prepare SE measurements

# 1. Resample all telemetry to common interval
for file in scada_*.parquet; do
    gat ts resample "$file" \
      --rule 1min \
      --agg last \
      --out "resampled_$file"
done

# 2. Join all measurements
gat ts join resampled_*.parquet \
  --on timestamp \
  --how outer \
  --out all_measurements.parquet

# 3. Fill short gaps (< 5 min)
gat ts fill all_measurements.parquet \
  --method forward \
  --max-gap 5min \
  --out filled_measurements.parquet

# 4. Validate data quality
gat ts validate filled_measurements.parquet > se_data_quality.txt

# 5. Extract latest snapshot for SE
gat ts slice filled_measurements.parquet \
  --last 1min \
  --out current_measurements.parquet
```

### Load Forecasting Feature Engineering

Create ML features from time series:

```bash
#!/bin/bash
# load_features.sh - Feature engineering for load forecasting

INPUT="historical_load.parquet"
OUTPUT="load_features.parquet"

# Create lagged features and aggregations
python3 << EOF
import polars as pl

df = pl.read_parquet("$INPUT")

# Add time features
df = df.with_columns([
    pl.col("timestamp").dt.hour().alias("hour"),
    pl.col("timestamp").dt.weekday().alias("weekday"),
    pl.col("timestamp").dt.month().alias("month"),
])

# Add rolling statistics
df = df.with_columns([
    pl.col("load_mw").rolling_mean(window_size=24).alias("load_24h_avg"),
    pl.col("load_mw").rolling_max(window_size=24).alias("load_24h_max"),
    pl.col("load_mw").rolling_std(window_size=24).alias("load_24h_std"),
])

# Add lag features
df = df.with_columns([
    pl.col("load_mw").shift(1).alias("load_lag_1h"),
    pl.col("load_mw").shift(24).alias("load_lag_24h"),
    pl.col("load_mw").shift(168).alias("load_lag_1w"),
])

df.write_parquet("$OUTPUT")
print(f"Created {len(df)} feature rows")
EOF

echo "Features saved to $OUTPUT"
```

### Real-Time Dashboard Data

Prepare streaming data for visualization:

```bash
#!/bin/bash
# dashboard_data.sh - Real-time dashboard refresh

while true; do
    # Get latest 15 minutes
    gat ts slice realtime_feed.parquet \
      --last 15min \
      --out /tmp/recent.parquet

    # Compute current statistics
    gat ts agg /tmp/recent.parquet \
      --group resource_type \
      --value power_mw \
      --agg sum,mean \
      --out /var/www/dashboard/current_stats.json \
      --format json

    # Update load profile chart
    gat ts resample realtime_feed.parquet \
      --rule 1min \
      --agg mean \
      --out /var/www/dashboard/load_profile.json \
      --format json

    sleep 60
done
```

## Python Integration

```python
import subprocess
import polars as pl
from datetime import datetime, timedelta

def resample_gat(input_path, rule, output_path):
    """Resample time series using GAT."""
    subprocess.run([
        "gat", "ts", "resample", input_path,
        "--rule", rule,
        "--out", output_path
    ], check=True)
    return pl.read_parquet(output_path)

def join_timeseries(files, output_path):
    """Join multiple time series files."""
    cmd = ["gat", "ts", "join"] + files + ["--on", "timestamp", "--out", output_path]
    subprocess.run(cmd, check=True)
    return pl.read_parquet(output_path)

# Example usage
load_hourly = resample_gat("load_raw.parquet", "1h", "load_hourly.parquet")
combined = join_timeseries(["load.parquet", "solar.parquet"], "combined.parquet")

print(f"Hourly load shape: {load_hourly.shape}")
print(f"Combined data range: {combined['timestamp'].min()} to {combined['timestamp'].max()}")
```

## Troubleshooting

### "Timestamp parse error"

**Cause:** Non-standard timestamp format

**Solution:**
```bash
# Specify format explicitly
gat ts resample input.csv \
  --timestamp-format "%Y-%m-%d %H:%M:%S" \
  --rule 1h \
  --out output.parquet
```

### Large gaps in resampled data

**Cause:** Source data has long gaps

**Solution:**
```bash
# Check for gaps first
gat ts validate input.parquet

# Fill gaps before resampling
gat ts fill input.parquet --method linear --out filled.parquet
gat ts resample filled.parquet --rule 1h --out hourly.parquet
```

### Memory issues with large files

**Solution:**
```bash
# Process in chunks
gat ts slice large_file.parquet \
  --from "2024-01" --to "2024-02" \
  --out jan.parquet

gat ts resample jan.parquet --rule 1h --out jan_hourly.parquet
# Repeat for other months, then join
```

### Misaligned timestamps after join

**Cause:** Different time zones or precision

**Solution:**
```bash
# Normalize to UTC first
gat ts normalize file1.parquet --tz UTC --out file1_utc.parquet
gat ts normalize file2.parquet --tz UTC --out file2_utc.parquet
gat ts join file1_utc.parquet file2_utc.parquet --on timestamp --out joined.parquet
```

## Related Commands

- [State Estimation](@/guide/se.md) — Process measurement time series
- [OPF](@/guide/opf.md) — Multi-period optimal dispatch
- [Batch Analysis](@/guide/batch.md) — Time-varying scenario analysis
- [DERMS](@/guide/derms.md) — DER schedule optimization
