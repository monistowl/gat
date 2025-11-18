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
* [`gat-cli pf`↴](#gat-cli-pf)
* [`gat-cli pf dc`↴](#gat-cli-pf-dc)
* [`gat-cli pf ac`↴](#gat-cli-pf-ac)
* [`gat-cli nminus1`↴](#gat-cli-nminus1)
* [`gat-cli nminus1 dc`↴](#gat-cli-nminus1-dc)
* [`gat-cli ts`↴](#gat-cli-ts)
* [`gat-cli ts resample`↴](#gat-cli-ts-resample)
* [`gat-cli ts join`↴](#gat-cli-ts-join)
* [`gat-cli ts agg`↴](#gat-cli-ts-agg)
* [`gat-cli opf`↴](#gat-cli-opf)
* [`gat-cli opf dc`↴](#gat-cli-opf-dc)
* [`gat-cli opf ac`↴](#gat-cli-opf-ac)
* [`gat-cli se`↴](#gat-cli-se)
* [`gat-cli se wls`↴](#gat-cli-se-wls)
* [`gat-cli viz`↴](#gat-cli-viz)
* [`gat-cli viz plot`↴](#gat-cli-viz-plot)
* [`gat-cli gui`↴](#gat-cli-gui)
* [`gat-cli gui run`↴](#gat-cli-gui-run)
* [`gat-cli runs`↴](#gat-cli-runs)
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

## `gat-cli`

**Usage:** `gat-cli [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `import` — Import data from various formats
* `validate` — Validate a dataset against a schema
* `graph` — Graph utilities
* `pf` — Power flow solvers
* `nminus1` — Contingency analysis
* `ts` — Time-series utilities
* `opf` — Optimal power flow
* `se` — State estimation
* `viz` — Visualization helpers
* `gui` — GUI dashboard
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

**Usage:** `gat-cli graph export --format <FORMAT> <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `--format <FORMAT>` — Output format (e.g., graphviz)



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



## `gat-cli gui`

GUI dashboard

**Usage:** `gat-cli gui <COMMAND>`

###### **Subcommands:**

* `run` — Launch the GUI dashboard (placeholder)



## `gat-cli gui run`

Launch the GUI dashboard (placeholder)

**Usage:** `gat-cli gui run [OPTIONS] <GRID_FILE>`

###### **Arguments:**

* `<GRID_FILE>` — Path to the grid data file (Arrow format)

###### **Options:**

* `-o`, `--output <OUTPUT>` — Optional path to persist the visualization artifact



## `gat-cli runs`

Run management

**Usage:** `gat-cli runs <COMMAND>`

###### **Subcommands:**

* `resume` — Resume a long run from a manifest



## `gat-cli runs resume`

Resume a long run from a manifest

**Usage:** `gat-cli runs resume [OPTIONS] <MANIFEST>`

###### **Arguments:**

* `<MANIFEST>` — Manifest JSON path

###### **Options:**

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



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
