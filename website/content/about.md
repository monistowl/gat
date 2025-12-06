+++
title = "About GAT"
weight = 100
description = "The story behind the Grid Analysis Toolkit"
template = "page.html"
+++

# About GAT

## The Grid Analysis Toolkit

GAT (Grid Analysis Toolkit) is a **high-performance power systems analysis tool** built in Rust. It brings industrial-grade optimization, power flow, and reliability analysis to a single, dependency-free binary.

## The Problem

Power systems analysis has traditionally required:

- **Expensive commercial software** with per-seat licensing
- **Complex Python environments** that break across updates
- **MATLAB dependencies** that aren't portable or reproducible
- **Vendor lock-in** with proprietary formats and APIs
- **Slow execution** that makes iterative analysis painful

Meanwhile:
- **Researchers** struggle to reproduce published results
- **Startups** can't afford $15k/seat commercial licenses
- **Operators** need air-gapped solutions without license servers
- **AI agents** need deterministic, fast physics engines

## The Solution

GAT reimagines power systems tools for the modern era:

### 🚀 **Fast**
Built in Rust with native performance. AC-OPF on 12k-bus systems in milliseconds, not minutes.

### 📦 **Portable**
Single binary. No dependencies. No Python environments. No license servers. Works behind firewalls.

### 🗄️ **Modern Data Stack**
Arrow/Parquet-first. Drop results straight into Polars, DuckDB, or Pandas. No parsing text files.

### 🤖 **Agent-Ready**
Structured outputs, deterministic behavior, and MCP server integration make GAT the perfect physics engine for AI.

### 💰 **Accessible**
Free for academic and personal use. Affordable commercial licensing for startups. No vendor lock-in.

### 🔓 **Source-Available**
Review the code. Understand the algorithms. Verify the math. Contribute improvements.

## Philosophy

### Research-First
GAT was built for power systems researchers who were tired of:
- Environments that break between paper submissions
- Results that can't be reproduced
- Licenses that expire mid-thesis
- Tools that don't match published algorithms

Academic use is **free forever**. No strings attached.

### Production-Ready
While many academic tools are prototypes, GAT is designed for production:
- Industrial solver backends (CBC, HiGHS, Clarabel)
- Comprehensive error handling and validation
- Deterministic, reproducible results
- Extensive testing and CI/CD
- Professional documentation

### Open Development
Development happens in the open:
- Public GitHub repository
- Issue tracking and discussions
- Community contributions welcome
- Transparent roadmap

## Technology Stack

- **Language:** Rust (for speed, safety, and portability)
- **Solvers:** CBC, HiGHS, Clarabel (open-source LP/QP/SOCP)
- **Data:** Apache Arrow, Parquet (columnar, zero-copy)
- **CLI:** clap (full-featured command-line interface)
- **TUI:** Ratatui (terminal user interface for exploration)
- **Platforms:** Linux, macOS, Windows (cross-platform from day one)

## Features

### Power Flow Analysis
- **DC Power Flow** - Fast linearized analysis
- **AC Power Flow** - Full nonlinear Newton-Raphson
- Solver selection, tolerance control, Parquet output

### Optimal Power Flow
- **DC-OPF** - Economic dispatch with DC approximation
- **AC-OPF** - Full nonlinear optimization
- Piecewise generator costs, transmission limits

### Reliability Analysis
- **LOLE** (Loss of Load Expectation)
- **EUE** (Expected Unserved Energy)
- Scenario-based analysis

### Contingency Analysis
- **N-1 Screening** - Fast contingency enumeration
- Performance metrics per contingency
- Parallelized evaluation

### Time Series
- Multi-period optimization
- Load forecasting integration
- Scenario analysis

### Domain-Specific
- **ADMS** - Advanced Distribution Management
- **DERMS** - Distributed Energy Resources
- **VVO** - Volt-VAR Optimization
- **FLISR** - Fault Location, Isolation, Service Restoration

## Use Cases

### Academia
- Reproducible research
- Thesis and dissertation work
- Teaching power systems courses
- Publishing verifiable results

### Startups
- Building energy optimization platforms
- DER aggregation and VPP management
- Grid planning tools
- Analysis-as-a-service

### Operators
- Internal planning studies
- Reliability assessment
- Scenario modeling
- Air-gapped deployments

### AI/Agents
- Physics-based constraint checking
- LMP and OPF for energy agents
- Deterministic grid simulations
- Tool-use compatible outputs

## Project Status

**Current Version:** v0.5.5

**Status:** Production-ready, actively developed

**Test Coverage:** 500+ tests across core, CLI, and TUI

**Platforms:** Linux, macOS (Windows support in progress)

## Team

GAT is developed and maintained by **Tom Wilson** with contributions from the open-source community.

### Contributing

We welcome contributions! See our [Contributing Guide](@/contributing.md) for details on:
- Code contributions
- Documentation improvements
- Bug reports and feature requests
- Testing and feedback

## Licensing

GAT is licensed under the **PolyForm Shield License 1.0.0**:

- ✅ **Free** for academic, personal, and internal business use
- 💼 **Commercial licensing** available for SaaS, consulting, and competitive use
- 🔓 **Source-available** for transparency and verification

See our [License & Terms](@/license.md) page for complete details.

## Roadmap

See our [project roadmap](https://github.com/monistowl/gat/blob/main/docs/ROADMAP.md) for upcoming features and long-term plans.

**Near-term priorities:**
- Enhanced TUI with real-time visualization
- Additional solver backends (Gurobi, MOSEK)
- Improved documentation and examples
- Extended domain workflows (VVO, FLISR)
- Performance optimizations for larger systems

## Community

- **GitHub:** [github.com/monistowl/gat](https://github.com/monistowl/gat)
- **Issues:** [Report bugs, request features](https://github.com/monistowl/gat/issues)
- **Discussions:** [Community Q&A](https://github.com/monistowl/gat/discussions)
- **Documentation:** [Full docs](@/guide/_index.md)

## Contact

For commercial licensing, partnerships, or general inquiries:

- **GitHub Issues:** [Open an issue](https://github.com/monistowl/gat/issues/new)
- **Discussions:** [Start a discussion](https://github.com/monistowl/gat/discussions)

---

**GAT: The grid analysis toolkit researchers always wished existed.**
