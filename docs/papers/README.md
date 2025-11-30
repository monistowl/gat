# GAT Research Papers

This directory contains research papers and preprints related to the Grid Analysis Toolkit.

## Contents

### `gat-arxiv-preprint-draft6.tex` (Latest - November 2024)

**Title**: GAT: A High-Performance Rust Toolkit for Power System Analysis — Comprehensive Technical Reference for Doctoral-Level Engineers

**Author**: Tom Wilson

**Abstract**: Presents the Grid Analysis Toolkit (GAT), an open-source command-line toolkit for power system analysis implemented in Rust. This expanded 24-page technical reference documents GAT's complete solver hierarchy for optimal power flow (OPF)—from sub-millisecond economic dispatch through DC-OPF, SOCP relaxation, and full nonlinear AC-OPF with IPOPT—alongside state estimation, N-k contingency analysis, and time-series dispatch. Details the framework's design decisions rooted in Rust's type system and memory safety guarantees, the challenges of parsing heterogeneous power system datasets (MATPOWER, PSS/E, CIM, pandapower), and the mathematical foundations underlying each analysis module.

**Key Results**:
- **< 0.01% objective gap** on IEEE 14-bus and IEEE 118-bus vs. PGLib reference
- Complete solver hierarchy: Economic Dispatch → DC → SOCP → AC-NLP
- Analytical Jacobian and Hessian for IPOPT with full thermal constraint support
- 24-page doctoral-level technical reference with complete mathematical formulations

**Major Sections**:

*Part I: Framework Architecture*
1. Introduction and Why Rust for Power Systems
2. Crate Architecture (gat-core, gat-algo, gat-io, gat-cli) with TikZ dependency diagram
3. Type-Driven Design (newtype pattern, algebraic data types, builder pattern)
4. Dataset Challenges and Validation (format heterogeneity, per-unit normalization)

*Part II: Mathematical Foundations*
5. Newton-Raphson Power Flow (full Jacobian element derivations)
6. Optimal Power Flow Solver Hierarchy
7. Economic Dispatch, DC-OPF, SOCP Relaxation
8. AC-OPF with IPOPT (analytical derivatives)
9. Contingency Analysis (PTDF/LODF matrices)
10. State Estimation via Weighted Least Squares

*Part III: Implementation and Benchmarks*
11. Benchmark Validation (PGLib-OPF, PFΔ, OPFData)
12. Profiling and Performance Analysis
13. Conclusions and Future Work

**Validated Results** (PGLib-OPF):
| Case | GAT Objective | Reference | Gap |
|------|--------------|-----------|-----|
| case14_ieee | $2,178.08/hr | $2,178.10/hr | -0.00% |
| case118_ieee | $97,213.61/hr | $97,214.00/hr | -0.00% |

**Citations**: 25+ references including Carpentier (1962), MATPOWER, PowerModels.jl, PGLib-OPF, IPOPT, Clarabel, Wood & Wollenberg, and foundational SOCP relaxation papers.

---

### Previous Drafts

- `gat-arxiv-preprint-draft5.tex` — 15-page technical reference with OPF solver hierarchy
- `gat-arxiv-preprint-draft4.tex` — Earlier version with DC-OPF and SOCP focus
- `gat-arxiv-preprint-draft3.tex` — Benchmark results version
- `gat-arxiv-preprint-draft2.tex` — Initial architecture overview
- `gat-arxiv-preprint-draft1.tex` — First draft

## Building

To compile the LaTeX source:

```bash
cd docs/papers

# Draft 6 (latest)
pdflatex gat-arxiv-preprint-draft6.tex
pdflatex gat-arxiv-preprint-draft6.tex  # Second pass for cross-references
```

Or use an online LaTeX editor like Overleaf.

## Reproducing Benchmark Results

```bash
# Run AC-OPF validation against PGLib
./scripts/with-ipopt.sh cargo test --features solver-ipopt -p gat-algo --test debug_case118 -- --test-threads=1

# PGLib benchmark (full suite)
gat benchmark pglib --pglib-dir test_data/pglib -o pglib_results.csv

# PFΔ benchmark
gat benchmark pfdelta --pfdelta-root test_data/pfdelta -o pfdelta_results.csv

# OPFData benchmark (requires downloaded data)
gat benchmark opfdata --opfdata-dir /path/to/opfdata -o opfdata_results.csv
```

See `docs/guide/datasets.md` for information on obtaining benchmark datasets.

## Development Notes

The IPOPT-based AC-OPF solver (`crates/gat-algo/src/opf/ac_nlp/`) includes:

- **`ipopt_solver.rs`**: IPOPT problem wrapper with warm-start support
- **`jacobian.rs`**: Analytical constraint Jacobian (power balance + thermal limits)
- **`hessian.rs`**: Analytical Lagrangian Hessian
- **`diagnostics.rs`**: Introspection helpers for debugging convergence issues

Key bug fix (November 2024): Corrected sign error in to-side thermal constraint Jacobian (`jacobian.rs:549-554`) that was causing convergence failures on case118.
