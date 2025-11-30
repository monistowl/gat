# GAT Research Papers

This directory contains research papers and preprints related to the Grid Analysis Toolkit.

## Contents

### `gat-arxiv-preprint-draft5.tex` (Latest - November 2024)

**Title**: GAT: A High-Performance Rust Toolkit for Power System Optimal Power Flow — Comprehensive Technical Reference

**Abstract**: Presents GAT as an open-source command-line toolkit for power system optimal power flow (OPF) implemented in Rust. Provides a comprehensive solver hierarchy spanning four levels of fidelity: economic dispatch, DC-OPF, SOCP relaxation, and full nonlinear AC-OPF. The AC-OPF solver supports both a penalty-based L-BFGS method (pure Rust) and an IPOPT-backed interior-point method with analytical Jacobian and Hessian computation.

**Key Results**:
- **< 0.01% objective gap** on IEEE 14-bus and IEEE 118-bus vs. PGLib reference
- Complete solver hierarchy: Economic Dispatch → DC → SOCP → AC-NLP
- Analytical Jacobian and Hessian for IPOPT with full thermal constraint support
- 15-page comprehensive technical reference with mathematical formulations

**Validated Results** (PGLib-OPF):
| Case | GAT Objective | Reference | Gap |
|------|--------------|-----------|-----|
| case14_ieee | $2,178.08/hr | $2,178.10/hr | -0.00% |
| case118_ieee | $97,213.61/hr | $97,214.00/hr | -0.00% |

**Contents**:
1. Mathematical Background (Y-bus, power flow equations)
2. AC-OPF Formulation (NLP in polar coordinates)
3. DC-OPF (Linear approximation)
4. SOCP Relaxation (Branch-flow model, exactness conditions)
5. IPOPT Solver (Analytical derivatives, warm-start)
6. Solver Pipeline Flow Diagrams
7. Benchmark Validation Results
8. Case Study: IEEE 118-bus convergence debugging
9. Appendices (Algorithm pseudocode, Jacobian sparsity)

**Citations**: 18 references including Carpentier (1962), MATPOWER, PowerModels.jl, PGLib-OPF, IPOPT, Clarabel, and foundational SOCP relaxation papers.

---

### Previous Drafts

- `gat-arxiv-preprint-draft4.tex` — Earlier version with DC-OPF and SOCP focus
- `gat-arxiv-preprint-draft3.tex` — Benchmark results version
- `gat-arxiv-preprint-draft2.tex` — Initial architecture overview
- `gat-arxiv-preprint-draft1.tex` — First draft

## Building

To compile the LaTeX source:

```bash
cd docs/papers

# Draft 5 (latest)
pdflatex gat-arxiv-preprint-draft5.tex
pdflatex gat-arxiv-preprint-draft5.tex  # Second pass for cross-references
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
