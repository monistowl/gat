+++
title = "Scaling Guide"
description = "Scaling roadmap"
weight = 42
+++

# Scaling roadmap

This roadmap explains how to scale GAT from a single machine to thousands of nodes, with concrete CLI/architecture changes tied to each horizon.

## Horizon 1 — Multicore on one machine

* Add `rayon` work-stealing for embarrassingly parallel workloads (N-1, Monte-Carlo, per-hour PF) while keeping the core math synchronous.
* Provide `--threads <N>|auto` on heavy commands (default to `num_cpus::get()`).
* Keep pure-Rust linear algebra (`faer`, `sprs`) but allow feature flags for OpenBLAS/MKL if dense backends are needed.
* Hide factorization engines behind a trait so you can hot-swap e.g. `--solver {faer,openblas,mkl}` later.
* Standardize outputs on partitioned Parquet with stable schemas and add `--out-partitions` (e.g., `run_id,date/contingency`).
* Memory-map Arrow IPC for zero-copy handoffs and checkpoint each long run with `run.json` so `gat runs resume` can pick up from the last saved chunk.

## Horizon 2 — Fan-out onto many small machines

* Introduce an `Executor` trait (default `LocalExecutor`, plus `ProcessPoolExecutor` to fork child `gat` binaries for memory isolation).
* CLI flags like `--executor {local,process}` and `--max-procs` control how many heads run simultaneously.
* Route all chunk IO through an object-store abstraction (e.g., `opendal`) so the same code works on local filesystems or S3/GCS buckets.
* Optional `gat-worker` binary pulls chunk specs from a queue (NATS JetStream, Redis Streams) and writes results into the artifact store.
* The GUI watches manifests instead of processes, making it equally useful for local or remote runs.

## Horizon 3 — Kubernetes / Nomad / Temporal

* Ship OCI images `ghcr.io/.../gat:<gitsha>` that include CPU/CUDA builds and obey input/output contract (S3 URIs) so subcommands can run as containers.
* Provide template generators for workflow engines (Argo, Flyte, Temporal) so teams can run import → partition → PF/OPF fanout → reduce DAGs.
* Introduce a tiny control-plane service (`gat-svc`) with a gRPC API to submit runs, list chunks, and stream logs.
* Emit OTLP-friendly `tracing` with dashboards for stage throughput, failure maps, and chunk timings.
* Add spot-friendly chunking (`--max-runtime-per-chunk`) and priority controls per stage.

## Horizon 4 — HPC / MPI / SLURM

* Implement a `SlurmExecutor` that emits ready-to-submit `sbatch` scripts with environment variables for credentials and run manifests.
* Optional `MPIExecutor` for tightly coupled solvers.
* Call into PETSc/Trilinos for distributed linear algebra while keeping Rust for orchestration.
* Domain decomposition (region-by-region workflows) keeps map data local and exchanges interface variables iteratively.

## Horizon 5 — Serverless bursts

* Package a cold-start-friendly worker (`gat-map`) that handles one chunk in <15 minutes for Lambda/Batch.
* Keep compute in Batch but orchestrate with Step Functions or Cloud Workflows.
* Optionally expose Arrow Flight endpoints so Python/R clients query telemetry without copying data.
* Use DataFusion/Ballista for distributed joins and aggregations near the data plane.

## Concrete scale-outs you can do today

1. **N-1 DC screening**: chunk by hour × contingency, run DCPF per chunk, emit violations, and reduce to top violators per element/hour. Start with `ProcessPoolExecutor` before moving to Argo or Slurm arrays.
2. **Monte-Carlo load/renewables**: chunk by scenario and hour, keep RNG seeds in the manifest for replay, and optionally sample weather with GPU (`wgpu` or CUDA) while saving CPU for solves.
3. **Rolling OPF**: DAG with forecasting → DC-OPF batches → post-checks. Run per BA or partition for data locality and stitch interfaces afterward.

## Code & CLI pipelines to prioritize

* `crates/gat-exec/` with the `Executor` trait plus `Local`, `Process`, and `Slurm` implementations.
* `gat --executor <name> --max-procs <N> --artifact <uri> --queue <uri>` CLI flags.
* A universal `ChunkSpec`/`ChunkResult` JSON contract and `--chunk-spec`/`--emit-chunk-specs` helpers for chunk producers/consumers.
* `crates/gat-artifacts/` (object store via `opendal`) and `crates/gat-metadata/` (manifests, checksums).
* Remote gRPC services (`SubmitRun`, `GetRun`, `ListChunks`, `GetLogs`) plus optional Arrow Flight for bulk results.
* `gat admin retry-failed <run_id>` plus exponential backoff for idempotent chunk retries.

## Solver & data strategy

* Keep AC PF in-process until you need distributed solves; 90% of throughput comes from chunk parallelism.
* Cache Jacobian sparsity patterns and warm-start Newton with the previous hour’s solution when replaying contingencies.
* Default to DC-OPF with HiGHS for fleet-scale throughput and reserve AC-OPF for flagged slices.
* Partition Parquet everywhere, keep run IDs content-addressed, and store large artifacts (plots, maps) in the artifact tree.
* Use short-lived object store credentials and namespace runs per org/project so multi-tenant policies (OPA/Rego) can gate access.
