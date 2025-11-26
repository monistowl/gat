+++
title = "FAQ"
weight = 100
description = "Frequently asked questions about GAT"
template = "page.html"
+++

# Frequently Asked Questions

## Licensing & Cost

### Is GAT really free?

**Yes, for most use cases.** GAT is free for:
- Academic research and education
- Personal projects
- Internal business use (not SaaS, not competitive)
- Open-source projects

GAT uses the **PolyForm Shield License 1.0.0**, which is free-for-most and source-available.

### When do I need commercial licensing?

You need commercial licensing if you:
- Build a **SaaS product** using GAT
- Offer **consulting services** based on GAT
- Use GAT in a **competitive product** (e.g., selling energy optimization software)
- Want commercial support and indemnification

**To inquire:** [Open a discussion]({{ config.extra.repo_url }}/discussions) or email through GitHub.

### Can I use GAT in my startup?

**Usually yes, at no cost.** If your startup:
- Uses GAT internally for your own analysis — ✅ **Free**
- Builds analysis tools for customers — ✅ **Free** (internal business use)
- Sells energy optimization as a service — ❌ **Needs commercial license**
- Resells GAT functionality directly — ❌ **Needs commercial license**

Unclear? [Start a discussion]({{ config.extra.repo_url }}/discussions) — we're friendly about this.

### Is the source code available?

**Yes.** The full source is on [GitHub]({{ config.extra.repo_url }}) under PolyForm Shield. You can:
- Review the code
- Audit algorithms and implementations
- Build modified versions for internal use
- Learn from the codebase

You cannot publish a competing product without a commercial license.

### Can I modify GAT for my use case?

**Yes.** You can modify GAT for internal use at no cost. If you want to:
- Publish modifications as open source — Contact us
- Distribute modified versions — Need commercial license
- Keep modifications private for internal use — ✅ **Free**

## Technical Questions

### How fast is GAT really?

Very fast. Here are realistic benchmarks on modern hardware:

| Grid Size | DC Power Flow | AC Power Flow |
|-----------|---------------|---------------|
| 9 buses | ~10ms | ~50ms |
| 30 buses | ~15ms | ~80ms |
| 118 buses | ~30ms | ~150ms |
| 1000+ buses | ~100ms | ~500ms |
| 10,000 buses | ~500ms | ~3s |

Times vary by:
- **Hardware** — Older laptops will be slower
- **Solver** — Clarabel (default) vs CBC vs HiGHS
- **Tolerance** — Tighter tolerances take longer
- **Convergence** — Ill-conditioned grids need more iterations

See [Benchmarks](/guide/benchmark/) for detailed comparisons.

### How does GAT compare to MATPOWER?

| Aspect | GAT | MATPOWER |
|--------|-----|----------|
| **Speed** | 10-100x faster | ~1s for 30-bus case |
| **Language** | Rust (compiled) | MATLAB (interpreted) |
| **Cost** | Free | Free (open source) |
| **License** | Commercial available | BSD |
| **Solvers** | Clarabel, CBC, HiGHS | MATPOWER default |
| **Features** | More (TUI, Arrow, agent-ready) | Mature, well-tested |
| **Python Bindings** | No (use Parquet output) | Yes (via Octave) |
| **AI Integration** | MCP Server | Not built-in |

**TL;DR:** GAT is faster, modern, and designed for automation and AI. MATPOWER is battle-tested and has more academic history.

### How does GAT compare to PyPower or PowerWorld?

**vs PyPower:**
- GAT is 50-100x faster (compiled vs Python)
- GAT has more solvers
- PyPower has more academic maturity
- Both are free/open

**vs PowerWorld:**
- GAT is free; PowerWorld costs $$$ per seat
- PowerWorld has more enterprise features (visualizations, stability analysis)
- GAT is faster and more portable
- PowerWorld is trusted in utilities

### Can GAT handle my grid size?

GAT can handle:
- **Small** (9-30 buses) — Instant (~10-50ms)
- **Medium** (100-1000 buses) — <500ms
- **Large** (1000-10,000 buses) — <3 seconds
- **Very Large** (10k+) — Seconds to minutes

For production systems at scale, you may need:
- Multiple machines (distributed computation)
- Custom solver tuning
- Commercial solver options (Gurobi, MOSEK)

See [Scaling](/internals/scaling/) for large-system guidance.

### Which solver should I use?

**Clarabel (default)**
- ✅ Good for most cases
- ✅ Stable and reliable
- ✅ Handles infeasibility well
- ❌ Slower on very large systems

**CBC**
- ✅ Fast for large systems
- ✅ Robust MIP support
- ❌ Less stable on some edge cases

**HiGHS**
- ✅ Fast and modern
- ✅ Good presolve
- ✅ Handles degeneracy well
- ❌ Newer (less field history)

**Recommendation:** Start with Clarabel (default). If slow, try HiGHS. If still slow, consider distributed computing.

### Does GAT support my file format?

Supported input formats:
- ✅ **MATPOWER** (.m files)
- ✅ **Pandapower** (.pkl files)
- ✅ **CSV** (with schema)
- ✅ **JSON** (power flow specs)

Supported output formats:
- ✅ **Parquet** (recommended — columnar, compressed, fast)
- ✅ **Arrow IPC** (for inter-process communication)
- ✅ **JSON** (for debugging)
- ✅ **CSV** (not recommended — large files)

**Can't find your format?** [Open an issue]({{ config.extra.repo_url }}/issues) with a sample file.

### How do I integrate GAT with my tool/language?

**Python:** Use Parquet output, read with `polars` or `pandas`
```python
import polars as pl
results = pl.read_parquet('flows.parquet')
```

**JavaScript/TypeScript:** Use Apache Arrow or Parquet libraries

**Julia/R:** Use Parquet packages

**SQL:** Use DuckDB to query Parquet directly
```bash
duckdb :memory: "SELECT * FROM read_parquet('results.parquet')"
```

**C/C++/Rust:** Use Arrow C Data Interface

See [Integration Guide](/guide/overview/#data-architecture) for details.

## Usage & Workflow

### What's the difference between DC and AC power flow?

**DC Power Flow**
- Fast, linearized approximation
- Ignores reactive power
- Assumes constant voltage (1.0 pu)
- Good for: Economic dispatch, N-1 screening, real-time applications
- Speed: ~10ms for medium grids

**AC Power Flow**
- Full nonlinear equations
- Accounts for reactive power and voltages
- More accurate, slower
- Good for: Detailed analysis, voltage stability, real power losses
- Speed: ~50-100ms for medium grids

**Rule of thumb:** Use DC for screening, AC for verification.

### How do I run batch analysis?

Use GAT's **manifest system** for batch automation:

```toml
# manifest.toml
[runs]
base_case = { network = "grid.m", name = "Base Case" }
contingency_n1 = { network = "grid.m", contingencies = true }

[analysis]
analysis_type = "power_flow"
method = "ac"
```

Then run:
```bash
gat runs describe manifest.toml
gat runs execute manifest.toml
```

See [Manifests](/internals/cli-architecture/#manifest-driven-workflows) for more.

### Can I parallelize analysis across multiple machines?

Not out-of-the-box, but it's easy to script:

```bash
# Run contingencies in parallel across cores
for i in {1..100}; do
  gat pf ac grid.m --contingency $i --out contingency_$i.parquet &
done
wait
```

For cloud distribution:
- Use the modular installer to deploy to multiple servers
- Use MCP Server for agent-based distribution
- Wrap GAT in containers (Docker) for orchestration

See [Scaling](/internals/scaling/) for distributed patterns.

## Installation & Setup

### Why isn't `cargo install gat-cli` working?

We don't publish to crates.io yet. Use the modular installer instead:

```bash
curl -fsSL \
  https://github.com/monistowl/gat/releases/download/v0.3.4/install-modular.sh \
  | bash
```

This downloads pre-built binaries, which is much faster than building from source.

### What if the modular installer fails?

See [Installation Troubleshooting](/guide/install-verify/#troubleshooting) for:
- Network issues
- Missing `jq` dependency
- Permission problems
- Platform-specific issues

The installer automatically falls back to building from source if binaries aren't available.

### Can I build GAT from source?

Yes, requires Rust:

```bash
git clone https://github.com/monistowl/gat.git
cd gat
cargo build -p gat-cli --release
```

Binary will be at `target/release/gat-cli`.

See [Contributing Guide](/contributing/) for development setup.

### Do I need to compile the solvers?

**No.** Solver backends (CBC, HiGHS, Clarabel) are:
- Included in the pre-built binaries
- Compiled as part of the Rust build
- Not separate installations

If you have system CBC installed, GAT will use it; otherwise, it uses the bundled version.

## Features & Capabilities

### What analyses can GAT perform?

**Basic**
- Power flow (DC, AC)
- Optimal power flow (DC, AC)
- State estimation (weighted least squares)
- Contingency analysis (N-1 screening)

**Advanced**
- Time series analysis (multi-period optimization)
- Reliability metrics (LOLE, EUE)
- Domain-specific workflows (ADMS, DERMS, VVO)
- Graph analysis (islands, cycles, meshing)

See [Feature Matrix](/internals/feature-matrix/) for the full list.

### Does GAT support renewable energy?

Yes:
- Variable renewable generation (wind, solar)
- Time series analysis for forecast integration
- Flexibility constraints (ramp limits, min-up/down)
- Renewable energy scenarios

See [Time Series Guide](/guide/ts/) for examples.

### Can GAT handle distribution networks?

Yes, specifically. Domain-specific features include:
- **ADMS** — Advanced Distribution Management
- **DERMS** — Distributed Energy Resources
- **VVO** — Volt-VAR Optimization
- **FLISR** — Fault Location, Isolation, Service Restoration

See [ADMS Guide](/guide/adms/) and [DERMS Guide](/guide/derms/).

### Does GAT support market clearing / LMP?

Yes. GAT can compute:
- **Locational Marginal Prices (LMP)** from OPF results
- **Congestion rents** and **loss recovery**
- **Binding constraints** and **shadow prices**

See [OPF Guide](/guide/opf/#locational-marginal-prices) for examples.

## Getting Help

### Where should I ask questions?

- **Discussions:** [Ask anything]({{ config.extra.repo_url }}/discussions) — best for questions
- **Issues:** [Report bugs]({{ config.extra.repo_url }}/issues) — for bugs and feature requests
- **Documentation:** [Full docs](/docs/) — for detailed guides

### How do I report a bug?

[Open an issue]({{ config.extra.repo_url }}/issues) with:
- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Output of `gat --version`
- OS and hardware info
- Minimal example grid (if applicable)

### How can I contribute?

See [Contributing Guide](/contributing/) for:
- Code contributions
- Documentation improvements
- Bug reports and feature requests
- Testing and feedback

We welcome all contributions, no matter how small!

### Where do I get sample data?

**In the GAT repository:**
```bash
git clone https://github.com/monistowl/gat.git
cd gat/examples
ls -la *.m  # MATPOWER test cases
```

**Public datasets:**
- [MATPOWER Cases](https://matpower.org/docs/ref/matpower5.1/caseformat.html) — IEEE test cases
- [pglib-opf](https://power-grid-lib.github.io/) — Diverse OPF test cases
- [Open Power System Data](https://data.open-power-system-data.org/) — Real-world European grid data

### What if I disagree with a result?

Good question! To debug:

1. **Compare with other tools** — Run the same analysis in MATPOWER or PyPower
2. **Check convergence** — Did the solver converge? (GAT reports this)
3. **Verify inputs** — Is your grid data correct?
4. **Relax tolerance** — Try `--tolerance 1e-3` to see if it's a precision issue
5. **Check the code** — [Review the solver implementation]({{ config.extra.repo_url }}/blob/main/crates/gat-algo/src/solver)

[Open an issue]({{ config.extra.repo_url }}/issues) with your analysis and we'll investigate.

## Performance & Optimization

### How can I make GAT faster?

1. **Use DC power flow instead of AC** — 5-10x faster
2. **Reduce contingencies** — Use DC screening first
3. **Adjust tolerance** — Looser tolerance = faster convergence
4. **Choose faster solver** — HiGHS is often faster than Clarabel
5. **Parallelize manually** — Run independent analyses in parallel
6. **Reduce network size** — Pre-process to remove unrelated areas

See [Scaling Guide](/internals/scaling/) for benchmarked optimizations.

### Does GAT use all my CPU cores?

Partially. GAT parallelizes:
- ✅ Matrix factorization (via solver)
- ✅ Contingency analysis (multiple cases in parallel)
- ✅ Independent analyses (time series periods)
- ❌ Single power flow solve (uses one core)

For single-case analysis, you can't parallelize further (it's inherently serial).

### How much memory does GAT need?

Typical memory usage:

| Grid Size | DC Power Flow | AC Power Flow |
|-----------|---------------|---------------|
| 30 buses | ~2MB | ~5MB |
| 118 buses | ~5MB | ~15MB |
| 1000 buses | ~50MB | ~150MB |
| 10,000 buses | ~500MB | ~1.5GB |

Memory is mostly for:
- Jacobian/Hessian matrices
- Solver internal data structures
- Solution vectors

Even very large grids fit in standard laptop RAM.

## Community & Support

### Is there an active community?

Yes! Join via:
- **Discussions:** [GitHub Discussions]({{ config.extra.repo_url }}/discussions)
- **Issues:** [GitHub Issues]({{ config.extra.repo_url }}/issues)
- **Contributors:** See [GitHub Contributors]({{ config.extra.repo_url }}/graphs/contributors)

### How often is GAT updated?

Currently:
- **Release cycle:** ~Monthly
- **Security patches:** As needed
- **Major features:** Every 3-6 months

Follow [releases]({{ config.extra.repo_url }}/releases) for updates.

### Is GAT production-ready?

**Yes, for most uses.** GAT is:
- ✅ Stable and well-tested (500+ tests)
- ✅ Used in academic research and startups
- ✅ Actively maintained
- ✅ Production power flow code

Caveat:
- Some experimental features (marked as such)
- Newer than MATPOWER (more field history for MATPOWER)
- Windows support in progress

For critical systems, test thoroughly in your environment.

---

**Didn't find your question?** [Start a discussion]({{ config.extra.repo_url }}/discussions) or [open an issue]({{ config.extra.repo_url }}/issues)!
