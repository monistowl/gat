+++
title = "Developer Internals"
description = "Architecture, crate structure, and contribution guide for GAT developers"
template = "section.html"
sort_by = "weight"
+++

# Developer Internals

Technical documentation for Rust developers contributing to GAT.

## Architecture

- **[CLI Architecture](cli-architecture/)** — Command hierarchy and module structure
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

- **[Contributing Guide](/contributing/)** — How to contribute
- **[CI/CD Pipeline](ci-cd/)** — Build, test, and release process
- **[Documentation Workflow](doc-workflow/)** — How docs are built

## API Reference

Rustdoc API documentation is available at:
- **[docs.rs/gat](https://docs.rs/gat)** (when published)
- Run `cargo doc --open` locally for the latest

## For Integrators

- **[MCP Server](mcp-onboarding/)** — Using GAT with AI agents
- **[Feature Matrix](feature-matrix/)** — Cargo features and targets
- **[Packaging](packaging/)** — Building releases
