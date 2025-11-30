+++
title = "Developer Internals"
description = "Architecture, crate structure, and contribution guide for GAT developers"
template = "section.html"
sort_by = "weight"
+++

# Developer Internals

Technical documentation for Rust developers contributing to GAT.

## Architecture

- **[Data Flow & Design](data-flow/)** — How data moves through GAT, from input to solution
- **[CLI Architecture](cli-architecture/)** — Command hierarchy and module structure
- **[Solver Architecture](solver-architecture/)** — Native solver plugin system, subprocess IPC, vendored builds
- **[Crate Layout](#crates)** — Workspace organization

## Crates

GAT is organized as a Cargo workspace with focused crates:

| Crate | Purpose |
|-------|---------|
| `gat-core` | Network graph, data structures, base types |
| `gat-algo` | Solvers: power flow, OPF, state estimation |
| `gat-io` | File I/O: MATPOWER, PSS/E, CIM |
| `gat-cli` | Command-line interface |
| `gat-dist` | Distribution system analysis |
| `gat-adms` | ADMS domain features |
| `gat-derms` | DERMS domain features |
| `gat-scenarios` | Scenario management and batch runs |

## Contributing

- **[Contributing Guide](@/contributing.md)** — How to contribute
- **[CI/CD Pipeline](ci-cd/)** — Build, test, and release process
- **[Documentation Workflow](doc-workflow/)** — How docs are built

## API Reference

Rustdoc API documentation:
- Run `cargo doc --open` locally for the latest
- docs.rs will be available when published to crates.io

## For Integrators

- **[MCP Server](mcp-onboarding/)** — Using GAT with AI agents
- **[Feature Matrix](feature-matrix/)** — Cargo features and targets
- **[Packaging](packaging/)** — Building releases
