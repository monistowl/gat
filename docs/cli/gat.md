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

## `gat-cli`

**Usage:** `gat-cli [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `import` — Import data from various formats
* `validate` — Validate a dataset against a schema
* `graph` — Graph utilities
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
* `runs` — Run management
* `dataset` — Dataset adapters

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



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
