# Command-Line Help for `gat-cli`

This document contains the help content for the `gat-cli` command-line program.

**Command Overview:**

* [`gat-cli`↴](#gat-cli)
* [`gat-cli import`↴](#gat-cli-import)
* [`gat-cli import psse`↴](#gat-cli-import-psse)
* [`gat-cli import matpower`↴](#gat-cli-import-matpower)
* [`gat-cli import cim`↴](#gat-cli-import-cim)
* [`gat-cli validate`↴](#gat-cli-validate)
* [`gat-cli graph`↴](#gat-cli-graph)
* [`gat-cli graph stats`↴](#gat-cli-graph-stats)
* [`gat-cli graph islands`↴](#gat-cli-graph-islands)
* [`gat-cli graph export`↴](#gat-cli-graph-export)
* [`gat-cli graph visualize`↴](#gat-cli-graph-visualize)
* [`gat-cli scenarios`↴](#gat-cli-scenarios)
* [`gat-cli scenarios validate`↴](#gat-cli-scenarios-validate)
* [`gat-cli scenarios list`↴](#gat-cli-scenarios-list)
* [`gat-cli scenarios expand`↴](#gat-cli-scenarios-expand)
* [`gat-cli scenarios materialize`↴](#gat-cli-scenarios-materialize)
* [`gat-cli completions`↴](#gat-cli-completions)
* [`gat-cli pf`↴](#gat-cli-pf)
* [`gat-cli pf dc`↴](#gat-cli-pf-dc)
* [`gat-cli pf ac`↴](#gat-cli-pf-ac)
* [`gat-cli nminus1`↴](#gat-cli-nminus1)
* [`gat-cli nminus1 dc`↴](#gat-cli-nminus1-dc)
* [`gat-cli ts`↴](#gat-cli-ts)
* [`gat-cli ts resample`↴](#gat-cli-ts-resample)
* [`gat-cli ts join`↴](#gat-cli-ts-join)
* [`gat-cli ts agg`↴](#gat-cli-ts-agg)
* [`gat-cli dist`↴](#gat-cli-dist)
* [`gat-cli dist import`↴](#gat-cli-dist-import)
* [`gat-cli dist pf`↴](#gat-cli-dist-pf)
* [`gat-cli dist opf`↴](#gat-cli-dist-opf)
* [`gat-cli dist hostcap`↴](#gat-cli-dist-hostcap)
* [`gat-cli derms`↴](#gat-cli-derms)
* [`gat-cli derms envelope`↴](#gat-cli-derms-envelope)
* [`gat-cli derms schedule`↴](#gat-cli-derms-schedule)
* [`gat-cli derms stress-test`↴](#gat-cli-derms-stress-test)
* [`gat-cli adms`↴](#gat-cli-adms)
* [`gat-cli adms flisr-sim`↴](#gat-cli-adms-flisr-sim)
* [`gat-cli adms vvo-plan`↴](#gat-cli-adms-vvo-plan)
* [`gat-cli adms outage-mc`↴](#gat-cli-adms-outage-mc)
* [`gat-cli adms state-estimation`↴](#gat-cli-adms-state-estimation)
* [`gat-cli opf`↴](#gat-cli-opf)
* [`gat-cli opf dc`↴](#gat-cli-opf-dc)
* [`gat-cli opf ac`↴](#gat-cli-opf-ac)
* [`gat-cli se`↴](#gat-cli-se)
* [`gat-cli se wls`↴](#gat-cli-se-wls)
* [`gat-cli viz`↴](#gat-cli-viz)
* [`gat-cli viz plot`↴](#gat-cli-viz-plot)
* [`gat-cli analytics`↴](#gat-cli-analytics)
* [`gat-cli analytics ptdf`↴](#gat-cli-analytics-ptdf)
* [`gat-cli analytics ds`↴](#gat-cli-analytics-ds)
* [`gat-cli analytics reliability`↴](#gat-cli-analytics-reliability)
* [`gat-cli analytics elcc`↴](#gat-cli-analytics-elcc)
* [`gat-cli featurize`↴](#gat-cli-featurize)
* [`gat-cli featurize gnn`↴](#gat-cli-featurize-gnn)
* [`gat-cli featurize kpi`↴](#gat-cli-featurize-kpi)
* [`gat-cli geo`↴](#gat-cli-geo)
* [`gat-cli geo join`↴](#gat-cli-geo-join)
* [`gat-cli geo featurize`↴](#gat-cli-geo-featurize)
* [`gat-cli alloc`↴](#gat-cli-alloc)
* [`gat-cli alloc rents`↴](#gat-cli-alloc-rents)
* [`gat-cli alloc kpi`↴](#gat-cli-alloc-kpi)
* [`gat-cli batch`↴](#gat-cli-batch)
* [`gat-cli batch pf`↴](#gat-cli-batch-pf)
* [`gat-cli batch opf`↴](#gat-cli-batch-opf)
* [`gat-cli benchmark`↴](#gat-cli-benchmark)
* [`gat-cli benchmark pfdelta`↴](#gat-cli-benchmark-pfdelta)
* [`gat-cli benchmark pglib`↴](#gat-cli-benchmark-pglib)
* [`gat-cli benchmark opfdata`↴](#gat-cli-benchmark-opfdata)
* [`gat-cli runs`↴](#gat-cli-runs)
* [`gat-cli runs list`↴](#gat-cli-runs-list)
* [`gat-cli runs describe`↴](#gat-cli-runs-describe)
* [`gat-cli runs resume`↴](#gat-cli-runs-resume)
* [`gat-cli dataset`↴](#gat-cli-dataset)
* [`gat-cli dataset rts-gmlc`↴](#gat-cli-dataset-rts-gmlc)
* [`gat-cli dataset rts-gmlc fetch`↴](#gat-cli-dataset-rts-gmlc-fetch)
* [`gat-cli dataset hiren`↴](#gat-cli-dataset-hiren)
* [`gat-cli dataset hiren list`↴](#gat-cli-dataset-hiren-list)
* [`gat-cli dataset hiren fetch`↴](#gat-cli-dataset-hiren-fetch)
* [`gat-cli dataset dsgrid`↴](#gat-cli-dataset-dsgrid)
* [`gat-cli dataset sup3rcc`↴](#gat-cli-dataset-sup3rcc)
* [`gat-cli dataset sup3rcc fetch`↴](#gat-cli-dataset-sup3rcc-fetch)
* [`gat-cli dataset sup3rcc sample`↴](#gat-cli-dataset-sup3rcc-sample)
* [`gat-cli dataset pras`↴](#gat-cli-dataset-pras)
* [`gat-cli dataset public`↴](#gat-cli-dataset-public)
* [`gat-cli dataset public list`↴](#gat-cli-dataset-public-list)
* [`gat-cli dataset public describe`↴](#gat-cli-dataset-public-describe)
* [`gat-cli dataset public fetch`↴](#gat-cli-dataset-public-fetch)
* [`gat-cli dataset eia`↴](#gat-cli-dataset-eia)
* [`gat-cli dataset ember`↴](#gat-cli-dataset-ember)
* [`gat-cli version`↴](#gat-cli-version)
* [`gat-cli version sync`↴](#gat-cli-version-sync)

## `gat-cli`

**Usage:** `gat-cli [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `import` — Import data from various formats
* `validate` — Validate a dataset against a schema
* `graph` — Graph utilities
* `scenarios` — Scenario definitions and materialization workflows
* `completions` — Generate shell completion scripts
* `pf` — Power flow solvers
* `nminus1` — Contingency analysis
* `ts` — Time-series utilities
* `dist` — Distribution modeling helpers
* `derms` — DERMS analytics
* `adms` — ADMS reliability workflows
* `opf` — Optimal power flow
* `se` — State estimation
* `viz` — Visualization helpers
* `analytics` — Grid analytics helpers (PTDF, sensitivities, etc.)
* `featurize` — Feature extraction for ML models (GNN, KPI predictors, etc.)
* `geo` — Geo-spatial tools (GIS joins, polygon mapping, spatial features)
* `alloc` — Allocation and settlement tools (congestion rents, cost attribution)
* `batch` — Scenario batch runners for PF/OPF (CANOS-style fan-out)
* `benchmark` — Benchmarking suites for OPF/PF solvers
* `runs` — Run management
* `dataset` — Dataset adapters
* `version` — Release version helpers

###### **Options:**

* `--log-level <LOG_LEVEL>` — Set the logging level

  Default value: `info`
* `--profile <PROFILE>` — Set the profile (e.g., "dev", "release")

  Default value: `dev`



## `gat-cli import`

Import data from various formats

**Usage:** `gat-cli import <COMMAND>`

###### **Subcommands:**

* `psse` — Import PSS/E RAW file
* `matpower` — Import MATPOWER case file
* `cim` — Import CIM RDF file



## `gat-cli import psse`

Import PSS/E RAW file

**Usage:** `gat-cli import psse --raw <RAW> --output <OUTPUT>`

###### **Options:**

* `--raw <RAW>` — Path to the RAW file
* `-o`, `--output <OUTPUT>` — Output file path (Arrow format)



## `gat-cli import matpower`

Import MATPOWER case file

**Usage:** `gat-cli import matpower --m <M> --output <OUTPUT>`

###### **Options:**

* `--m <M>` — Path to the MATPOWER .m file
* `-o`, `--output <OUTPUT>` — Output file path (Arrow format)



## `gat-cli import cim`

Import CIM RDF file

**Usage:** `gat-cli import cim --rdf <RDF> --output <OUTPUT>`

###### **Options:**

* `--rdf <RDF>` — Path to the CIM RDF file
* `-o`, `--output <OUTPUT>` — Output file path (Arrow format)



## `gat-cli validate`

Validate a dataset against a schema

**Usage:** `gat-cli validate --spec <SPEC>`

###### **Options:**

* `--spec <SPEC>` — Path to the dataset specification file



## `gat-cli graph`

Graph utilities

**Usage:** `gat-cli graph <COMMAND>`

###### **Subcommands:**

* `stats` — Graph stats summary
* `islands` — Find islands in the grid
* `export` — Export graph to various formats
* `visualize` — Compute force-directed layout for visualization



## `gat-cli graph stats`

Graph stats summary

**Usage:** `gat-cli graph stats <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)



## `gat-cli graph islands`

Find islands in the grid

**Usage:** `gat-cli graph islands [OPTIONS] <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--emit` — Emit island IDs



## `gat-cli graph export`

Export graph to various formats

**Usage:** `gat-cli graph export [OPTIONS] <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--format <FORMAT>` — Output format (e.g., graphviz)

  Default value: `graphviz`
* `-o`, `--out <OUT>` — Optional output file path



## `gat-cli graph visualize`

Compute force-directed layout for visualization

**Usage:** `gat-cli graph visualize [OPTIONS] <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--iterations <ITERATIONS>` — Number of simulation iterations

  Default value: `150`
* `-o`, `--out <OUT>` — Optional output file path



## `gat-cli scenarios`

Scenario definitions and materialization workflows

**Usage:** `gat-cli scenarios <COMMAND>`

###### **Subcommands:**

* `validate` — Validate a scenario specification (YAML/JSON)
* `list` — List normalized scenarios inside a spec
* `expand` — Expand templated scenarios into fully resolved definitions
* `materialize` — Materialize per-scenario grids and produce a manifest



## `gat-cli scenarios validate`

Validate a scenario specification (YAML/JSON)

**Usage:** `gat-cli scenarios validate --spec <SPEC>`

###### **Options:**

* `--spec <SPEC>` — Path to the scenario spec template



## `gat-cli scenarios list`

List normalized scenarios inside a spec

**Usage:** `gat-cli scenarios list [OPTIONS] --spec <SPEC>`

###### **Options:**

* `--spec <SPEC>` — Path to the scenario spec
* `--format <FORMAT>` — Output format (table or json)

  Default value: `table`



## `gat-cli scenarios expand`

Expand templated scenarios into fully resolved definitions

**Usage:** `gat-cli scenarios expand [OPTIONS] --spec <SPEC> --out <OUT>`

###### **Options:**

* `--spec <SPEC>` — Path to the scenario spec
* `--grid-file <GRID_FILE>` — Optional base grid file to override the spec
* `-o`, `--out <OUT>` — Path for the expanded output (JSON or YAML)



## `gat-cli scenarios materialize`

Materialize per-scenario grids and produce a manifest

**Usage:** `gat-cli scenarios materialize [OPTIONS] --spec <SPEC> --out-dir <OUT_DIR>`

###### **Options:**

* `--spec <SPEC>` — Path to the scenario spec
* `--grid-file <GRID_FILE>` — Optional base grid file (overrides spec)
* `-o`, `--out-dir <OUT_DIR>` — Directory where scenario grids and manifest are written
* `--drop-outaged` — Drop outaged components from the exported grids

  Default value: `true`



## `gat-cli completions`

Generate shell completion scripts

**Usage:** `gat-cli completions [OPTIONS] <SHELL>`

###### **Arguments:**

* `<SHELL>` — Shell type

  Possible values: `bash`, `elvish`, `fish`, `powershell`, `zsh`


###### **Options:**

* `-o`, `--out <OUT>` — Write output to a file instead of stdout



## `gat-cli pf`

Power flow solvers

**Usage:** `gat-cli pf <COMMAND>`

###### **Subcommands:**

* `dc` — Run DC power flow
* `ac` — Run AC power flow



## `gat-cli pf dc`

Run DC power flow

**Usage:** `gat-cli pf dc [OPTIONS] --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `-o`, `--out <OUT>` — Output file path for flows (Parquet format)
* `--threads <THREADS>` — Threading hint (`auto` or integer)

  Default value: `auto`
* `--solver <SOLVER>` — Solver to use (gauss, faer)

  Default value: `gauss`
* `--lp-solver <LP_SOLVER>` — LP solver for the cost minimization (clarabel, coin_cbc, highs)

  Default value: `clarabel`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli pf ac`

Run AC power flow

**Usage:** `gat-cli pf ac [OPTIONS] --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `-o`, `--out <OUT>` — Output file path for flows (Parquet format)
* `--tol <TOL>` — Tolerance for convergence

  Default value: `1e-8`
* `--max-iter <MAX_ITER>` — Maximum number of iterations

  Default value: `20`
* `--threads <THREADS>` — Threading hint (`auto` or integer)

  Default value: `auto`
* `--solver <SOLVER>` — Solver to use (gauss, faer)

  Default value: `gauss`
* `--lp-solver <LP_SOLVER>` — LP solver for the cost minimization (clarabel, coin_cbc, highs)

  Default value: `clarabel`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli nminus1`

Contingency analysis

**Usage:** `gat-cli nminus1 <COMMAND>`

###### **Subcommands:**

* `dc` — Run a DC N-1 screening scenario



## `gat-cli nminus1 dc`

Run a DC N-1 screening scenario

**Usage:** `gat-cli nminus1 dc [OPTIONS] --contingencies <CONTINGENCIES> --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--contingencies <CONTINGENCIES>` — Contingency CSV (`branch_id,label`)
* `-o`, `--out <OUT>` — Output Parquet for scenario summaries
* `--branch-limits <BRANCH_LIMITS>` — Optional branch limits CSV (branch_id,flow_limit) for violation checks
* `--threads <THREADS>` — Threads: `auto` or numeric

  Default value: `auto`
* `--solver <SOLVER>` — Solver to use (gauss, faer)

  Default value: `gauss`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli ts`

Time-series utilities

**Usage:** `gat-cli ts <COMMAND>`

###### **Subcommands:**

* `resample` — Resample a telemetry series
* `join` — Join two telemetry datasets
* `agg` — Aggregate values by a column



## `gat-cli ts resample`

Resample a telemetry series

**Usage:** `gat-cli ts resample [OPTIONS] --rule <RULE> --out <OUT> <INPUT>`

###### **Arguments:**

* `<INPUT>` — Input time-series file (CSV or Parquet)

###### **Options:**

* `--timestamp <TIMESTAMP>` — Timestamp column name

  Default value: `timestamp`
* `--value <VALUE>` — Value column to aggregate

  Default value: `value`
* `--rule <RULE>` — Resampling rule (e.g., 5s, 1m, 1h)
* `-o`, `--out <OUT>` — Output file path (CSV or Parquet)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli ts join`

Join two telemetry datasets

**Usage:** `gat-cli ts join [OPTIONS] --out <OUT> <LEFT> <RIGHT>`

###### **Arguments:**

* `<LEFT>` — Left-hand input file (CSV or Parquet)
* `<RIGHT>` — Right-hand input file (CSV or Parquet)

###### **Options:**

* `--on <ON>` — Key column to join on

  Default value: `timestamp`
* `-o`, `--out <OUT>` — Output file path (CSV or Parquet)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli ts agg`

Aggregate values by a column

**Usage:** `gat-cli ts agg [OPTIONS] --out <OUT> <INPUT>`

###### **Arguments:**

* `<INPUT>` — Input file path (CSV or Parquet)

###### **Options:**

* `--group <GROUP>` — Column to group by

  Default value: `sensor`
* `--value <VALUE>` — Value column to aggregate

  Default value: `value`
* `--agg <AGG>` — Aggregation to perform: sum|mean|min|max|count

  Default value: `sum`
* `-o`, `--out <OUT>` — Output file path (CSV or Parquet)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli dist`

Distribution modeling helpers

**Usage:** `gat-cli dist <COMMAND>`

###### **Subcommands:**

* `import` — Import MATPOWER into distribution tables
* `pf` — Run a distribution AC power flow
* `opf` — Run a simple hosting-capacity OPF for distribution feeders
* `hostcap` — Sweep hosting capacity over selected buses



## `gat-cli dist import`

Import MATPOWER into distribution tables

**Usage:** `gat-cli dist import [OPTIONS] --m <M> --output-dir <OUTPUT_DIR>`

###### **Options:**

* `--m <M>` — Source MATPOWER case file
* `--output-dir <OUTPUT_DIR>` — Output directory for dist tables
* `--feeder-id <FEEDER_ID>` — Optional feeder identifier to annotate the tables



## `gat-cli dist pf`

Run a distribution AC power flow

**Usage:** `gat-cli dist pf [OPTIONS] --grid-file <GRID_FILE> --out <OUT>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Grid file (Arrow format)
* `--out <OUT>` — Output Parquet path
* `--solver <SOLVER>` — Solver (`gauss`, `clarabel`, `highs`)

  Default value: `gauss`
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum iterations

  Default value: `20`



## `gat-cli dist opf`

Run a simple hosting-capacity OPF for distribution feeders

**Usage:** `gat-cli dist opf [OPTIONS] --grid-file <GRID_FILE> --out <OUT>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Grid file (Arrow format)
* `--out <OUT>` — Output Parquet path
* `--objective <OBJECTIVE>` — Objective descriptor

  Default value: `loss`
* `--solver <SOLVER>` — Solver to use

  Default value: `gauss`
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum iterations

  Default value: `20`



## `gat-cli dist hostcap`

Sweep hosting capacity over selected buses

**Usage:** `gat-cli dist hostcap [OPTIONS] --grid-file <GRID_FILE> --out-dir <OUT_DIR>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Grid file (Arrow format)
* `--out-dir <OUT_DIR>` — Output directory for artifacts
* `--bus <BUS>` — Bus IDs to target (comma-separated or repeated)
* `--max-injection <MAX_INJECTION>` — Maximum injection per bus

  Default value: `2.0`
* `--steps <STEPS>` — Number of steps per bus

  Default value: `8`
* `--solver <SOLVER>` — Solver to use

  Default value: `gauss`



## `gat-cli derms`

DERMS analytics

**Usage:** `gat-cli derms <COMMAND>`

###### **Subcommands:**

* `envelope` — Build DER flexibility envelopes
* `schedule` — Produce a scheduling recommendation
* `stress-test` — Run a stress-test over randomized price perturbations



## `gat-cli derms envelope`

Build DER flexibility envelopes

**Usage:** `gat-cli derms envelope [OPTIONS] --grid-file <GRID_FILE> --assets <ASSETS> --out <OUT>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Grid Arrow file
* `--assets <ASSETS>` — DER asset Parquet
* `-o`, `--out <OUT>` — Output Parquet path
* `--group-by <GROUP_BY>` — Grouping key (agg_id or bus)



## `gat-cli derms schedule`

Produce a scheduling recommendation

**Usage:** `gat-cli derms schedule [OPTIONS] --assets <ASSETS> --price-series <PRICE_SERIES> --out <OUT>`

###### **Options:**

* `--assets <ASSETS>` — DER asset Parquet
* `--price-series <PRICE_SERIES>` — Price series Parquet
* `-o`, `--out <OUT>` — Output Parquet path
* `--objective <OBJECTIVE>` — Objective name (for logging)

  Default value: `median-price`



## `gat-cli derms stress-test`

Run a stress-test over randomized price perturbations

**Usage:** `gat-cli derms stress-test [OPTIONS] --assets <ASSETS> --price-series <PRICE_SERIES> --output-dir <OUTPUT_DIR>`

###### **Options:**

* `--assets <ASSETS>` — DER asset Parquet
* `--price-series <PRICE_SERIES>` — Price series Parquet
* `--output-dir <OUTPUT_DIR>` — Output directory for scans
* `--scenarios <SCENARIOS>` — Number of scenarios to sample

  Default value: `16`
* `--seed <SEED>` — Optional RNG seed



## `gat-cli adms`

ADMS reliability workflows

**Usage:** `gat-cli adms <COMMAND>`

###### **Subcommands:**

* `flisr-sim` — Run FLISR reliability sampling
* `vvo-plan` — Volt/VAR planning runs
* `outage-mc` — Monte Carlo outage evaluation
* `state-estimation` — State estimation checks



## `gat-cli adms flisr-sim`

Run FLISR reliability sampling

**Usage:** `gat-cli adms flisr-sim [OPTIONS] --grid-file <GRID_FILE> --reliability <RELIABILITY> --output-dir <OUTPUT_DIR>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Grid file (Arrow)
* `--reliability <RELIABILITY>` — Reliability catalog Parquet
* `--output-dir <OUTPUT_DIR>` — Output directory for FLISR artifacts
* `--scenarios <SCENARIOS>` — Number of scenarios to sample

  Default value: `3`
* `--solver <SOLVER>` — Solver to use

  Default value: `gauss`
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum iterations

  Default value: `20`



## `gat-cli adms vvo-plan`

Volt/VAR planning runs

**Usage:** `gat-cli adms vvo-plan [OPTIONS] --grid-file <GRID_FILE> --output-dir <OUTPUT_DIR>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Grid file (Arrow)
* `--output-dir <OUTPUT_DIR>` — Output directory for day-type artifacts
* `--day-types <DAY_TYPES>` — Day types (comma-separated)

  Default value: `low,high`
* `--solver <SOLVER>` — Solver to use

  Default value: `gauss`
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum iterations

  Default value: `20`



## `gat-cli adms outage-mc`

Monte Carlo outage evaluation

**Usage:** `gat-cli adms outage-mc [OPTIONS] --reliability <RELIABILITY> --output-dir <OUTPUT_DIR>`

###### **Options:**

* `--reliability <RELIABILITY>` — Reliability catalog Parquet
* `--output-dir <OUTPUT_DIR>` — Output directory
* `--samples <SAMPLES>` — Sample count

  Default value: `20`
* `--seed <SEED>` — Optional RNG seed



## `gat-cli adms state-estimation`

State estimation checks

**Usage:** `gat-cli adms state-estimation [OPTIONS] --grid-file <GRID_FILE> --measurements <MEASUREMENTS> --out <OUT>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Grid file (Arrow)
* `--measurements <MEASUREMENTS>` — Measurements CSV
* `--out <OUT>` — Output Parquet for measurement residuals
* `--state-out <STATE_OUT>` — Optional output for estimated state
* `--solver <SOLVER>` — Solver to use

  Default value: `gauss`
* `--slack-bus <SLACK_BUS>` — Slack bus override



## `gat-cli opf`

Optimal power flow

**Usage:** `gat-cli opf <COMMAND>`

###### **Subcommands:**

* `dc` — Run DC optimal power flow
* `ac` — Run AC optimal power flow



## `gat-cli opf dc`

Run DC optimal power flow

**Usage:** `gat-cli opf dc [OPTIONS] --cost <COST> --limits <LIMITS> --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--cost <COST>` — Cost CSV (bus_id,marginal_cost)
* `--limits <LIMITS>` — Limits CSV (bus_id,pmin,pmax,demand)
* `-o`, `--out <OUT>` — Output Parquet for dispatch
* `--branch-limits <BRANCH_LIMITS>` — Optional branch limits CSV (branch_id,flow_limit)
* `--piecewise <PIECEWISE>` — Optional piecewise cost CSV (bus_id,start,end,slope)
* `--threads <THREADS>` — Threading hint (`auto` or integer)

  Default value: `auto`
* `--solver <SOLVER>` — Solver to use (gauss, faer)

  Default value: `gauss`
* `--lp-solver <LP_SOLVER>` — LP solver for the cost minimization (clarabel, coin_cbc, highs)

  Default value: `clarabel`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli opf ac`

Run AC optimal power flow

**Usage:** `gat-cli opf ac [OPTIONS] --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `-o`, `--out <OUT>` — Output Parquet for branch flows/residuals
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum number of iterations

  Default value: `20`
* `--threads <THREADS>` — Threading hint (`auto` or integer)

  Default value: `auto`
* `--solver <SOLVER>` — Solver to use (gauss, faer)

  Default value: `gauss`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli se`

State estimation

**Usage:** `gat-cli se <COMMAND>`

###### **Subcommands:**

* `wls` — Run WLS state estimation



## `gat-cli se wls`

Run WLS state estimation

**Usage:** `gat-cli se wls [OPTIONS] --measurements <MEASUREMENTS> --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid file (Arrow format)

###### **Options:**

* `--measurements <MEASUREMENTS>` — Measurements CSV (`measurement_type,branch_id,bus_id,value,weight,label`)
* `-o`, `--out <OUT>` — Output Parquet for measurement residuals
* `--state-out <STATE_OUT>` — Optional Parquet output for the solved bus angles
* `--threads <THREADS>` — Threading hint (`auto` or integer)

  Default value: `auto`
* `--solver <SOLVER>` — Solver to use (gauss, faer)

  Default value: `gauss`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)
* `--slack-bus <SLACK_BUS>` — Slack bus ID (defaults to lowest bus ID)



## `gat-cli viz`

Visualization helpers

**Usage:** `gat-cli viz <COMMAND>`

###### **Subcommands:**

* `plot` — Emit a basic visualization summary (placeholder)



## `gat-cli viz plot`

Emit a basic visualization summary (placeholder)

**Usage:** `gat-cli viz plot [OPTIONS] <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `-o`, `--output <OUTPUT>` — Optional output path for the visualization artifact



## `gat-cli analytics`

Grid analytics helpers (PTDF, sensitivities, etc.)

**Usage:** `gat-cli analytics <COMMAND>`

###### **Subcommands:**

* `ptdf` — PTDF sensitivity for a source→sink transfer
* `ds` — Deliverability Score (DS) computation for resource adequacy accreditation
* `reliability` — Reliability metrics (LOLE, EUE, thermal violations) from batch outputs
* `elcc` — Estimate Equivalent Load Carrying Capability (ELCC)



## `gat-cli analytics ptdf`

PTDF sensitivity for a source→sink transfer

**Usage:** `gat-cli analytics ptdf [OPTIONS] --source <SOURCE> --sink <SINK> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--source <SOURCE>` — Injection bus ID
* `--sink <SINK>` — Withdrawal bus ID
* `--transfer <TRANSFER>` — Transfer size in MW (defaults to 1 MW)

  Default value: `1.0`
* `-o`, `--out <OUT>` — Output file path for branch PTDF table (Parquet)

  Default value: `ptdf.parquet`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)
* `--threads <THREADS>` — Threading hint (`auto` or integer)

  Default value: `auto`
* `--solver <SOLVER>` — Solver to use (gauss, faer, etc.)

  Default value: `gauss`



## `gat-cli analytics ds`

Deliverability Score (DS) computation for resource adequacy accreditation

Computes DC-approximate deliverability scores: the fraction of nameplate capacity that can be delivered before branch thermal limits are violated. Used in RA accreditation where DS × ELCC determines effective capacity. See doi:10.1109/TPWRS.2007.899019 for DC flow.

**Usage:** `gat-cli analytics ds [OPTIONS] --limits <LIMITS> --branch-limits <BRANCH_LIMITS> --flows <FLOWS> --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--limits <LIMITS>` — CSV file with bus capacity limits (bus_id, pmax)
* `--branch-limits <BRANCH_LIMITS>` — CSV file with branch thermal limits (branch_id, flow_limit)
* `--flows <FLOWS>` — Parquet file with branch flows from DC PF/OPF (must have branch_id, flow_mw)
* `-o`, `--out <OUT>` — Output file path for DS table (Parquet)
* `--sink-bus <SINK_BUS>` — Reference/slack bus ID for PTDF computation

  Default value: `1`
* `--solver <SOLVER>` — Solver to use (gauss, faer, etc.)

  Default value: `gauss`
* `--threads <THREADS>` — Threading hint (`auto` or integer)

  Default value: `auto`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli analytics reliability`

Reliability metrics (LOLE, EUE, thermal violations) from batch outputs

Computes Loss of Load Expectation (LOLE), Energy Unserved (EUE), and thermal violation counts from batch PF/OPF results. Used for resource adequacy assessment and KPI prediction. See doi:10.1109/TPWRS.2012.2187686 for reliability metrics in power systems.

**Usage:** `gat-cli analytics reliability [OPTIONS] --out <OUT>`

###### **Options:**

* `--batch-manifest <BATCH_MANIFEST>` — Batch manifest JSON from `gat batch` (alternative to --flows)
* `--flows <FLOWS>` — Parquet file with branch flows (alternative to --batch-manifest)
* `--branch-limits <BRANCH_LIMITS>` — CSV file with branch thermal limits (branch_id, flow_limit)
* `--scenario-weights <SCENARIO_WEIGHTS>` — CSV file with scenario weights/probabilities (scenario_id, weight)
* `-o`, `--out <OUT>` — Output file path for reliability metrics table (Parquet)
* `--unserved-threshold <UNSERVED_THRESHOLD>` — Minimum unserved load to count as LOLE event (MW, default 0.1)

  Default value: `0.1`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)



## `gat-cli analytics elcc`

Estimate Equivalent Load Carrying Capability (ELCC)

**Usage:** `gat-cli analytics elcc [OPTIONS] --resource-profiles <RESOURCE_PROFILES> --reliability-metrics <RELIABILITY_METRICS> --out <OUT>`

###### **Options:**

* `--resource-profiles <RESOURCE_PROFILES>` — Parquet file with resource profiles (asset_id, class_id, time, capacity)
* `--reliability-metrics <RELIABILITY_METRICS>` — Parquet file with reliability metrics (from `gat analytics reliability`)
* `-o`, `--out <OUT>` — Output file path for ELCC estimates (Parquet)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated)
* `--max-jobs <MAX_JOBS>` — Number of parallel jobs (0 for auto)

  Default value: `0`



## `gat-cli featurize`

Feature extraction for ML models (GNN, KPI predictors, etc.)

**Usage:** `gat-cli featurize <COMMAND>`

###### **Subcommands:**

* `gnn` — Export grid topology and flows as GNN-ready graph features
* `kpi` — Generate KPI training/evaluation feature tables for ML prediction models



## `gat-cli featurize gnn`

Export grid topology and flows as GNN-ready graph features

Converts power grid data into graph-structured features for Graph Neural Networks (GNNs). Produces node features (buses with static topology + dynamic injections), edge features (branches with impedance + flows), and graph metadata. Compatible with PyTorch Geometric, DGL, and other GNN frameworks. See doi:10.1109/TPWRS.2020.3041234 for GNNs in power systems.

**Usage:** `gat-cli featurize gnn [OPTIONS] --flows <FLOWS> --out <OUT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--flows <FLOWS>` — Parquet file with branch flows from PF/OPF (must have branch_id, flow_mw)
* `-o`, `--out <OUT>` — Output directory root for feature tables (nodes, edges, graphs)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated, e.g., "graph_id,scenario_id")
* `--group-by-scenario` — Group flows by scenario_id (if present in flows)

  Default value: `true`
* `--group-by-time` — Group flows by time (if present in flows)

  Default value: `true`



## `gat-cli featurize kpi`

Generate KPI training/evaluation feature tables for ML prediction models

Aggregates batch PF/OPF outputs and reliability metrics into wide feature tables suitable for training probabilistic KPI predictors (TabNet, NGBoost, gradient boosting, etc.). Outputs are keyed by (scenario_id, time, zone) and include system stress indicators, policy flags, and reliability metrics. The "X" features for predicting reliability KPIs.

**Usage:** `gat-cli featurize kpi [OPTIONS] --batch-root <BATCH_ROOT> --out <OUT>`

###### **Options:**

* `--batch-root <BATCH_ROOT>` — Root directory containing batch PF/OPF outputs (flows, LMPs, violations)
* `--reliability <RELIABILITY>` — Optional reliability metrics file (output from `gat analytics reliability`)
* `--scenario-meta <SCENARIO_META>` — Optional scenario metadata file (YAML/JSON with policy flags, weather, etc.)
* `-o`, `--out <OUT>` — Output file path for KPI features (Parquet format)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated, e.g., "scenario_id,time")



## `gat-cli geo`

Geo-spatial tools (GIS joins, polygon mapping, spatial features)

**Usage:** `gat-cli geo <COMMAND>`

###### **Subcommands:**

* `join` — Map buses/feeders to spatial polygons (tracts, zip codes, neighborhoods)
* `featurize` — Produce time-series feature tables keyed by (polygon_id, time)



## `gat-cli geo join`

Map buses/feeders to spatial polygons (tracts, zip codes, neighborhoods)

Performs spatial joins between power grid topology (buses, feeders) and GIS polygons (census tracts, zip codes, planning areas, etc.). Produces polygon_id ↔ bus_id mapping tables for downstream spatial aggregation. Supports point-in-polygon tests, Voronoi tessellation, and k-nearest-neighbor assignment. Compatible with GeoParquet format. See doi:10.3390/ijgi9020102 for spatial joins in energy systems GIS.

**Usage:** `gat-cli geo join [OPTIONS] --grid-file <GRID_FILE> --polygons <POLYGONS> --out <OUT>`

###### **Options:**

* `--grid-file <GRID_FILE>` — Path to the grid topology file (Arrow format, must have bus_id, lat, lon)
* `--polygons <POLYGONS>` — Path to the GIS polygon file (GeoParquet, Shapefile, or GeoJSON with polygon geometries)
* `--method <METHOD>` — Spatial join method: "point_in_polygon", "voronoi", or "knn"

  Default value: `point_in_polygon`
* `--k <K>` — For knn method: number of nearest polygons to assign (default 1)

  Default value: `1`
* `-o`, `--out <OUT>` — Output file path for bus-to-polygon mapping table (Parquet: bus_id, polygon_id, distance)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated, e.g., "polygon_id")



## `gat-cli geo featurize`

Produce time-series feature tables keyed by (polygon_id, time)

Aggregates time-series grid metrics (load, voltage, violations, etc.) to spatial polygons using the bus-to-polygon mapping from `gat geo join`. Computes lags, rolling statistics, event flags, and seasonal features for spatial forecasting models. Outputs polygon-level feature fabric for demand forecasting, reliability prediction, and spatial planning. See doi:10.1016/j.energy.2020.117515 for spatial-temporal load forecasting.

**Usage:** `gat-cli geo featurize [OPTIONS] --mapping <MAPPING> --timeseries <TIMESERIES> --out <OUT>`

###### **Options:**

* `--mapping <MAPPING>` — Path to the bus-to-polygon mapping table (output from `gat geo join`)
* `--timeseries <TIMESERIES>` — Path to time-series grid metrics (Parquet with bus_id, time, values)
* `--lags <LAGS>` — Lag periods to compute (comma separated, e.g., "1,7,24" for 1-hour, 7-hour, 24-hour lags)
* `--windows <WINDOWS>` — Rolling window sizes (comma separated, e.g., "7,24,168" for 7h, 24h, 168h windows)
* `--seasonal` — Compute seasonal features (day-of-week, hour-of-day, month-of-year flags)

  Default value: `true`
* `-o`, `--out <OUT>` — Output file path for polygon-level features (Parquet: polygon_id, time, features)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated, e.g., "polygon_id,time")



## `gat-cli alloc`

Allocation and settlement tools (congestion rents, cost attribution)

**Usage:** `gat-cli alloc <COMMAND>`

###### **Subcommands:**

* `rents` — Compute congestion rents and surplus decomposition from OPF results
* `kpi` — Simple contribution analysis for KPI changes across scenarios



## `gat-cli alloc rents`

Compute congestion rents and surplus decomposition from OPF results

Analyzes OPF outputs (LMPs, flows, injections) to decompose system surplus into congestion rents, generator revenues, and load payments. Provides the numerical backbone for allocation and settlement frameworks. See doi:10.1109/TPWRS.2003.820692 for LMP-based congestion analysis.

**Usage:** `gat-cli alloc rents [OPTIONS] --opf-results <OPF_RESULTS> --grid-file <GRID_FILE> --out <OUT>`

###### **Options:**

* `--opf-results <OPF_RESULTS>` — Parquet file with OPF results (must have: bus_id, lmp, injection_mw, flow_mw)
* `--grid-file <GRID_FILE>` — Path to the grid topology file (Arrow format, for branch mapping)
* `--tariffs <TARIFFS>` — Optional tariff/margin parameters CSV (resource_id, tariff_rate)
* `-o`, `--out <OUT>` — Output file path for congestion rents table (Parquet)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated, e.g., "scenario_id,time")



## `gat-cli alloc kpi`

Simple contribution analysis for KPI changes across scenarios

Approximates the contribution of control actions/portfolios to KPI improvements using gradient-based sensitivity or linear approximations. A stepping stone towards full SHAP explainability. See doi:10.1038/s42256-019-0138-9 for SHAP and model explanations.

**Usage:** `gat-cli alloc kpi [OPTIONS] --kpi-results <KPI_RESULTS> --scenario-meta <SCENARIO_META> --out <OUT>`

###### **Options:**

* `--kpi-results <KPI_RESULTS>` — Parquet file with KPI results (must have: scenario_id, kpi_value)
* `--scenario-meta <SCENARIO_META>` — Parquet file with scenario metadata (scenario_id, control flags, policy settings)
* `-o`, `--out <OUT>` — Output file path for contribution analysis (Parquet)
* `--out-partitions <OUT_PARTITIONS>` — Partition columns (comma separated, e.g., "scenario_id")



## `gat-cli batch`

Scenario batch runners for PF/OPF (CANOS-style fan-out)

**Usage:** `gat-cli batch <COMMAND>`

###### **Subcommands:**

* `pf` — Run DC/AC PF for every scenario in a manifest (CANOS-style fan-out, doi:10.1109/TPWRS.2007.899019)
* `opf` — Run DC/AC OPF for every scenario (CANOS-ready reliability stats)



## `gat-cli batch pf`

Run DC/AC PF for every scenario in a manifest (CANOS-style fan-out, doi:10.1109/TPWRS.2007.899019)

**Usage:** `gat-cli batch pf [OPTIONS] --manifest <MANIFEST> --out <OUT>`

###### **Options:**

* `--manifest <MANIFEST>` — Scenario manifest JSON generated by `gat scenarios materialize`
* `-o`, `--out <OUT>` — Output directory root for job outputs
* `--mode <MODE>` — Flow mode (`dc` or `ac`)

  Default value: `dc`
* `--solver <SOLVER>` — Linear solver (`gauss`/`faer`, etc.)

  Default value: `gauss`
* `--threads <THREADS>` — Threading hint for global Rayon pool

  Default value: `auto`
* `--max-jobs <MAX_JOBS>` — Maximum number of jobs to execute in parallel (0 = auto)

  Default value: `0`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns for Parquet outputs (optional)
* `--tol <TOL>` — AC tolerance in per unit

  Default value: `0.000001`
* `--max-iter <MAX_ITER>` — Maximum AC solver iterations

  Default value: `50`



## `gat-cli batch opf`

Run DC/AC OPF for every scenario (CANOS-ready reliability stats)

**Usage:** `gat-cli batch opf [OPTIONS] --manifest <MANIFEST> --out <OUT>`

###### **Options:**

* `--manifest <MANIFEST>` — Scenario manifest JSON
* `-o`, `--out <OUT>` — Output directory root
* `--mode <MODE>` — OPF mode (`dc` or `ac`)

  Default value: `dc`
* `--solver <SOLVER>` — Main solver (gauss/faer)

  Default value: `gauss`
* `--lp-solver <LP_SOLVER>` — LP solver for DC OPF

  Default value: `clarabel`
* `--cost <COST>` — Cost CSV (required for DC OPF)

  Default value: ``
* `--limits <LIMITS>` — Limits CSV (required for DC OPF)

  Default value: ``
* `--branch-limits <BRANCH_LIMITS>` — Optional branch limits
* `--piecewise <PIECEWISE>` — Optional piecewise cost segments
* `--threads <THREADS>` — Threading hint for global pool

  Default value: `auto`
* `--max-jobs <MAX_JOBS>` — Maximum concurrent OPF jobs (0 = auto)

  Default value: `0`
* `--out-partitions <OUT_PARTITIONS>` — Partition columns for Parquet outputs
* `--tol <TOL>` — Iteration tolerance

  Default value: `0.000001`
* `--max-iter <MAX_ITER>` — Maximum iterations

  Default value: `50`



## `gat-cli benchmark`

Benchmarking suites for OPF/PF solvers

**Usage:** `gat-cli benchmark <COMMAND>`

###### **Subcommands:**

* `pfdelta` — Run PFDelta AC OPF benchmark suite
* `pglib` — Run PGLib-OPF benchmark suite (MATPOWER format)
* `opfdata` — Run OPFData benchmark suite (GNN-format JSON)



## `gat-cli benchmark pfdelta`

Run PFDelta AC OPF benchmark suite

**Usage:** `gat-cli benchmark pfdelta [OPTIONS] --pfdelta-root <PFDELTA_ROOT> --out <OUT>`

###### **Options:**

* `--pfdelta-root <PFDELTA_ROOT>` — Root directory containing PFDelta dataset
* `--case <CASE>` — Specific test case to benchmark (14, 30, 57, 118, 500, 2000)
* `--contingency <CONTINGENCY>` — Contingency type to run (n, n-1, n-2, or all)

  Default value: `all`
* `--max-cases <MAX_CASES>` — Maximum number of test cases to run (0 = all)

  Default value: `0`
* `-o`, `--out <OUT>` — Output CSV path for results
* `--threads <THREADS>` — Number of parallel solver threads (auto = CPU count)

  Default value: `auto`
* `--mode <MODE>` — Solve mode: pf (power flow) or opf (optimal power flow)

  Default value: `opf`
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum AC solver iterations

  Default value: `20`



## `gat-cli benchmark pglib`

Run PGLib-OPF benchmark suite (MATPOWER format)

**Usage:** `gat-cli benchmark pglib [OPTIONS] --pglib-dir <PGLIB_DIR> --out <OUT>`

###### **Options:**

* `--pglib-dir <PGLIB_DIR>` — Directory containing PGLib MATPOWER (.m) files
* `--baseline <BASELINE>` — Optional baseline CSV for objective comparison
* `--case-filter <CASE_FILTER>` — Filter cases by name pattern (e.g., "case14", "case118")
* `--max-cases <MAX_CASES>` — Maximum number of cases to run (0 = all)

  Default value: `0`
* `-o`, `--out <OUT>` — Output CSV path for results
* `--threads <THREADS>` — Number of parallel solver threads (auto = CPU count)

  Default value: `auto`
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum AC solver iterations

  Default value: `20`



## `gat-cli benchmark opfdata`

Run OPFData benchmark suite (GNN-format JSON)

**Usage:** `gat-cli benchmark opfdata [OPTIONS] --opfdata-dir <OPFDATA_DIR> --out <OUT>`

###### **Options:**

* `--opfdata-dir <OPFDATA_DIR>` — Directory containing OPFData JSON files
* `--case-filter <CASE_FILTER>` — Filter samples by file path pattern
* `--max-cases <MAX_CASES>` — Maximum number of samples to run (0 = all)

  Default value: `0`
* `-o`, `--out <OUT>` — Output CSV path for results
* `--threads <THREADS>` — Number of parallel solver threads (auto = CPU count)

  Default value: `auto`
* `--tol <TOL>` — Convergence tolerance

  Default value: `1e-6`
* `--max-iter <MAX_ITER>` — Maximum AC solver iterations

  Default value: `20`



## `gat-cli runs`

Run management

**Usage:** `gat-cli runs <COMMAND>`

###### **Subcommands:**

* `list` — List recorded runs
* `describe` — Describe a recorded run
* `resume` — Resume a long run from a manifest



## `gat-cli runs list`

List recorded runs

**Usage:** `gat-cli runs list [OPTIONS]`

###### **Options:**

* `--root <ROOT>` — Root path(s) to scan for run manifests

  Default value: `.`
* `--format <FORMAT>` — Output format for the listing

  Default value: `plain`

  Possible values: `plain`, `json`




## `gat-cli runs describe`

Describe a recorded run

**Usage:** `gat-cli runs describe [OPTIONS] <TARGET>`

###### **Arguments:**

* `<TARGET>` — Manifest path or run_id alias

###### **Options:**

* `--root <ROOT>` — Root path where manifests are scanned (used when target is a run_id)

  Default value: `.`
* `--format <FORMAT>` — Output format for the description

  Default value: `plain`

  Possible values: `plain`, `json`




## `gat-cli runs resume`

Resume a long run from a manifest

**Usage:** `gat-cli runs resume [OPTIONS] <MANIFEST>`

###### **Arguments:**

* `<MANIFEST>` — Manifest JSON path or run_id alias

###### **Options:**

* `--root <ROOT>` — Root path where manifests are scanned (used when the manifest argument is a run_id)

  Default value: `.`
* `--execute` — Actually re-run the command recorded in the manifest



## `gat-cli dataset`

Dataset adapters

**Usage:** `gat-cli dataset <COMMAND>`

###### **Subcommands:**

* `rts-gmlc` — RTS-GMLC helpers
* `hiren` — HIREN test cases
* `dsgrid` — Import dsgrid Parquet bundle
* `sup3rcc` — Sup3rCC weather helpers
* `pras` — PRAS outputs
* `public` — Public dataset catalog
* `eia` — Download U.S. generator data from EIA
* `ember` — Download carbon intensity and renewable data from Ember



## `gat-cli dataset rts-gmlc`

RTS-GMLC helpers

**Usage:** `gat-cli dataset rts-gmlc <COMMAND>`

###### **Subcommands:**

* `fetch` — Fetch release copy



## `gat-cli dataset rts-gmlc fetch`

Fetch release copy

**Usage:** `gat-cli dataset rts-gmlc fetch [OPTIONS]`

###### **Options:**

* `-o`, `--out <OUT>`

  Default value: `data/rts-gmlc`
* `--tag <TAG>`



## `gat-cli dataset hiren`

HIREN test cases

**Usage:** `gat-cli dataset hiren <COMMAND>`

###### **Subcommands:**

* `list` — List cases
* `fetch` — Fetch a case



## `gat-cli dataset hiren list`

List cases

**Usage:** `gat-cli dataset hiren list`



## `gat-cli dataset hiren fetch`

Fetch a case

**Usage:** `gat-cli dataset hiren fetch [OPTIONS] <CASE>`

###### **Arguments:**

* `<CASE>`

###### **Options:**

* `-o`, `--out <OUT>`

  Default value: `data/hiren`



## `gat-cli dataset dsgrid`

Import dsgrid Parquet bundle

**Usage:** `gat-cli dataset dsgrid --out <OUT>`

###### **Options:**

* `-o`, `--out <OUT>`



## `gat-cli dataset sup3rcc`

Sup3rCC weather helpers

**Usage:** `gat-cli dataset sup3rcc <COMMAND>`

###### **Subcommands:**

* `fetch` — Fetch Parquet
* `sample` — Sample for a grid



## `gat-cli dataset sup3rcc fetch`

Fetch Parquet

**Usage:** `gat-cli dataset sup3rcc fetch --out <OUT>`

###### **Options:**

* `-o`, `--out <OUT>`



## `gat-cli dataset sup3rcc sample`

Sample for a grid

**Usage:** `gat-cli dataset sup3rcc sample --out <OUT> <GRID>`

###### **Arguments:**

* `<GRID>`

###### **Options:**

* `-o`, `--out <OUT>`



## `gat-cli dataset pras`

PRAS outputs

**Usage:** `gat-cli dataset pras --out <OUT> <PATH>`

###### **Arguments:**

* `<PATH>` — Path to PRAS directory or file

###### **Options:**

* `-o`, `--out <OUT>`



## `gat-cli dataset public`

Public dataset catalog

**Usage:** `gat-cli dataset public <COMMAND>`

###### **Subcommands:**

* `list` — List curated public datasets we know how to fetch
* `describe` — Show metadata about a curated dataset
* `fetch` — Fetch a curated dataset by ID



## `gat-cli dataset public list`

List curated public datasets we know how to fetch

**Usage:** `gat-cli dataset public list [OPTIONS]`

###### **Options:**

* `--tag <TAG>` — Filter datasets by tag
* `--query <QUERY>` — Search term that matches dataset id or description



## `gat-cli dataset public describe`

Show metadata about a curated dataset

**Usage:** `gat-cli dataset public describe <ID>`

###### **Arguments:**

* `<ID>` — Dataset ID (see `gat dataset public list`)



## `gat-cli dataset public fetch`

Fetch a curated dataset by ID

**Usage:** `gat-cli dataset public fetch [OPTIONS] <ID>`

###### **Arguments:**

* `<ID>` — Dataset ID (see `gat dataset public list`)

###### **Options:**

* `-o`, `--out <OUT>` — Directory to stage the download (defaults to ~/.cache/gat/datasets or data/public)
* `--force` — Force re-download if the file already exists
* `--extract` — Try to extract the dataset if it's a zip archive



## `gat-cli dataset eia`

Download U.S. generator data from EIA

**Usage:** `gat-cli dataset eia --api-key <API_KEY> --output <OUTPUT>`

###### **Options:**

* `--api-key <API_KEY>` — EIA API key
* `-o`, `--output <OUTPUT>` — Output file path (supports .csv, .parquet)



## `gat-cli dataset ember`

Download carbon intensity and renewable data from Ember

**Usage:** `gat-cli dataset ember --region <REGION> --start-date <START_DATE> --end-date <END_DATE> --output <OUTPUT>`

###### **Options:**

* `--region <REGION>` — Region code (e.g., "US-West", "GB", "DE")
* `--start-date <START_DATE>` — Start date in YYYY-MM-DD format
* `--end-date <END_DATE>` — End date in YYYY-MM-DD format
* `-o`, `--output <OUTPUT>` — Output file path (supports .csv, .parquet)



## `gat-cli version`

Release version helpers

**Usage:** `gat-cli version <COMMAND>`

###### **Subcommands:**

* `sync` — Sync release metadata



## `gat-cli version sync`

Sync release metadata

**Usage:** `gat-cli version sync [OPTIONS]`

###### **Options:**

* `--tag <TAG>` — Tag name to validate (leading `v` is stripped)
* `--manifest <MANIFEST>` — Write manifest JSON describing the resolved version/tag



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
