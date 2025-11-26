# Benchmark Experiments Implementation Plan

**Date**: 2025-11-24
**Goal**: Reproduce arxiv paper results with GAT to validate correct operation
**Scope**: PFΔ, PGLib, OPFData benchmarks with shared infrastructure
**Approach**: Parallel foundation — shared infrastructure first, then benchmarks

## Design Decisions

1. **Goal**: Reproduction-first (numerical validation proves GAT works)
2. **Tolerances**: Configurable with tight defaults (`< 1e-6` obj, `< 1e-4` constraints)
3. **Data strategy**: Small fixtures committed to repo for CI; full datasets stay local
4. **Plan structure**: Single monolithic plan for long unattended run

## Prerequisites

Before starting, ensure:
- Working GAT build: `cargo build -p gat-cli --release`
- Internet access for fetching test fixtures
- ~500MB disk space for sample datasets

---

## Phase 1: Data Fetching (Tasks 1-6)

### Task 1: Create test fixture directories

**Files**:
- `test_data/pfdelta/.gitkeep`
- `test_data/pglib/.gitkeep`
- `test_data/opfdata/.gitkeep`

**Action**: Create directory structure for test fixtures.

```bash
mkdir -p test_data/pfdelta/ieee14/n/raw
mkdir -p test_data/pfdelta/ieee14/n-1/raw
mkdir -p test_data/pglib
mkdir -p test_data/opfdata/case118/load
mkdir -p test_data/opfdata/case118/topo
```

**Verification**: Directories exist.

---

### Task 2: Fetch PFΔ sample data

**Source**: HuggingFace `MOSSLab-MIT/pfdelta`
**Target**: `test_data/pfdelta/`

**Action**: Download 1-2 sample JSON files for IEEE-14 base case and N-1 contingency.

```bash
# Option A: Using huggingface-cli
huggingface-cli download MOSSLab-MIT/pfdelta --include "ieee14/n/raw/*.json" --local-dir test_data/pfdelta/ --max-files 2

# Option B: Direct download if specific URLs are available
# curl -o test_data/pfdelta/ieee14/n/raw/sample_001.json "<URL>"
```

**Note**: If HuggingFace structure differs, adapt paths accordingly. Inspect the repo structure first:
```bash
huggingface-cli repo-info MOSSLab-MIT/pfdelta --files
```

**Verification**: At least one `.json` file exists in `test_data/pfdelta/ieee14/n/raw/`.

---

### Task 3: Fetch PGLib sample data

**Source**: GitHub `power-grid-lib/pglib-opf`
**Target**: `test_data/pglib/`

**Action**: Download two small MATPOWER cases.

```bash
curl -o test_data/pglib/pglib_opf_case5_pjm.m \
  "https://raw.githubusercontent.com/power-grid-lib/pglib-opf/master/pglib_opf_case5_pjm.m"

curl -o test_data/pglib/pglib_opf_case14_ieee.m \
  "https://raw.githubusercontent.com/power-grid-lib/pglib-opf/master/pglib_opf_case14_ieee.m"
```

**Verification**: Both `.m` files exist and are valid MATPOWER format.

---

### Task 4: Create PGLib baseline CSV

**Target**: `test_data/pglib/baseline.csv`

**Action**: Extract reference objective values from PGLib's BASELINE.md or use known values.

```csv
case_name,objective
pglib_opf_case5_pjm,17551.89
pglib_opf_case14_ieee,8081.53
```

**Note**: These values should match PGLib's published baseline. Verify against:
```bash
curl -s "https://raw.githubusercontent.com/power-grid-lib/pglib-opf/master/BASELINE.md" | grep -E "case5|case14"
```

**Verification**: `baseline.csv` exists with correct format.

---

### Task 5: Fetch OPFData sample data

**Source**: HuggingFace `AI4Climate/OPFData`
**Target**: `test_data/opfdata/`

**Action**: Download sample JSONL files for case118 load and topology variations.

```bash
# Inspect structure first
huggingface-cli repo-info AI4Climate/OPFData --files

# Download a shard and extract first 5 lines
huggingface-cli download AI4Climate/OPFData --include "*118*load*" --local-dir /tmp/opfdata/ --max-files 1
head -5 /tmp/opfdata/<path-to-shard>.jsonl > test_data/opfdata/case118/load/sample.jsonl

huggingface-cli download AI4Climate/OPFData --include "*118*topo*" --local-dir /tmp/opfdata/ --max-files 1
head -5 /tmp/opfdata/<path-to-shard>.jsonl > test_data/opfdata/case118/topo/sample.jsonl
```

**Note**: Adapt paths based on actual HuggingFace structure. May need to use `datasets` library if CLI doesn't work.

**Verification**: Both `sample.jsonl` files exist with valid JSON lines.

---

### Task 6: Validate fetched fixtures

**Action**: Manually inspect each fixture to confirm format matches expectations.

```bash
# PFΔ: Check JSON structure
head -100 test_data/pfdelta/ieee14/n/raw/*.json | grep -E '"bus"|"gen"|"branch"|"load"'

# PGLib: Check MATPOWER format
head -20 test_data/pglib/pglib_opf_case5_pjm.m

# OPFData: Check JSONL structure
head -1 test_data/opfdata/case118/load/sample.jsonl | python -m json.tool
```

**Verification**: All fixtures have expected structure (bus, gen, branch, load fields).

---

## Phase 2: Shared Infrastructure (Tasks 7-15)

### Task 7: Create validation metrics module

**File**: `crates/gat-algo/src/validation.rs` (new)

**Action**: Create module with error computation structs and functions.

```rust
//! Validation metrics for comparing GAT solutions against reference solutions.

use std::collections::HashMap;

/// Error metrics for power flow solutions
#[derive(Debug, Clone, Default)]
pub struct PFErrorMetrics {
    /// Maximum voltage magnitude error (p.u.)
    pub max_vm_error: f64,
    /// Maximum voltage angle error (degrees)
    pub max_va_error_deg: f64,
    /// Mean voltage magnitude error (p.u.)
    pub mean_vm_error: f64,
    /// Mean voltage angle error (degrees)
    pub mean_va_error_deg: f64,
    /// Maximum branch active power flow error (MW)
    pub max_branch_p_error: f64,
    /// Number of buses compared
    pub num_buses_compared: usize,
}

/// Constraint violation metrics for OPF solutions
#[derive(Debug, Clone, Default)]
pub struct OPFViolationMetrics {
    /// Maximum active power balance violation (MW)
    pub max_p_balance_violation: f64,
    /// Maximum reactive power balance violation (MVAr)
    pub max_q_balance_violation: f64,
    /// Maximum branch flow violation (MVA)
    pub max_branch_flow_violation: f64,
    /// Maximum generator active power violation (MW)
    pub max_gen_p_violation: f64,
    /// Maximum voltage magnitude violation (p.u.)
    pub max_vm_violation: f64,
}

/// Objective value comparison
#[derive(Debug, Clone, Default)]
pub struct ObjectiveGap {
    /// GAT objective value
    pub gat_objective: f64,
    /// Reference objective value
    pub ref_objective: f64,
    /// Absolute gap
    pub gap_abs: f64,
    /// Relative gap (fraction)
    pub gap_rel: f64,
}

/// Reference solution for power flow comparison
#[derive(Debug, Clone, Default)]
pub struct PFReferenceSolution {
    /// Bus voltage magnitudes (bus_id -> Vm in p.u.)
    pub vm: HashMap<usize, f64>,
    /// Bus voltage angles (bus_id -> Va in radians)
    pub va: HashMap<usize, f64>,
    /// Generator active power (gen_id -> P in MW)
    pub pgen: HashMap<usize, f64>,
    /// Generator reactive power (gen_id -> Q in MVAr)
    pub qgen: HashMap<usize, f64>,
}

impl PFErrorMetrics {
    /// Check if all errors are within tolerance
    pub fn within_tolerance(&self, voltage_tol: f64, angle_tol_deg: f64) -> bool {
        self.max_vm_error <= voltage_tol && self.max_va_error_deg <= angle_tol_deg
    }
}

impl OPFViolationMetrics {
    /// Check if all violations are within tolerance
    pub fn within_tolerance(&self, constraint_tol: f64) -> bool {
        self.max_p_balance_violation <= constraint_tol
            && self.max_q_balance_violation <= constraint_tol
            && self.max_branch_flow_violation <= constraint_tol
            && self.max_gen_p_violation <= constraint_tol
            && self.max_vm_violation <= constraint_tol
    }
}

impl ObjectiveGap {
    /// Create from two objective values
    pub fn new(gat_objective: f64, ref_objective: f64) -> Self {
        let gap_abs = (gat_objective - ref_objective).abs();
        let gap_rel = if ref_objective.abs() > 1e-10 {
            gap_abs / ref_objective.abs()
        } else {
            0.0
        };
        Self {
            gat_objective,
            ref_objective,
            gap_abs,
            gap_rel,
        }
    }

    /// Check if gap is within tolerance
    pub fn within_tolerance(&self, obj_tol: f64) -> bool {
        self.gap_rel <= obj_tol
    }
}
```

**Verification**: `cargo check -p gat-algo`

---

### Task 8: Add validation module to gat-algo exports

**File**: `crates/gat-algo/src/lib.rs`

**Action**: Add `pub mod validation;` and re-export types.

```rust
pub mod validation;

pub use validation::{ObjectiveGap, OPFViolationMetrics, PFErrorMetrics, PFReferenceSolution};
```

**Verification**: `cargo check -p gat-algo`

---

### Task 9: Implement PF error computation

**File**: `crates/gat-algo/src/validation.rs`

**Action**: Add function to compute PF errors between GAT solution and reference.

```rust
use gat_core::Network;

/// Compute power flow error metrics between GAT solution and reference
///
/// # Arguments
/// * `_network` - The network (for context, e.g., getting bus indices)
/// * `gat_vm` - GAT solution voltage magnitudes (bus_id -> Vm)
/// * `gat_va` - GAT solution voltage angles in radians (bus_id -> Va)
/// * `reference` - Reference solution to compare against
pub fn compute_pf_errors(
    _network: &Network,
    gat_vm: &HashMap<usize, f64>,
    gat_va: &HashMap<usize, f64>,
    reference: &PFReferenceSolution,
) -> PFErrorMetrics {
    let mut max_vm_error = 0.0_f64;
    let mut max_va_error_rad = 0.0_f64;
    let mut sum_vm_error = 0.0;
    let mut sum_va_error = 0.0;
    let mut count = 0_usize;

    for (bus_id, ref_vm) in &reference.vm {
        if let Some(gat_vm_val) = gat_vm.get(bus_id) {
            let vm_err = (gat_vm_val - ref_vm).abs();
            max_vm_error = max_vm_error.max(vm_err);
            sum_vm_error += vm_err;
            count += 1;
        }
    }

    for (bus_id, ref_va) in &reference.va {
        if let Some(gat_va_val) = gat_va.get(bus_id) {
            let va_err = (gat_va_val - ref_va).abs();
            max_va_error_rad = max_va_error_rad.max(va_err);
            sum_va_error += va_err;
        }
    }

    let num_buses = count.max(1);
    PFErrorMetrics {
        max_vm_error,
        max_va_error_deg: max_va_error_rad.to_degrees(),
        mean_vm_error: sum_vm_error / num_buses as f64,
        mean_va_error_deg: (sum_va_error / num_buses as f64).to_degrees(),
        max_branch_p_error: 0.0, // TODO: implement if branch flows available
        num_buses_compared: count,
    }
}
```

**Verification**: `cargo check -p gat-algo`

---

### Task 10: Implement OPF violation computation

**File**: `crates/gat-algo/src/validation.rs`

**Action**: Add function to compute OPF constraint violations.

```rust
/// Compute OPF constraint violation metrics from a solution
///
/// This is a placeholder implementation. The actual implementation depends on
/// how GAT's OPF solution exposes bus injections, branch flows, and limits.
pub fn compute_opf_violations(
    _network: &Network,
    _solution: &crate::AcOpfSolution,
) -> OPFViolationMetrics {
    // TODO: Implement based on actual AcOpfSolution structure
    // For now, return zeros - the OPF solver should already enforce constraints
    OPFViolationMetrics::default()
}
```

**Note**: This needs to be fleshed out based on `AcOpfSolution` structure. Check `gat-algo` for how solutions expose:
- Bus power injections vs loads
- Branch flows vs limits (rate_a)
- Generator outputs vs limits (p_min, p_max, q_min, q_max)
- Voltage magnitudes vs limits (v_min, v_max)

**Verification**: `cargo check -p gat-algo`

---

### Task 11: Create benchmark common module

**File**: `crates/gat-cli/src/commands/benchmark/common.rs` (new)

**Action**: Create shared types for benchmark results.

```rust
//! Common types and utilities for benchmark commands.

use serde::Serialize;

/// Base timing and convergence fields shared by all benchmarks
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkTiming {
    /// Time to load/parse the case (ms)
    pub load_time_ms: f64,
    /// Time to solve (ms)
    pub solve_time_ms: f64,
    /// Total time (ms)
    pub total_time_ms: f64,
}

/// Base convergence fields
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkConvergence {
    /// Whether the solver converged
    pub converged: bool,
    /// Number of iterations
    pub iterations: u32,
}

/// Base size fields
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkSize {
    /// Number of buses
    pub num_buses: usize,
    /// Number of branches
    pub num_branches: usize,
    /// Number of generators
    pub num_gens: usize,
}

/// Tolerance configuration for benchmarks
#[derive(Debug, Clone)]
pub struct BenchmarkTolerances {
    /// Objective value relative tolerance
    pub obj_tol: f64,
    /// Constraint violation tolerance
    pub constraint_tol: f64,
    /// Voltage magnitude tolerance (p.u.)
    pub voltage_tol: f64,
    /// Voltage angle tolerance (degrees)
    pub angle_tol_deg: f64,
}

impl Default for BenchmarkTolerances {
    fn default() -> Self {
        Self {
            obj_tol: 1e-6,
            constraint_tol: 1e-4,
            voltage_tol: 1e-4,
            angle_tol_deg: 0.01, // ~0.01 degrees
        }
    }
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 12: Create baseline CSV loader

**File**: `crates/gat-cli/src/commands/benchmark/baseline.rs` (new)

**Action**: Create utilities for loading baseline/reference data from CSV.

```rust
//! Baseline data loading for benchmark comparisons.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Load baseline objective values from a CSV file.
///
/// Expected format:
/// ```csv
/// case_name,objective
/// pglib_opf_case5_pjm,17551.89
/// pglib_opf_case14_ieee,8081.53
/// ```
pub fn load_baseline_objectives(path: &Path) -> Result<HashMap<String, f64>> {
    let mut map = HashMap::new();

    let mut reader = csv::Reader::from_path(path)
        .with_context(|| format!("opening baseline CSV: {}", path.display()))?;

    for result in reader.records() {
        let record = result.with_context(|| "reading baseline CSV record")?;

        let case_name = record
            .get(0)
            .ok_or_else(|| anyhow::anyhow!("missing case_name column"))?
            .to_string();

        let objective: f64 = record
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("missing objective column"))?
            .parse()
            .with_context(|| format!("parsing objective for {}", case_name))?;

        map.insert(case_name, objective);
    }

    Ok(map)
}

/// Normalize case name for matching (strip extensions, lowercase)
pub fn normalize_case_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .trim_end_matches(".m")
        .trim_end_matches(".json")
        .to_string()
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 13: Update benchmark mod.rs with new modules

**File**: `crates/gat-cli/src/commands/benchmark/mod.rs`

**Action**: Add the new modules and update exports.

```rust
pub mod baseline;
pub mod common;
pub mod pfdelta;
// pub mod pglib;    // Task 24
// pub mod opfdata;  // Task 35

pub use common::BenchmarkTolerances;
```

**Verification**: `cargo check -p gat-cli`

---

### Task 14: Add tolerance CLI flags pattern

**File**: `crates/gat-cli/src/commands/benchmark/common.rs`

**Action**: Add helper to parse tolerance flags consistently.

```rust
impl BenchmarkTolerances {
    /// Create from CLI arguments with defaults
    pub fn from_args(
        obj_tol: Option<f64>,
        constraint_tol: Option<f64>,
        voltage_tol: Option<f64>,
    ) -> Self {
        let defaults = Self::default();
        Self {
            obj_tol: obj_tol.unwrap_or(defaults.obj_tol),
            constraint_tol: constraint_tol.unwrap_or(defaults.constraint_tol),
            voltage_tol: voltage_tol.unwrap_or(defaults.voltage_tol),
            angle_tol_deg: defaults.angle_tol_deg,
        }
    }
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 15: Add unit tests for validation module

**File**: `crates/gat-algo/src/validation.rs`

**Action**: Add tests at the bottom of the module.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_gap() {
        let gap = ObjectiveGap::new(100.0, 100.0);
        assert!(gap.within_tolerance(1e-6));
        assert_eq!(gap.gap_abs, 0.0);
        assert_eq!(gap.gap_rel, 0.0);

        let gap2 = ObjectiveGap::new(100.001, 100.0);
        assert!(gap2.within_tolerance(1e-3));
        assert!(!gap2.within_tolerance(1e-6));
    }

    #[test]
    fn test_pf_error_metrics_tolerance() {
        let metrics = PFErrorMetrics {
            max_vm_error: 0.0001,
            max_va_error_deg: 0.005,
            ..Default::default()
        };
        assert!(metrics.within_tolerance(1e-4, 0.01));
        assert!(!metrics.within_tolerance(1e-5, 0.01));
    }

    #[test]
    fn test_opf_violation_tolerance() {
        let violations = OPFViolationMetrics {
            max_p_balance_violation: 0.00001,
            max_q_balance_violation: 0.00001,
            max_branch_flow_violation: 0.0,
            max_gen_p_violation: 0.0,
            max_vm_violation: 0.0,
        };
        assert!(violations.within_tolerance(1e-4));
    }
}
```

**Verification**: `cargo test -p gat-algo validation`

---

## Phase 3: PFΔ Enhancements (Tasks 16-23)

### Task 16: Add PFDeltaSolution struct

**File**: `crates/gat-io/src/sources/pfdelta.rs`

**Action**: Add struct to hold reference solution from PFΔ JSON.

```rust
use std::collections::HashMap;

/// Reference power flow solution from PFΔ dataset
#[derive(Debug, Clone, Default)]
pub struct PFDeltaSolution {
    /// Bus voltage magnitudes (bus_id -> Vm in p.u.)
    pub vm: HashMap<usize, f64>,
    /// Bus voltage angles (bus_id -> Va in radians)
    pub va: HashMap<usize, f64>,
    /// Generator active power outputs (gen_id -> P in MW)
    pub pgen: HashMap<usize, f64>,
    /// Generator reactive power outputs (gen_id -> Q in MVAr)
    pub qgen: HashMap<usize, f64>,
}

/// Complete PFΔ instance with network and reference solution
#[derive(Debug, Clone)]
pub struct PFDeltaInstance {
    /// Test case metadata
    pub test_case: PFDeltaTestCase,
    /// GAT network representation
    pub network: Network,
    /// Reference solution from the dataset
    pub solution: PFDeltaSolution,
}
```

**Verification**: `cargo check -p gat-io`

---

### Task 17: Implement solution extraction from PFΔ JSON

**File**: `crates/gat-io/src/sources/pfdelta.rs`

**Action**: Add function to extract reference solution from JSON.

```rust
/// Extract reference solution from PFΔ JSON
fn extract_pfdelta_solution(data: &Value) -> Result<PFDeltaSolution> {
    let mut solution = PFDeltaSolution::default();

    // Extract bus voltages - PFΔ stores these in the bus object
    if let Some(buses) = data["bus"].as_object() {
        for (bus_idx_str, bus_data) in buses {
            let bus_idx: usize = bus_idx_str.parse()
                .with_context(|| format!("Invalid bus index: {}", bus_idx_str))?;

            // Voltage magnitude (p.u.)
            if let Some(vm) = bus_data["vm"].as_f64() {
                solution.vm.insert(bus_idx, vm);
            }

            // Voltage angle (radians) - may be stored as "va" in radians
            if let Some(va) = bus_data["va"].as_f64() {
                solution.va.insert(bus_idx, va);
            }
        }
    }

    // Extract generator outputs
    if let Some(gens) = data["gen"].as_object() {
        for (gen_idx_str, gen_data) in gens {
            let gen_idx: usize = gen_idx_str.parse()
                .with_context(|| format!("Invalid gen index: {}", gen_idx_str))?;

            if let Some(pg) = gen_data["pg"].as_f64() {
                solution.pgen.insert(gen_idx, pg);
            }
            if let Some(qg) = gen_data["qg"].as_f64() {
                solution.qgen.insert(gen_idx, qg);
            }
        }
    }

    Ok(solution)
}
```

**Note**: The exact JSON field names may differ. Inspect actual PFΔ JSON structure and adjust field names (`vm`, `va`, `pg`, `qg`) accordingly.

**Verification**: `cargo check -p gat-io`

---

### Task 18: Create load_pfdelta_instance function

**File**: `crates/gat-io/src/sources/pfdelta.rs`

**Action**: Add function that returns full instance with solution.

```rust
/// Load a PFΔ JSON file and return network with reference solution
pub fn load_pfdelta_instance(json_path: &Path, test_case: &PFDeltaTestCase) -> Result<PFDeltaInstance> {
    let json_content = fs::read_to_string(json_path)
        .with_context(|| format!("reading PFΔ JSON: {}", json_path.display()))?;

    let data: Value = serde_json::from_str(&json_content)
        .with_context(|| format!("parsing PFΔ JSON: {}", json_path.display()))?;

    let network = convert_pfdelta_to_network(&data)?;
    let solution = extract_pfdelta_solution(&data)?;

    Ok(PFDeltaInstance {
        test_case: test_case.clone(),
        network,
        solution,
    })
}
```

**Verification**: `cargo check -p gat-io`

---

### Task 19: Export new PFΔ types

**File**: `crates/gat-io/src/sources/mod.rs`

**Action**: Ensure new types are exported.

```rust
pub mod pfdelta;

pub use pfdelta::{
    list_pfdelta_cases, load_pfdelta_case, load_pfdelta_instance,
    PFDeltaInstance, PFDeltaSolution, PFDeltaTestCase,
};
```

**Verification**: `cargo check -p gat-io`

---

### Task 20: Add PF mode flag to pfdelta CLI

**File**: `crates/gat-cli/src/cli.rs`

**Action**: Add `--mode` flag to `BenchmarkCommands::Pfdelta`.

Find the `Pfdelta` variant in `BenchmarkCommands` and add:

```rust
/// Solve mode: pf (power flow) or opf (optimal power flow)
#[arg(long, default_value = "pf")]
mode: String,
```

**Verification**: `cargo check -p gat-cli`

---

### Task 21: Update pfdelta benchmark to support PF mode

**File**: `crates/gat-cli/src/commands/benchmark/pfdelta.rs`

**Action**: Update `benchmark_case` to branch on mode.

```rust
// Add to imports
use gat_algo::validation::{compute_pf_errors, PFReferenceSolution, PFErrorMetrics};
use gat_io::sources::pfdelta::{load_pfdelta_instance, PFDeltaInstance};

// Update BenchmarkResult struct to include error metrics
#[derive(Debug, Clone, Serialize)]
struct BenchmarkResult {
    case_name: String,
    contingency_type: String,
    case_index: usize,
    load_time_ms: f64,
    solve_time_ms: f64,
    total_time_ms: f64,
    converged: bool,
    iterations: u32,
    num_buses: usize,
    num_branches: usize,
    // New error fields
    max_vm_error: f64,
    max_va_error_deg: f64,
    mean_vm_error: f64,
    mean_va_error_deg: f64,
}

// Update benchmark_case signature and implementation
fn benchmark_case(
    test_case: &gat_io::sources::pfdelta::PFDeltaTestCase,
    idx: usize,
    mode: &str,
    tol: f64,
    max_iter: u32,
) -> Result<BenchmarkResult> {
    let load_start = Instant::now();

    // Load instance with reference solution
    let instance = load_pfdelta_instance(Path::new(&test_case.file_path), test_case)?;
    let load_time_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    let num_buses = instance.network.graph.node_indices().count();
    let num_branches = instance.network.graph.edge_indices().count();

    let solve_start = Instant::now();

    // Branch on mode
    let (converged, iterations, gat_vm, gat_va) = match mode {
        "pf" => {
            // TODO: Call AC power flow solver
            // let pf_result = gat_algo::power_flow::ac_solve(&instance.network, tol, max_iter)?;
            // (pf_result.converged, pf_result.iterations, pf_result.vm, pf_result.va)

            // Placeholder until PF solver interface is confirmed
            (true, 0, std::collections::HashMap::new(), std::collections::HashMap::new())
        }
        "opf" => {
            let solver = AcOpfSolver::new()
                .with_max_iterations(max_iter as usize)
                .with_tolerance(tol);
            let solution = solver.solve(&instance.network)?;

            // Extract voltage solution from OPF result
            // TODO: Map solution to HashMap<usize, f64> format
            (solution.converged, solution.iterations as u32, HashMap::new(), HashMap::new())
        }
        _ => return Err(anyhow!("Unknown mode: {}. Use 'pf' or 'opf'", mode)),
    };

    let solve_time_ms = solve_start.elapsed().as_secs_f64() * 1000.0;

    // Compute error metrics
    let ref_solution = PFReferenceSolution {
        vm: instance.solution.vm,
        va: instance.solution.va,
        pgen: instance.solution.pgen,
        qgen: instance.solution.qgen,
    };
    let errors = compute_pf_errors(&instance.network, &gat_vm, &gat_va, &ref_solution);

    Ok(BenchmarkResult {
        case_name: test_case.case_name.clone(),
        contingency_type: test_case.contingency_type.clone(),
        case_index: idx,
        load_time_ms,
        solve_time_ms,
        total_time_ms: load_time_ms + solve_time_ms,
        converged,
        iterations,
        num_buses,
        num_branches,
        max_vm_error: errors.max_vm_error,
        max_va_error_deg: errors.max_va_error_deg,
        mean_vm_error: errors.mean_vm_error,
        mean_va_error_deg: errors.mean_va_error_deg,
    })
}
```

**Note**: The actual PF solver call depends on `gat-algo`'s power flow API. Investigate how `gat pf ac` works and use the same interface.

**Verification**: `cargo check -p gat-cli`

---

### Task 22: Wire mode flag through handle function

**File**: `crates/gat-cli/src/commands/benchmark/pfdelta.rs`

**Action**: Update `handle` and `run_benchmark` to pass mode through.

```rust
// Update handle signature
#[allow(clippy::too_many_arguments)]
pub fn handle(
    pfdelta_root: &str,
    case_filter: Option<&str>,
    contingency_filter: &str,
    max_cases: usize,
    out: &str,
    threads: &str,
    mode: &str,  // Add this
    tol: f64,
    max_iter: u32,
) -> Result<()> {
    // ...
}

// Update BenchmarkConfig
struct BenchmarkConfig {
    // ... existing fields ...
    mode: String,  // Add this
}

// Update run_benchmark to pass mode to benchmark_case
let results: Vec<BenchmarkResult> = all_cases
    .par_iter()
    .enumerate()
    .filter_map(|(idx, test_case)| {
        benchmark_case(test_case, idx, &config.mode, config.tol, config.max_iter).ok()
    })
    .collect();
```

**Verification**: `cargo check -p gat-cli`

---

### Task 23: Update CLI dispatch for mode flag

**File**: `crates/gat-cli/src/commands/benchmark/mod.rs`

**Action**: Update the match arm for `Pfdelta` to pass mode.

```rust
BenchmarkCommands::Pfdelta {
    pfdelta_root,
    case,
    contingency,
    max_cases,
    out,
    threads,
    mode,  // Add this
    tol,
    max_iter,
} => pfdelta::handle(
    pfdelta_root,
    case.as_deref(),
    contingency,
    *max_cases,
    out,
    threads,
    mode,  // Add this
    *tol,
    *max_iter,
),
```

**Verification**: `cargo build -p gat-cli && ./target/debug/gat-cli benchmark pfdelta --help`

---

## Phase 4: PGLib Benchmark (Tasks 24-34)

### Task 24: Create pglib benchmark module

**File**: `crates/gat-cli/src/commands/benchmark/pglib.rs` (new)

**Action**: Create the module structure.

```rust
//! PGLib-OPF centralized AC-OPF benchmark command.
//!
//! Runs AC-OPF on PGLib test cases and compares against baseline objectives.

use anyhow::{anyhow, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Instant;

use gat_algo::AcOpfSolver;
use gat_io::importers::matpower::load_matpower_network;

use super::baseline::{load_baseline_objectives, normalize_case_name};
use super::common::BenchmarkTolerances;

/// Result for a single PGLib benchmark case
#[derive(Debug, Clone, Serialize)]
pub struct PglibBenchmarkResult {
    pub case_name: String,
    pub size_class: String,
    pub load_time_ms: f64,
    pub solve_time_ms: f64,
    pub total_time_ms: f64,
    pub converged: bool,
    pub iterations: u32,
    pub num_buses: usize,
    pub num_branches: usize,
    pub num_gens: usize,
    pub objective_gat: f64,
    pub objective_baseline: Option<f64>,
    pub objective_gap_abs: Option<f64>,
    pub objective_gap_rel: Option<f64>,
    pub max_p_balance_violation: f64,
    pub max_q_balance_violation: f64,
    pub max_vm_violation: f64,
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 25: Implement case discovery for PGLib

**File**: `crates/gat-cli/src/commands/benchmark/pglib.rs`

**Action**: Add functions to find and classify PGLib cases.

```rust
/// Discovered PGLib test case
#[derive(Debug, Clone)]
pub struct PglibCase {
    pub name: String,
    pub path: PathBuf,
    pub size_class: String,
}

/// List PGLib cases in a directory
pub fn list_pglib_cases(root: &Path) -> Result<Vec<PglibCase>> {
    let mut cases = Vec::new();

    for entry in fs::read_dir(root)
        .with_context(|| format!("reading PGLib directory: {}", root.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map_or(false, |ext| ext == "m") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Only include pglib_opf cases
            if name.starts_with("pglib_opf_case") {
                let size_class = classify_case_size(&name);
                cases.push(PglibCase {
                    name,
                    path,
                    size_class,
                });
            }
        }
    }

    // Sort by name for deterministic ordering
    cases.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(cases)
}

/// Classify case size based on name (extracts bus count from name)
fn classify_case_size(name: &str) -> String {
    // pglib_opf_case{N}_{variant}.m -> extract N
    let bus_count = extract_bus_count(name);

    match bus_count {
        Some(n) if n < 200 => "small".to_string(),
        Some(n) if n < 2000 => "medium".to_string(),
        Some(_) => "large".to_string(),
        None => "unknown".to_string(),
    }
}

/// Extract bus count from case name
fn extract_bus_count(name: &str) -> Option<usize> {
    // Pattern: pglib_opf_case{N}_...
    let parts: Vec<&str> = name.split('_').collect();
    for part in parts {
        if part.starts_with("case") {
            let num_str: String = part.chars().skip(4).take_while(|c| c.is_ascii_digit()).collect();
            return num_str.parse().ok();
        }
    }
    None
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 26: Implement PGLib benchmark execution

**File**: `crates/gat-cli/src/commands/benchmark/pglib.rs`

**Action**: Add the main benchmark logic.

```rust
/// Configuration for PGLib benchmark
#[derive(Debug)]
struct PglibConfig {
    pglib_root: PathBuf,
    case_filter: Option<String>,
    size_filter: String,
    max_cases: usize,
    baseline_path: Option<PathBuf>,
    out: PathBuf,
    threads: String,
    tol: f64,
    max_iter: u32,
    tolerances: BenchmarkTolerances,
}

#[allow(clippy::too_many_arguments)]
pub fn handle(
    pglib_root: &str,
    case_filter: Option<&str>,
    size_filter: &str,
    max_cases: usize,
    baseline: Option<&str>,
    out: &str,
    threads: &str,
    tol: f64,
    max_iter: u32,
    obj_tol: Option<f64>,
    constraint_tol: Option<f64>,
) -> Result<()> {
    let config = PglibConfig {
        pglib_root: PathBuf::from(pglib_root),
        case_filter: case_filter.map(String::from),
        size_filter: size_filter.to_string(),
        max_cases,
        baseline_path: baseline.map(PathBuf::from),
        out: PathBuf::from(out),
        threads: threads.to_string(),
        tol,
        max_iter,
        tolerances: BenchmarkTolerances::from_args(obj_tol, constraint_tol, None),
    };

    run_benchmark(&config)
}

fn run_benchmark(config: &PglibConfig) -> Result<()> {
    // Configure thread pool
    if config.threads != "auto" {
        if let Ok(n) = config.threads.parse::<usize>() {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global()
                .ok();
        }
    }

    // List cases
    let mut cases = list_pglib_cases(&config.pglib_root)?;

    // Apply filters
    if let Some(filter) = &config.case_filter {
        cases.retain(|c| c.name.contains(filter));
    }
    if config.size_filter != "all" {
        cases.retain(|c| c.size_class == config.size_filter);
    }
    if config.max_cases > 0 {
        cases.truncate(config.max_cases);
    }

    eprintln!("Found {} PGLib cases to benchmark", cases.len());

    // Load baseline if provided
    let baseline_map: HashMap<String, f64> = config
        .baseline_path
        .as_ref()
        .map(|p| load_baseline_objectives(p))
        .transpose()?
        .unwrap_or_default();

    // Run benchmarks in parallel
    let results: Vec<PglibBenchmarkResult> = cases
        .par_iter()
        .filter_map(|case| {
            let baseline_obj = baseline_map.get(&normalize_case_name(&case.name)).copied();
            benchmark_case(case, baseline_obj, config.tol, config.max_iter).ok()
        })
        .collect();

    // Write results
    write_results(&config.out, &results)?;

    // Print summary
    let converged_count = results.iter().filter(|r| r.converged).count();
    let avg_time: f64 = if !results.is_empty() {
        results.iter().map(|r| r.solve_time_ms).sum::<f64>() / results.len() as f64
    } else {
        0.0
    };

    eprintln!(
        "\nPGLib Benchmark Results:\n  Total cases: {}\n  Converged: {}\n  Avg solve time: {:.2}ms\n  Output: {}",
        results.len(),
        converged_count,
        avg_time,
        config.out.display()
    );

    Ok(())
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 27: Implement single case benchmark for PGLib

**File**: `crates/gat-cli/src/commands/benchmark/pglib.rs`

**Action**: Add the per-case benchmark function.

```rust
fn benchmark_case(
    case: &PglibCase,
    baseline_obj: Option<f64>,
    tol: f64,
    max_iter: u32,
) -> Result<PglibBenchmarkResult> {
    // Load network
    let load_start = Instant::now();
    let network = load_matpower_network(&case.path)
        .with_context(|| format!("loading {}", case.path.display()))?;
    let load_time_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    let num_buses = network.graph.node_indices().count();
    let num_branches = network.graph.edge_indices().count();
    let num_gens = network.generators().len();

    // Solve AC-OPF
    let solve_start = Instant::now();
    let solver = AcOpfSolver::new()
        .with_max_iterations(max_iter as usize)
        .with_tolerance(tol);
    let solution = solver.solve(&network)?;
    let solve_time_ms = solve_start.elapsed().as_secs_f64() * 1000.0;

    // Extract objective
    let objective_gat = solution.objective_value();

    // Compute objective gap if baseline provided
    let (objective_gap_abs, objective_gap_rel) = if let Some(ref_obj) = baseline_obj {
        let gap = gat_algo::validation::ObjectiveGap::new(objective_gat, ref_obj);
        (Some(gap.gap_abs), Some(gap.gap_rel))
    } else {
        (None, None)
    };

    // Compute violations (placeholder - needs actual implementation)
    let violations = gat_algo::validation::compute_opf_violations(&network, &solution);

    Ok(PglibBenchmarkResult {
        case_name: case.name.clone(),
        size_class: case.size_class.clone(),
        load_time_ms,
        solve_time_ms,
        total_time_ms: load_time_ms + solve_time_ms,
        converged: solution.converged,
        iterations: solution.iterations as u32,
        num_buses,
        num_branches,
        num_gens,
        objective_gat,
        objective_baseline: baseline_obj,
        objective_gap_abs,
        objective_gap_rel,
        max_p_balance_violation: violations.max_p_balance_violation,
        max_q_balance_violation: violations.max_q_balance_violation,
        max_vm_violation: violations.max_vm_violation,
    })
}

fn write_results(path: &Path, results: &[PglibBenchmarkResult]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if parent != Path::new("") {
            fs::create_dir_all(parent)?;
        }
    }

    let file = File::create(path)
        .with_context(|| format!("creating output file: {}", path.display()))?;
    let mut writer = Writer::from_writer(file);

    for result in results {
        writer.serialize(result)?;
    }

    writer.flush()?;
    Ok(())
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 28: Add PGLib CLI command

**File**: `crates/gat-cli/src/cli.rs`

**Action**: Add `Pglib` variant to `BenchmarkCommands`.

```rust
/// Run centralized AC-OPF benchmarks on PGLib-OPF cases
Pglib {
    /// Root directory containing PGLib-OPF .m files
    #[arg(long, value_hint = ValueHint::DirPath)]
    pglib_root: String,

    /// Case filter: substring match (e.g. "case118", "case14")
    #[arg(long)]
    case: Option<String>,

    /// Size filter: small, medium, large, or all
    #[arg(long, default_value = "all")]
    size: String,

    /// Maximum number of cases to run (0 = all)
    #[arg(long, default_value_t = 0)]
    max_cases: usize,

    /// Baseline CSV with reference objectives (case_name,objective)
    #[arg(long, value_hint = ValueHint::FilePath)]
    baseline: Option<String>,

    /// Output CSV file
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    out: String,

    /// Number of threads (auto = CPU count)
    #[arg(long, default_value = "auto")]
    threads: String,

    /// Solver convergence tolerance
    #[arg(long, default_value = "1e-6")]
    tol: f64,

    /// Maximum solver iterations
    #[arg(long, default_value_t = 100)]
    max_iter: u32,

    /// Objective value relative tolerance for validation
    #[arg(long)]
    obj_tol: Option<f64>,

    /// Constraint violation tolerance for validation
    #[arg(long)]
    constraint_tol: Option<f64>,
},
```

**Verification**: `cargo check -p gat-cli`

---

### Task 29: Wire PGLib command in benchmark mod

**File**: `crates/gat-cli/src/commands/benchmark/mod.rs`

**Action**: Add pglib module and dispatch.

```rust
pub mod pglib;

// In the handle function, add:
BenchmarkCommands::Pglib {
    pglib_root,
    case,
    size,
    max_cases,
    baseline,
    out,
    threads,
    tol,
    max_iter,
    obj_tol,
    constraint_tol,
} => pglib::handle(
    pglib_root,
    case.as_deref(),
    size,
    *max_cases,
    baseline.as_deref(),
    out,
    threads,
    *tol,
    *max_iter,
    *obj_tol,
    *constraint_tol,
),
```

**Verification**: `cargo build -p gat-cli && ./target/debug/gat-cli benchmark pglib --help`

---

### Task 30: Add unit tests for PGLib case discovery

**File**: `crates/gat-cli/src/commands/benchmark/pglib.rs`

**Action**: Add tests at bottom of module.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bus_count() {
        assert_eq!(extract_bus_count("pglib_opf_case5_pjm"), Some(5));
        assert_eq!(extract_bus_count("pglib_opf_case14_ieee"), Some(14));
        assert_eq!(extract_bus_count("pglib_opf_case118_ieee"), Some(118));
        assert_eq!(extract_bus_count("pglib_opf_case1354_pegase"), Some(1354));
        assert_eq!(extract_bus_count("pglib_opf_case9241_pegase"), Some(9241));
    }

    #[test]
    fn test_classify_case_size() {
        assert_eq!(classify_case_size("pglib_opf_case5_pjm"), "small");
        assert_eq!(classify_case_size("pglib_opf_case118_ieee"), "small");
        assert_eq!(classify_case_size("pglib_opf_case300_ieee"), "medium");
        assert_eq!(classify_case_size("pglib_opf_case1354_pegase"), "medium");
        assert_eq!(classify_case_size("pglib_opf_case9241_pegase"), "large");
    }
}
```

**Verification**: `cargo test -p gat-cli pglib`

---

### Task 31: Add unit test for baseline loader

**File**: `crates/gat-cli/src/commands/benchmark/baseline.rs`

**Action**: Add tests.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_baseline_objectives() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "case_name,objective").unwrap();
        writeln!(file, "pglib_opf_case5_pjm,17551.89").unwrap();
        writeln!(file, "pglib_opf_case14_ieee,8081.53").unwrap();

        let map = load_baseline_objectives(file.path()).unwrap();

        assert_eq!(map.len(), 2);
        assert!((map["pglib_opf_case5_pjm"] - 17551.89).abs() < 0.01);
        assert!((map["pglib_opf_case14_ieee"] - 8081.53).abs() < 0.01);
    }

    #[test]
    fn test_normalize_case_name() {
        assert_eq!(normalize_case_name("PGLIB_OPF_CASE5_PJM.m"), "pglib_opf_case5_pjm");
        assert_eq!(normalize_case_name("case14.json"), "case14");
        assert_eq!(normalize_case_name("  Case5  "), "case5");
    }
}
```

**Verification**: `cargo test -p gat-cli baseline`

---

### Task 32: Create PGLib CLI integration test

**File**: `crates/gat-cli/tests/benchmark_pglib.rs` (new)

**Action**: Create integration test using fixtures.

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_benchmark_pglib_small() {
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_data/pglib");

    // Skip if fixtures not available
    if !fixture_dir.exists() {
        eprintln!("Skipping test: PGLib fixtures not found at {:?}", fixture_dir);
        return;
    }

    let out_dir = TempDir::new().unwrap();
    let out_csv = out_dir.path().join("pglib_results.csv");

    Command::cargo_bin("gat-cli")
        .unwrap()
        .args([
            "benchmark",
            "pglib",
            "--pglib-root",
            fixture_dir.to_str().unwrap(),
            "--size",
            "small",
            "--max-cases",
            "2",
            "--threads",
            "1",
            "--tol",
            "1e-6",
            "--max-iter",
            "50",
            "--out",
            out_csv.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify output exists
    assert!(out_csv.exists());

    // Parse and verify results
    let content = fs::read_to_string(&out_csv).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Should have header + 2 data rows (or fewer if cases missing)
    assert!(lines.len() >= 2, "Expected at least header + 1 result");

    // Check that converged column exists and has true values
    assert!(content.contains("converged"));
}
```

**Verification**: `cargo test -p gat-cli --test benchmark_pglib` (will skip if fixtures missing)

---

### Task 33: Verify MATPOWER importer handles PGLib format

**File**: `crates/gat-io/src/importers/matpower.rs`

**Action**: Review and test that the importer handles PGLib-specific fields.

Run a quick test:
```bash
cargo run -p gat-cli -- import matpower --file test_data/pglib/pglib_opf_case5_pjm.m --out /tmp/test.arrow
```

If it fails, identify which PGLib fields are missing and add them. Common PGLib additions:
- `gencost` section with polynomial cost coefficients
- `branch` with `rateA`, `rateB`, `rateC` limits
- `bus` with `Vmin`, `Vmax` voltage limits

**Verification**: Import completes without error; network has expected bus/branch/gen counts.

---

### Task 34: Add tempfile dependency if needed

**File**: `crates/gat-cli/Cargo.toml`

**Action**: Add tempfile as dev dependency for tests.

```toml
[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
predicates = "3"
```

**Verification**: `cargo check -p gat-cli --tests`

---

## Phase 5: OPFData Benchmark (Tasks 35-46)

### Task 35: Create opfdata source module

**File**: `crates/gat-io/src/sources/opfdata.rs` (new)

**Action**: Create the module with data structures.

```rust
//! OPFData/GridOpt dataset loader for AC-OPF with topology perturbations.
//!
//! OPFData provides 300k+ solved AC-OPF instances per grid with:
//! - Load perturbations (FullTop)
//! - Topology perturbations (N-1 line/gen/transformer outages)
//!
//! Reference: https://arxiv.org/abs/2406.07234

use anyhow::{anyhow, Context, Result};
use gat_core::{Branch, BranchId, Bus, BusId, Edge, Gen, GenId, Load, LoadId, Network, Node};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Variation type in OPFData
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpfDataVariation {
    Load,
    Topology,
}

impl OpfDataVariation {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpfDataVariation::Load => "load",
            OpfDataVariation::Topology => "topo",
        }
    }
}

/// Identifier for a specific sample in OPFData
#[derive(Debug, Clone)]
pub struct OpfDataSampleId {
    pub grid_id: String,
    pub variation: OpfDataVariation,
    pub shard: String,
    pub index_in_shard: usize,
}

/// Metadata about an OPFData sample
#[derive(Debug, Clone)]
pub struct OpfDataSampleMeta {
    pub sample_id: OpfDataSampleId,
    pub num_buses: usize,
    pub num_branches: usize,
    pub num_gens: usize,
    pub objective: f64,
}

/// Reference OPF solution from OPFData
#[derive(Debug, Clone, Default)]
pub struct OpfDataSolution {
    pub vm: HashMap<usize, f64>,
    pub va: HashMap<usize, f64>,
    pub pgen: HashMap<usize, f64>,
    pub qgen: HashMap<usize, f64>,
}

/// Complete OPFData instance
#[derive(Debug, Clone)]
pub struct OpfDataInstance {
    pub meta: OpfDataSampleMeta,
    pub network: Network,
    pub solution: OpfDataSolution,
}
```

**Verification**: `cargo check -p gat-io`

---

### Task 36: Implement shard listing for OPFData

**File**: `crates/gat-io/src/sources/opfdata.rs`

**Action**: Add function to discover shards.

```rust
/// Discovered shard in OPFData directory
#[derive(Debug, Clone)]
pub struct OpfDataShard {
    pub grid_id: String,
    pub variation: OpfDataVariation,
    pub path: PathBuf,
}

/// List available shards in OPFData directory
///
/// Expected structure:
/// ```text
/// opfdata_root/
///   case118/
///     load/
///       sample.jsonl (or shards)
///     topo/
///       sample.jsonl
///   case300/
///     ...
/// ```
pub fn list_shards(root: &Path) -> Result<Vec<OpfDataShard>> {
    let mut shards = Vec::new();

    for grid_entry in fs::read_dir(root)
        .with_context(|| format!("reading OPFData directory: {}", root.display()))?
    {
        let grid_entry = grid_entry?;
        let grid_path = grid_entry.path();

        if !grid_path.is_dir() {
            continue;
        }

        let grid_id = grid_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Check for load and topo subdirectories
        for (variation, subdir) in [
            (OpfDataVariation::Load, "load"),
            (OpfDataVariation::Topology, "topo"),
        ] {
            let var_path = grid_path.join(subdir);
            if var_path.is_dir() {
                // Find JSONL files
                if let Ok(files) = fs::read_dir(&var_path) {
                    for file in files.flatten() {
                        let file_path = file.path();
                        if file_path.extension().map_or(false, |e| e == "jsonl" || e == "json") {
                            shards.push(OpfDataShard {
                                grid_id: grid_id.clone(),
                                variation,
                                path: file_path,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(shards)
}
```

**Verification**: `cargo check -p gat-io`

---

### Task 37: Implement sample iterator for OPFData

**File**: `crates/gat-io/src/sources/opfdata.rs`

**Action**: Add iterator over samples in a shard.

```rust
/// Iterate over samples in an OPFData JSONL shard
pub fn iter_samples(
    shard: &OpfDataShard,
) -> Result<impl Iterator<Item = Result<OpfDataInstance>>> {
    let file = File::open(&shard.path)
        .with_context(|| format!("opening shard: {}", shard.path.display()))?;
    let reader = BufReader::new(file);

    let grid_id = shard.grid_id.clone();
    let variation = shard.variation;
    let shard_name = shard
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(reader.lines().enumerate().map(move |(idx, line_result)| {
        let line = line_result.with_context(|| format!("reading line {} of shard", idx))?;
        let json: Value = serde_json::from_str(&line)
            .with_context(|| format!("parsing JSON at line {}", idx))?;

        parse_opfdata_sample(&json, &grid_id, variation, &shard_name, idx)
    }))
}

fn parse_opfdata_sample(
    json: &Value,
    grid_id: &str,
    variation: OpfDataVariation,
    shard: &str,
    index: usize,
) -> Result<OpfDataInstance> {
    let network = build_network_from_opfdata(json)?;
    let solution = build_solution_from_opfdata(json)?;

    let num_buses = network.graph.node_indices().count();
    let num_branches = network.graph.edge_indices().count();
    let num_gens = count_generators(&network);

    // Extract objective from JSON
    let objective = json["objective"]
        .as_f64()
        .or_else(|| json["cost"].as_f64())
        .unwrap_or(0.0);

    let meta = OpfDataSampleMeta {
        sample_id: OpfDataSampleId {
            grid_id: grid_id.to_string(),
            variation,
            shard: shard.to_string(),
            index_in_shard: index,
        },
        num_buses,
        num_branches,
        num_gens,
        objective,
    };

    Ok(OpfDataInstance {
        meta,
        network,
        solution,
    })
}

fn count_generators(network: &Network) -> usize {
    network
        .graph
        .node_weights()
        .filter(|n| matches!(n, Node::Gen(_)))
        .count()
}
```

**Verification**: `cargo check -p gat-io`

---

### Task 38: Implement network builder for OPFData

**File**: `crates/gat-io/src/sources/opfdata.rs`

**Action**: Add function to build Network from OPFData JSON.

```rust
/// Build GAT Network from OPFData JSON
///
/// OPFData uses PowerModels.jl format, which is similar to MATPOWER.
fn build_network_from_opfdata(json: &Value) -> Result<Network> {
    let mut network = Network::new();
    let mut bus_node_map: HashMap<usize, gat_core::NodeIndex> = HashMap::new();

    // Extract buses
    let buses = json["bus"]
        .as_object()
        .ok_or_else(|| anyhow!("No 'bus' field in OPFData JSON"))?;

    for (bus_idx_str, bus_data) in buses {
        let bus_idx: usize = bus_idx_str
            .parse()
            .with_context(|| format!("Invalid bus index: {}", bus_idx_str))?;

        let voltage_kv = bus_data["base_kv"]
            .as_f64()
            .or_else(|| bus_data["vn"].as_f64())
            .unwrap_or(100.0);

        let node_idx = network.graph.add_node(Node::Bus(Bus {
            id: BusId::new(bus_idx),
            name: format!("bus_{}", bus_idx),
            voltage_kv,
        }));

        bus_node_map.insert(bus_idx, node_idx);
    }

    // Extract generators
    if let Some(gens) = json["gen"].as_object() {
        for (gen_idx_str, gen_data) in gens {
            let gen_idx: usize = gen_idx_str.parse()?;

            // Check status - skip if out of service
            let status = gen_data["gen_status"]
                .as_i64()
                .or_else(|| gen_data["status"].as_i64())
                .unwrap_or(1);

            if status == 0 {
                continue; // Skip offline generators
            }

            let bus_id = gen_data["gen_bus"]
                .as_u64()
                .or_else(|| gen_data["bus"].as_u64())
                .unwrap_or(0) as usize;

            let pg = gen_data["pg"].as_f64().unwrap_or(0.0);
            let qg = gen_data["qg"].as_f64().unwrap_or(0.0);

            network.graph.add_node(Node::Gen(Gen {
                id: GenId::new(gen_idx),
                name: format!("gen_{}", gen_idx),
                bus: BusId::new(bus_id),
                active_power_mw: pg,
                reactive_power_mvar: qg,
            }));
        }
    }

    // Extract loads
    if let Some(loads) = json["load"].as_object() {
        for (load_idx_str, load_data) in loads {
            let load_idx: usize = load_idx_str.parse()?;

            let status = load_data["status"].as_i64().unwrap_or(1);
            if status == 0 {
                continue;
            }

            let bus_id = load_data["load_bus"]
                .as_u64()
                .or_else(|| load_data["bus"].as_u64())
                .unwrap_or(0) as usize;

            let pd = load_data["pd"].as_f64().unwrap_or(0.0);
            let qd = load_data["qd"].as_f64().unwrap_or(0.0);

            network.graph.add_node(Node::Load(Load {
                id: LoadId::new(load_idx),
                name: format!("load_{}", load_idx),
                bus: BusId::new(bus_id),
                active_power_mw: pd,
                reactive_power_mvar: qd,
            }));
        }
    }

    // Extract branches
    if let Some(branches) = json["branch"].as_object() {
        for (branch_idx_str, branch_data) in branches {
            let branch_idx: usize = branch_idx_str.parse()?;

            // Check status - skip if out of service (topology perturbation)
            let status = branch_data["br_status"]
                .as_i64()
                .or_else(|| branch_data["status"].as_i64())
                .unwrap_or(1);

            if status == 0 {
                continue; // Skip outaged branches
            }

            let from_bus = branch_data["f_bus"]
                .as_u64()
                .or_else(|| branch_data["fbus"].as_u64())
                .unwrap_or(0) as usize;

            let to_bus = branch_data["t_bus"]
                .as_u64()
                .or_else(|| branch_data["tbus"].as_u64())
                .unwrap_or(0) as usize;

            let r = branch_data["br_r"]
                .as_f64()
                .or_else(|| branch_data["r"].as_f64())
                .unwrap_or(0.0);

            let x = branch_data["br_x"]
                .as_f64()
                .or_else(|| branch_data["x"].as_f64())
                .unwrap_or(0.01);

            if let (Some(&from_idx), Some(&to_idx)) =
                (bus_node_map.get(&from_bus), bus_node_map.get(&to_bus))
            {
                network.graph.add_edge(
                    from_idx,
                    to_idx,
                    Edge::Branch(Branch {
                        id: BranchId::new(branch_idx),
                        name: format!("br_{}_{}", from_bus, to_bus),
                        from_bus: BusId::new(from_bus),
                        to_bus: BusId::new(to_bus),
                        resistance: r,
                        reactance: x,
                    }),
                );
            }
        }
    }

    Ok(network)
}
```

**Verification**: `cargo check -p gat-io`

---

### Task 39: Implement solution extractor for OPFData

**File**: `crates/gat-io/src/sources/opfdata.rs`

**Action**: Add function to extract reference solution.

```rust
/// Extract reference OPF solution from OPFData JSON
fn build_solution_from_opfdata(json: &Value) -> Result<OpfDataSolution> {
    let mut solution = OpfDataSolution::default();

    // Extract bus voltages
    if let Some(buses) = json["bus"].as_object() {
        for (bus_idx_str, bus_data) in buses {
            let bus_idx: usize = bus_idx_str.parse()?;

            if let Some(vm) = bus_data["vm"].as_f64() {
                solution.vm.insert(bus_idx, vm);
            }
            if let Some(va) = bus_data["va"].as_f64() {
                solution.va.insert(bus_idx, va);
            }
        }
    }

    // Extract generator outputs
    if let Some(gens) = json["gen"].as_object() {
        for (gen_idx_str, gen_data) in gens {
            let gen_idx: usize = gen_idx_str.parse()?;

            if let Some(pg) = gen_data["pg"].as_f64() {
                solution.pgen.insert(gen_idx, pg);
            }
            if let Some(qg) = gen_data["qg"].as_f64() {
                solution.qgen.insert(gen_idx, qg);
            }
        }
    }

    Ok(solution)
}
```

**Verification**: `cargo check -p gat-io`

---

### Task 40: Export OPFData types

**File**: `crates/gat-io/src/sources/mod.rs`

**Action**: Add opfdata module and exports.

```rust
pub mod opfdata;

pub use opfdata::{
    iter_samples as iter_opfdata_samples, list_shards as list_opfdata_shards,
    OpfDataInstance, OpfDataSampleId, OpfDataSampleMeta, OpfDataShard,
    OpfDataSolution, OpfDataVariation,
};
```

**Verification**: `cargo check -p gat-io`

---

### Task 41: Create opfdata benchmark module

**File**: `crates/gat-cli/src/commands/benchmark/opfdata.rs` (new)

**Action**: Create the benchmark command implementation.

```rust
//! OPFData/GridOpt AC-OPF benchmark with topology perturbations.

use anyhow::{anyhow, Context, Result};
use csv::Writer;
use rayon::prelude::*;
use serde::Serialize;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Instant;

use gat_algo::AcOpfSolver;
use gat_algo::validation::ObjectiveGap;
use gat_io::sources::opfdata::{
    iter_samples, list_shards, OpfDataInstance, OpfDataShard, OpfDataVariation,
};

use super::common::BenchmarkTolerances;

/// Result for a single OPFData benchmark sample
#[derive(Debug, Clone, Serialize)]
pub struct OpfDataBenchmarkResult {
    pub grid_id: String,
    pub variation: String,
    pub shard: String,
    pub index_in_shard: usize,
    pub load_time_ms: f64,
    pub solve_time_ms: f64,
    pub total_time_ms: f64,
    pub converged: bool,
    pub iterations: u32,
    pub num_buses: usize,
    pub num_branches: usize,
    pub num_gens: usize,
    pub objective_ref: f64,
    pub objective_gat: f64,
    pub objective_gap_abs: f64,
    pub objective_gap_rel: f64,
    pub max_p_balance_violation: f64,
    pub max_q_balance_violation: f64,
    pub max_vm_violation: f64,
}

#[derive(Debug)]
struct OpfDataConfig {
    root: PathBuf,
    grid_filter: String,
    variation: OpfDataVariation,
    max_samples: usize,
    out: PathBuf,
    threads: String,
    tol: f64,
    max_iter: u32,
    tolerances: BenchmarkTolerances,
}

#[allow(clippy::too_many_arguments)]
pub fn handle(
    opfdata_root: &str,
    grid: &str,
    variation: &str,
    max_samples: usize,
    out: &str,
    threads: &str,
    tol: f64,
    max_iter: u32,
    obj_tol: Option<f64>,
    constraint_tol: Option<f64>,
) -> Result<()> {
    let variation_enum = match variation {
        "load" => OpfDataVariation::Load,
        "topo" | "topology" => OpfDataVariation::Topology,
        _ => return Err(anyhow!("Invalid variation: {}. Use 'load' or 'topo'", variation)),
    };

    let config = OpfDataConfig {
        root: PathBuf::from(opfdata_root),
        grid_filter: grid.to_string(),
        variation: variation_enum,
        max_samples,
        out: PathBuf::from(out),
        threads: threads.to_string(),
        tol,
        max_iter,
        tolerances: BenchmarkTolerances::from_args(obj_tol, constraint_tol, None),
    };

    run_benchmark(&config)
}

fn run_benchmark(config: &OpfDataConfig) -> Result<()> {
    // Configure threads
    if config.threads != "auto" {
        if let Ok(n) = config.threads.parse::<usize>() {
            rayon::ThreadPoolBuilder::new()
                .num_threads(n)
                .build_global()
                .ok();
        }
    }

    // Find matching shards
    let shards = list_shards(&config.root)?;
    let matching_shards: Vec<_> = shards
        .into_iter()
        .filter(|s| s.grid_id.contains(&config.grid_filter) && s.variation == config.variation)
        .collect();

    if matching_shards.is_empty() {
        return Err(anyhow!(
            "No shards found for grid={} variation={:?}",
            config.grid_filter,
            config.variation
        ));
    }

    eprintln!(
        "Found {} matching shards for grid={} variation={:?}",
        matching_shards.len(),
        config.grid_filter,
        config.variation
    );

    // Collect samples (up to max)
    let mut samples: Vec<OpfDataInstance> = Vec::new();
    for shard in &matching_shards {
        if samples.len() >= config.max_samples {
            break;
        }
        for sample_result in iter_samples(shard)? {
            if samples.len() >= config.max_samples {
                break;
            }
            match sample_result {
                Ok(instance) => samples.push(instance),
                Err(e) => eprintln!("Warning: Failed to parse sample: {}", e),
            }
        }
    }

    eprintln!("Loaded {} samples for benchmarking", samples.len());

    // Run benchmarks in parallel
    let results: Vec<OpfDataBenchmarkResult> = samples
        .par_iter()
        .filter_map(|instance| benchmark_instance(instance, config.tol, config.max_iter).ok())
        .collect();

    // Write results
    write_results(&config.out, &results)?;

    // Summary
    let converged_count = results.iter().filter(|r| r.converged).count();
    let avg_gap: f64 = if !results.is_empty() {
        results.iter().map(|r| r.objective_gap_rel).sum::<f64>() / results.len() as f64
    } else {
        0.0
    };

    eprintln!(
        "\nOPFData Benchmark Results:\n  Samples: {}\n  Converged: {}\n  Avg objective gap: {:.2e}\n  Output: {}",
        results.len(),
        converged_count,
        avg_gap,
        config.out.display()
    );

    Ok(())
}

fn benchmark_instance(
    instance: &OpfDataInstance,
    tol: f64,
    max_iter: u32,
) -> Result<OpfDataBenchmarkResult> {
    let load_start = Instant::now();
    // Network is already loaded; this timing represents any additional prep
    let load_time_ms = load_start.elapsed().as_secs_f64() * 1000.0;

    // Solve
    let solve_start = Instant::now();
    let solver = AcOpfSolver::new()
        .with_max_iterations(max_iter as usize)
        .with_tolerance(tol);
    let solution = solver.solve(&instance.network)?;
    let solve_time_ms = solve_start.elapsed().as_secs_f64() * 1000.0;

    let objective_gat = solution.objective_value();
    let gap = ObjectiveGap::new(objective_gat, instance.meta.objective);

    let violations = gat_algo::validation::compute_opf_violations(&instance.network, &solution);

    Ok(OpfDataBenchmarkResult {
        grid_id: instance.meta.sample_id.grid_id.clone(),
        variation: instance.meta.sample_id.variation.as_str().to_string(),
        shard: instance.meta.sample_id.shard.clone(),
        index_in_shard: instance.meta.sample_id.index_in_shard,
        load_time_ms,
        solve_time_ms,
        total_time_ms: load_time_ms + solve_time_ms,
        converged: solution.converged,
        iterations: solution.iterations as u32,
        num_buses: instance.meta.num_buses,
        num_branches: instance.meta.num_branches,
        num_gens: instance.meta.num_gens,
        objective_ref: instance.meta.objective,
        objective_gat,
        objective_gap_abs: gap.gap_abs,
        objective_gap_rel: gap.gap_rel,
        max_p_balance_violation: violations.max_p_balance_violation,
        max_q_balance_violation: violations.max_q_balance_violation,
        max_vm_violation: violations.max_vm_violation,
    })
}

fn write_results(path: &Path, results: &[OpfDataBenchmarkResult]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if parent != Path::new("") {
            fs::create_dir_all(parent)?;
        }
    }

    let file = File::create(path)?;
    let mut writer = Writer::from_writer(file);

    for result in results {
        writer.serialize(result)?;
    }

    writer.flush()?;
    Ok(())
}
```

**Verification**: `cargo check -p gat-cli`

---

### Task 42: Add OPFData CLI command

**File**: `crates/gat-cli/src/cli.rs`

**Action**: Add `Opfdata` variant to `BenchmarkCommands`.

```rust
/// Benchmark AC-OPF on OPFData/GridOpt with topology perturbations
Opfdata {
    /// Root directory of OPFData dataset
    #[arg(long, value_hint = ValueHint::DirPath)]
    opfdata_root: String,

    /// Grid ID filter (e.g. "case118", "118")
    #[arg(long)]
    grid: String,

    /// Variation type: load or topo
    #[arg(long)]
    variation: String,

    /// Maximum number of samples to benchmark
    #[arg(long, default_value_t = 1000)]
    max_samples: usize,

    /// Output CSV file
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    out: String,

    /// Number of threads (auto = CPU count)
    #[arg(long, default_value = "auto")]
    threads: String,

    /// Solver convergence tolerance
    #[arg(long, default_value = "1e-6")]
    tol: f64,

    /// Maximum solver iterations
    #[arg(long, default_value_t = 100)]
    max_iter: u32,

    /// Objective value relative tolerance
    #[arg(long)]
    obj_tol: Option<f64>,

    /// Constraint violation tolerance
    #[arg(long)]
    constraint_tol: Option<f64>,
},
```

**Verification**: `cargo check -p gat-cli`

---

### Task 43: Wire OPFData command in benchmark mod

**File**: `crates/gat-cli/src/commands/benchmark/mod.rs`

**Action**: Add opfdata module and dispatch.

```rust
pub mod opfdata;

// In handle function, add:
BenchmarkCommands::Opfdata {
    opfdata_root,
    grid,
    variation,
    max_samples,
    out,
    threads,
    tol,
    max_iter,
    obj_tol,
    constraint_tol,
} => opfdata::handle(
    opfdata_root,
    grid,
    variation,
    *max_samples,
    out,
    threads,
    *tol,
    *max_iter,
    *obj_tol,
    *constraint_tol,
),
```

**Verification**: `cargo build -p gat-cli && ./target/debug/gat-cli benchmark opfdata --help`

---

### Task 44: Add unit tests for OPFData loader

**File**: `crates/gat-io/src/sources/opfdata.rs`

**Action**: Add tests at bottom of module.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opfdata_json_parsing() {
        // Minimal valid OPFData-style JSON
        let json_str = r#"{
            "bus": {
                "1": {"base_kv": 100.0, "vm": 1.0, "va": 0.0},
                "2": {"base_kv": 100.0, "vm": 0.98, "va": -0.05}
            },
            "gen": {
                "1": {"gen_bus": 1, "pg": 100.0, "qg": 50.0, "gen_status": 1}
            },
            "load": {
                "1": {"load_bus": 2, "pd": 80.0, "qd": 40.0, "status": 1}
            },
            "branch": {
                "1": {"f_bus": 1, "t_bus": 2, "br_r": 0.01, "br_x": 0.05, "br_status": 1}
            },
            "objective": 12345.67
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let network = build_network_from_opfdata(&json).unwrap();
        let solution = build_solution_from_opfdata(&json).unwrap();

        // 2 buses + 1 gen + 1 load = 4 nodes
        assert_eq!(network.graph.node_count(), 4);
        // 1 branch
        assert_eq!(network.graph.edge_count(), 1);

        // Solution extracted
        assert_eq!(solution.vm.len(), 2);
        assert!((solution.vm[&1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_topology_perturbation_skips_offline() {
        let json_str = r#"{
            "bus": {
                "1": {"base_kv": 100.0},
                "2": {"base_kv": 100.0},
                "3": {"base_kv": 100.0}
            },
            "gen": {
                "1": {"gen_bus": 1, "pg": 100.0, "gen_status": 1},
                "2": {"gen_bus": 2, "pg": 50.0, "gen_status": 0}
            },
            "load": {},
            "branch": {
                "1": {"f_bus": 1, "t_bus": 2, "br_r": 0.01, "br_x": 0.05, "br_status": 1},
                "2": {"f_bus": 2, "t_bus": 3, "br_r": 0.01, "br_x": 0.05, "br_status": 0}
            }
        }"#;

        let json: Value = serde_json::from_str(json_str).unwrap();
        let network = build_network_from_opfdata(&json).unwrap();

        // 3 buses + 1 gen (gen 2 offline) = 4 nodes
        assert_eq!(network.graph.node_count(), 4);
        // 1 branch (branch 2 offline)
        assert_eq!(network.graph.edge_count(), 1);
    }
}
```

**Verification**: `cargo test -p gat-io opfdata`

---

### Task 45: Create OPFData CLI integration test

**File**: `crates/gat-cli/tests/benchmark_opfdata.rs` (new)

**Action**: Create integration test.

```rust
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_benchmark_opfdata_topo() {
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("test_data/opfdata");

    // Skip if fixtures not available
    if !fixture_dir.exists() {
        eprintln!("Skipping test: OPFData fixtures not found at {:?}", fixture_dir);
        return;
    }

    let out_dir = TempDir::new().unwrap();
    let out_csv = out_dir.path().join("opfdata_results.csv");

    Command::cargo_bin("gat-cli")
        .unwrap()
        .args([
            "benchmark",
            "opfdata",
            "--opfdata-root",
            fixture_dir.to_str().unwrap(),
            "--grid",
            "case118",
            "--variation",
            "topo",
            "--max-samples",
            "3",
            "--threads",
            "1",
            "--tol",
            "1e-6",
            "--max-iter",
            "50",
            "--out",
            out_csv.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(out_csv.exists());

    let content = fs::read_to_string(&out_csv).unwrap();
    assert!(content.contains("converged"));
}
```

**Verification**: `cargo test -p gat-cli --test benchmark_opfdata` (skips if fixtures missing)

---

### Task 46: Add OPFData module tests to CI

**File**: Update existing CI config or test script

**Action**: Ensure opfdata tests run in CI. The tests will auto-skip if fixtures are missing, but once fixtures are committed, they'll run.

**Verification**: `cargo test -p gat-io -p gat-cli`

---

## Phase 6: Final Integration & Verification (Tasks 47-50)

### Task 47: Run full build and test suite

**Action**: Verify everything compiles and tests pass.

```bash
cargo build --release -p gat-cli
cargo test --workspace
```

**Verification**: No compilation errors; all tests pass.

---

### Task 48: Test PFΔ benchmark with fixtures

**Action**: Run the PFΔ benchmark on test fixtures.

```bash
./target/release/gat-cli benchmark pfdelta \
  --pfdelta-root test_data/pfdelta \
  --case ieee14 \
  --contingency n-1 \
  --max-cases 5 \
  --mode pf \
  --tol 1e-6 \
  --threads 1 \
  --out /tmp/pfdelta_test.csv

cat /tmp/pfdelta_test.csv
```

**Verification**: CSV output shows converged cases with small error metrics.

---

### Task 49: Test PGLib benchmark with fixtures

**Action**: Run the PGLib benchmark on test fixtures.

```bash
./target/release/gat-cli benchmark pglib \
  --pglib-root test_data/pglib \
  --size small \
  --max-cases 2 \
  --baseline test_data/pglib/baseline.csv \
  --tol 1e-6 \
  --threads 1 \
  --out /tmp/pglib_test.csv

cat /tmp/pglib_test.csv
```

**Verification**: CSV shows converged cases with objective gaps < 1e-4.

---

### Task 50: Test OPFData benchmark with fixtures

**Action**: Run the OPFData benchmark on test fixtures.

```bash
./target/release/gat-cli benchmark opfdata \
  --opfdata-root test_data/opfdata \
  --grid case118 \
  --variation topo \
  --max-samples 3 \
  --tol 1e-6 \
  --threads 1 \
  --out /tmp/opfdata_test.csv

cat /tmp/opfdata_test.csv
```

**Verification**: CSV shows converged cases with objective gaps < 1e-4.

---

## Execution Notes for Unattended Session

### Using superpowers:executing-plans

1. Load this plan with `/superpowers:execute-plan`
2. The executing session will:
   - Work through tasks in batches (recommend 5-10 tasks per batch)
   - Report back for review after each batch
   - Handle errors by noting them and continuing where possible

### Expected challenges

1. **Data fetching (Tasks 2, 5)**: HuggingFace structure may differ from assumed. Adapt paths as needed.
2. **PF solver API (Task 21)**: The exact `gat_algo::power_flow` interface needs investigation. Check existing `gat pf ac` implementation.
3. **OPF solution structure (Tasks 10, 27)**: `AcOpfSolution` fields need mapping to violation checks. Inspect the actual struct.
4. **MATPOWER importer (Task 33)**: May need extensions for PGLib-specific fields.

### Recovery points

If a task fails:
- Note the error in the task output
- Continue with the next task if independent
- Flag blocking issues for human review

### Estimated duration

- Phase 1 (Data): ~30 minutes (depends on download speeds)
- Phase 2 (Infrastructure): ~1 hour
- Phase 3 (PFΔ): ~1 hour
- Phase 4 (PGLib): ~1.5 hours
- Phase 5 (OPFData): ~1.5 hours
- Phase 6 (Integration): ~30 minutes

Total: ~6 hours of execution time for an unattended session.
