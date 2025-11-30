+++
title = "GAT – Grid Analysis Toolkit"
description = "A full grid lab in a single binary. AC-OPF, N-1 contingency analysis, and state estimation without the Python headaches or commercial invoices."
transparent = true
+++

# Stop Installing. Start Solving.

**GAT** is a full grid lab in a single binary. Run AC-OPF, N-1 contingency analysis, and state estimation without the Python headaches or commercial invoices.

## Quick Start

```bash
# Install using the modular installer
curl -fsSL https://github.com/monistowl/gat/releases/download/v0.5.0/install-modular.sh | bash
export PATH="$HOME/.gat/bin:$PATH"

# Import a MATPOWER case and run power flow
gat import matpower --m grid.m -o grid.arrow
gat pf dc grid.arrow --out flows.parquet
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
- **[Contributing](@/contributing.md)** — How to contribute
- **[Blog](/blog/)** — Development updates
