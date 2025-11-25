+++
title = "GAT – Grid Analysis Toolkit"
description = "A full grid lab in a single binary. AC-OPF, N-1 contingency analysis, and state estimation without the Python headaches or commercial invoices."
sort_by = "date"
paginate_by = 5
+++

# Stop Installing. Start Solving.

**GAT** is a full grid lab in a single binary. Run AC-OPF, N-1 contingency analysis, and state estimation without the Python headaches or commercial invoices.

## Quick Start

```bash
# Download single binary (no dependencies)
curl -L https://github.com/monistowl/gat/releases/latest/download/gat-linux-x86_64 -o gat
chmod +x gat

# Run power flow on a standard test case
./gat pf case14.m

# Run optimal power flow
./gat opf case14.m
```

## Why GAT?

- **Single Binary** — No Python, no Julia, no MATLAB licenses
- **Fast** — Native Rust performance, parallel execution
- **Standards-Based** — MATPOWER, PSS/E, and CIM support
- **Scriptable** — JSON output, clean exit codes, composable commands

## Documentation

- **[User Guide](/guide/)** — Getting started and CLI usage
- **[Developer Internals](/internals/)** — Architecture for contributors
- **[Reference & Theory](/reference/)** — Algorithms and power systems background

## Get Involved

- **[GitHub](https://github.com/monistowl/gat)** — Source code and issues
- **[Contributing](/contributing/)** — How to contribute
- **[Blog](/blog/)** — Development updates
