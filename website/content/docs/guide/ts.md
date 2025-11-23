+++
title = "Time Series"
description = "Time-Series CLI"
weight = 12
+++

# Time-Series CLI

`gat ts` provides helpers for telemetry files (CSV or Parquet) indexed by a timestamp column (default `timestamp`). All commands emit Parquet by default.

## Resample

```bash
gat ts resample test_data/ts/telemetry.parquet \
  --rule 5s \
  --timestamp timestamp \
  --value value \
  --out out/telemetry.resampled.parquet
```

Buckets `timestamp` values into fixed-width windows. The output table includes `bucket_start`, `count`, `mean_value`, `min_value`, and `max_value`.

## Join

```bash
gat ts join test_data/ts/telemetry.parquet \
  test_data/ts/telemetry_extra.parquet \
  --on timestamp \
  --out out/telemetry.joined.parquet
```

Performs an outer join, preserving the shared `timestamp` column. Use the joined result to align multiple sensor feeds before visualization or analytics.

## Aggregate

```bash
gat ts agg test_data/ts/telemetry.parquet \
  --group sensor \
  --value value \
  --agg sum \
  --out out/telemetry.agg.parquet
```

Groups by the specified column and runs the requested aggregation (`sum | mean | min | max | count`), emitting columns such as `value_sum` following Polars naming.
