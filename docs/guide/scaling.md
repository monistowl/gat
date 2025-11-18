# Scaling workflows (practical steps)

This guide distills the multi-horizon scaling plan into near-term actions you can apply immediately. See `docs/archive/SCALING.md` for the longer-range roadmap.

## On a single machine
- **Use all cores:** pass `--threads auto` (default) or an explicit count to heavy commands (`pf`, `opf`, `nminus1`, `se`).
- **Prefer sparse/dense backends where they fit:** choose `--solver faer` for dense-friendly paths or stay with the default Gaussian solver for small cases.
- **Write partitioned Parquet:** set `--out-partitions run_id,date[/contingency]` so downstream tools and resumptions can filter work efficiently.
- **Capture run manifests:** keep the emitted `run.json` files beside outputs so `gat runs resume` or dashboards can trace provenance.

## Fanning out embarrassingly parallel work
- **Chunk by contingency or time:** for `gat nminus1 dc`, partition by `(contingency × hour)` so workers remain independent.
- **Process pools for isolation:** introduce a process-level executor when memory isolation matters (see the executor trait sketch in `docs/archive/SCALING.md`).
- **Shared artifact store:** standardize outputs under `runs/<run_id>/<stage>/...` so local disks or object stores (via `opendal`) can serve multiple workers.

## Preparing for cluster/job runners
- **Emit ready-made specs:** plan for `gat plan ... --emit argo.yaml` or similar to hand workloads to Argo/Flyte/Temporal once executor plumbing lands.
- **Observability hooks:** wire `tracing` subscribers to OTLP early so logs/metrics survive a transition to remote execution.

These steps keep today’s CLI runs fast while leaving clear upgrade paths to the larger-scale designs in the archived document.
