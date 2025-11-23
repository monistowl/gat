# gat-batch — Parallel Job Orchestration

Batch execution framework for running power-flow, OPF, and analysis solves in parallel across scenarios and machines.

## Quick Overview

**Use Cases:**
- Run DC/AC power flow over 1000+ scenarios
- Execute N-1 contingency analysis at scale
- Perform OPF optimization over time-varying conditions
- Parallelize analytics across datasets

## Core APIs

```rust
// Submit batch power-flow job
let batch = BatchJob::powerflow(manifest, max_workers)?;
let results = batch.run().await?;

// Check job status
let status = batch.status();
println!("Progress: {}/{}", status.completed, status.total);

// Resume incomplete batch
let batch = BatchJob::resume(job_id)?;
results.extend(batch.run().await?);
```

## Features

- **Parallel Execution** — Fan out solves across CPU cores and machines
- **Resume Support** — Pick up incomplete batches without re-running finished jobs
- **Progress Tracking** — Monitor job completion in real-time
- **Result Aggregation** — Collect and summarize results
- **Error Recovery** — Configurable retry logic with exponential backoff

## Manifest Format

Batch jobs consume a manifest JSON describing scenarios:

```json
{
  "scenarios": [
    {"id": "base", "grid": "base.arrow", "loads": "loads_base.csv"},
    {"id": "peak_demand", "grid": "base.arrow", "loads": "loads_peak.csv"}
  ],
  "solver": "dc",
  "timeout_secs": 60,
  "max_jobs": 4
}
```

## Output Structure

Results saved as:
```
runs/batch/<job-name>/
├── batch_manifest.json       # Status and metadata
├── job_<id>/
│   └── result.parquet        # Per-scenario results
└── summary/
    └── aggregated_results.parquet
```

## Configuration

```rust
let config = BatchConfig {
    max_workers: 8,
    timeout_per_job: Duration::from_secs(60),
    retry_attempts: 3,
    output_dir: "runs/batch".into(),
};
```

## Testing

```bash
cargo test -p gat-batch
```

## Related Crates

- **gat-core** — Solvers executed by batch
- **gat-io** — Manifest and result I/O
- **gat-scenarios** — Generates batch manifests

## See Also

- [GAT Main README](../../README.md)
- `docs/guide/cli-architecture.md` — Batch execution pipeline
- [gat-cli README](../gat-cli/README.md) — `gat batch` commands
