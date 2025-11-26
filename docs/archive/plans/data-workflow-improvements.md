# Data Workflow Improvements

Proposals for improving benchmark dataset workflows, based on lessons learned from PGLib/PFΔ/OPFData integration.

## Quick Wins (< 1 day each)

### 1. `gat data fetch` command
Add a CLI command to fetch standard benchmark datasets:
```bash
gat data fetch pglib      # Clone PGLib-OPF from GitHub
gat data fetch pfdelta    # Download PFΔ from HuggingFace
gat data fetch opfdata    # Download OPFData from HuggingFace
gat data list             # Show available datasets and local status
```

**Why**: Eliminates manual git clone / hf download commands, handles authentication, shows progress, validates checksums.

**Implementation**: Add `commands/data.rs` with dataset registry (URLs, checksums, expected sizes).

### 2. Network validation on load
Add `Network::validate()` that checks:
- Total load > 0 MW (catches parser bugs like OPFData issue)
- At least one generator with capacity > min load
- All buses referenced by gen/load/branch exist
- No isolated buses (connectivity check)
- Branch impedances are reasonable (r > 0, x > 0)

**Why**: Fails fast with clear errors instead of cryptic solver failures.

**Implementation**: Add to `gat-core/src/network.rs`, call automatically on import.

### 3. Parser dry-run mode
```bash
gat import matpower case.m --dry-run
# Output: 14 buses, 5 gens (total 200 MW), 11 loads (total 150 MW), 20 branches
```

**Why**: Quickly verify a file parses correctly before running expensive solvers.

### 4. Baseline CSV auto-discovery
Currently benchmark commands require `--baseline path/to/baseline.csv`. Instead:
- Look for `BASELINE.md` or `baseline.csv` in dataset root
- Parse markdown tables automatically
- Cache parsed baselines

**Why**: PGLib includes BASELINE.md but we had to manually create baseline.csv.

---

## Medium Projects (1-3 days each)

### 5. Dataset format auto-detection
Unify import under single command:
```bash
gat import auto path/to/data
```

Auto-detect format by:
- `.m` file → MATPOWER
- `.raw` file → PSS/E
- `.json` with `grid.nodes` → OPFData/GridOpt
- `.json` with `pf_input` → PFΔ
- `.arrow`/`.parquet` → Arrow format
- Directory with `*.rdf` → CIM

**Why**: Users shouldn't need to know format names.

### 6. Benchmark result comparison
```bash
gat benchmark compare results/run1.csv results/run2.csv
# Shows: cases that regressed, improved, new failures
```

**Why**: Track solver improvements across code changes.

### 7. Progress reporting for large benchmarks
Currently benchmarks are silent during long runs. Add:
- Progress bar (X/Y cases, ETA)
- Streaming CSV output (partial results on interrupt)
- Summary stats every N cases

**Why**: 66 PGLib cases or 300k OPFData samples take minutes/hours.

---

## Large Projects (1+ week)

### 8. Unified Dataset Abstraction
Create `Dataset` trait:
```rust
trait Dataset {
    fn name(&self) -> &str;
    fn cases(&self) -> impl Iterator<Item = Case>;
    fn reference_solution(&self, case: &Case) -> Option<Solution>;
}

// Implementations for PGLib, PFΔ, OPFData, etc.
```

**Why**: Benchmark commands currently have separate logic for each dataset. Unifying enables:
- Single `gat benchmark run --dataset X` command
- Cross-dataset comparisons
- Pluggable new datasets

### 9. Solver Cost Function Implementation
The OPF solver currently returns `objective_value = 0` because cost minimization isn't implemented. Need:
- Parse `gencost` from MATPOWER files (polynomial/piecewise)
- Add cost terms to QP objective in `ac_opf.rs`
- Validate against PGLib reference objectives

**Why**: 100% objective gap makes OPF benchmarks meaningless for accuracy assessment.

### 10. Test Data Generation
```bash
gat data generate --buses 100 --topology radial --load-profile residential
```

Generate synthetic test cases with known solutions for:
- Unit testing solver edge cases
- Performance profiling at various scales
- CI/CD regression testing without large downloads

---

## Lessons Learned

1. **Always validate parsed data** - The OPFData bug (loads=0) would have been caught by a simple `total_load > 0` check.

2. **Test with real data early** - The test_data fixtures were too small to reveal format differences.

3. **Document expected formats** - OPFData JSON structure wasn't documented in our code, leading to wrong assumptions about `context` being a dict vs list.

4. **Support multiple format variants** - PGLib can be flat .m files or subdirectories; both should work.

5. **Fail loudly on parse errors** - Silent defaults (e.g., `unwrap_or(0.0)`) hide bugs. Consider requiring explicit handling.
