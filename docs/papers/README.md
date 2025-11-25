# GAT Research Papers

This directory contains research papers and preprints related to the Grid Analysis Toolkit.

## Contents

### `gat-arxiv-preprint.tex`

**Title**: GAT: A High-Performance Rust Toolkit for Power System Analysis

**Abstract**: Presents GAT as an open-source command-line toolkit for power system modeling, analysis, and optimization. Validates solvers against three benchmark datasets: PFΔ, PGLib-OPF, and OPFData.

**Key Results**:
- 100% convergence on PFΔ and OPFData test cases
- Sub-millisecond solve times (0.03-0.59 ms) for networks up to 118 buses
- Exact agreement with reference solutions for power flow

**Citations**:
1. MATPOWER (Zimmerman et al., 2011)
2. PowerModels.jl (Coffrin et al., 2018)
3. pandapower (Thurner et al., 2018)
4. PyPSA (Brown et al., 2018)
5. PGLib-OPF (Babaeinejadsarookolaee et al., 2019)
6. OPFData (Piloto et al., 2024)

## Building

To compile the LaTeX source:

```bash
cd docs/papers
pdflatex gat-arxiv-preprint.tex
bibtex gat-arxiv-preprint
pdflatex gat-arxiv-preprint.tex
pdflatex gat-arxiv-preprint.tex
```

Or use an online LaTeX editor like Overleaf.

## Reproducing Benchmark Results

```bash
# PFΔ benchmark
gat benchmark pfdelta --pfdelta-root test_data/pfdelta -o pfdelta_results.csv

# PGLib-OPF benchmark
gat benchmark pglib --pglib-dir test_data/pglib -o pglib_results.csv

# OPFData benchmark (requires downloaded data)
gat benchmark opfdata --opfdata-dir /path/to/opfdata -o opfdata_results.csv
```

See `docs/guide/datasets.md` for information on obtaining benchmark datasets.
