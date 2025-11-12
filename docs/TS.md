# Time-Series CLI

`gat ts` exposes lightweight helpers for working with telemetry files (CSV or Parquet). All commands operate over a timestamp column (default `timestamp`) with measured values.

## Resample

```
gat ts resample test_data/ts/telemetry.parquet \
  --rule 5s \
  --timestamp timestamp \
  --value value \
  --out out/telemetry.resampled.parquet
```

The resampler buckets `timestamp` values (seconds since epoch) into fixed-width windows. The output table contains `bucket_start`, `count`, `mean_value`, `min_value`, and `max_value`.

## Join

```
gat ts join test_data/ts/telemetry.parquet \
  test_data/ts/telemetry_extra.parquet \
  --on timestamp \
  --out out/telemetry.joined.parquet
```

This performs an outer join and preserves the `timestamp` column once in the output. Use the joined table to align multiple sensor feeds before feeding them to analytics or visualizations.

For both commands, either CSV or Parquet inputs/outputs are supported; matching the extension (`.csv`/`.parquet`) determines the parser/writer.

## Aggregate

```
gat ts agg test_data/ts/telemetry.parquet \
  --group sensor \
  --value value \
  --agg sum \
  --out out/telemetry.agg.parquet
```

Groups the input table by the chosen column and runs `sum | mean | min | max | count` across the selected value column, emitting the aggregated table (Polars naming convention applies, e.g., `value_sum`).
