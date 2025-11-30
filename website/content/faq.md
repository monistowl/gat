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

**To inquire:** [Open a discussion](https://github.com/monistowl/gat/discussions) or email through GitHub.

---

## Power Systems Fundamentals

### Why use the per-unit system instead of actual values?

The per-unit (p.u.) system normalizes all quantities to dimensionless ratios around 1.0:

1. **Numerical stability**: Avoids floating-point issues when mixing voltages (120 V to 765 kV) and powers (kW to GW)
2. **Simplified calculations**: Transformer turns ratios disappear; ideal transformers become 1:1
3. **Quick sanity checks**: Normal voltages are ~1.0 p.u., abnormal values stand out immediately

See [Units & Conventions](/reference/units-conventions/) for the full derivation.

### What's the difference between real power (P) and reactive power (Q)?

| Property | Real Power (P) | Reactive Power (Q) |
|----------|---------------|-------------------|
| Units | Watts (W), MW | VAR, MVAR |
| Physical meaning | Actual energy transfer | Energy oscillation (no net transfer) |
| What it powers | Motors, lights, heat | Magnetic fields in motors/transformers |

**Why Q matters**: Reactive power affects voltage levels. Too little Q → voltage drops. Too much Q → voltage rises.

### What is a slack bus (reference bus)?

The **slack bus** serves two purposes:

1. **Angle reference**: All other bus angles are measured relative to the slack (θ = 0°)
2. **Power balance**: Absorbs the mismatch between total generation and total load + losses

**Choosing a slack bus**: Pick a large generator with sufficient capacity headroom. In GAT, identified by `bus_type = 3`.

### What are Locational Marginal Prices (LMPs)?

**LMP** = The cost to serve one more MW of load at a specific bus.

```
LMP = Energy + Congestion + Losses
```

When transmission is congested, cheap generators can't reach all loads. Buses behind congestion have higher LMPs.

### What does "LOLE = 2.4 hours/year" actually mean?

**LOLE** (Loss of Load Expectation): On average, there will be 2.4 hours per year when available capacity is less than demand.

**Important**: This is a probabilistic expectation, not a guarantee of actual blackouts. Operators take emergency actions before load is actually shed.

**Planning standard**: LOLE ≤ 0.1 days/year (2.4 hours/year) is common in North America.

---

### Can I use GAT in my startup?

**Usually yes, at no cost.** If your startup:
- Uses GAT internally for your own analysis — ✅ **Free**
- Builds analysis tools for customers — ✅ **Free** (internal business use)
- Sells energy optimization as a service — ❌ **Needs commercial license**
- Resells GAT functionality directly — ❌ **Needs commercial license**

Unclear? [Start a discussion](https://github.com/monistowl/gat/discussions) — we're friendly about this.

### Is the source code available?

**Yes.** The full source is on [GitHub](https://github.com/monistowl/gat) under PolyForm Shield. You can:
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

GAT provides a **native solver plugin system** with automatic fallback:

**Pure-Rust (always available):**
- **L-BFGS** — Default for AC-OPF, always works
- **Clarabel** — Default for SOCP/LP, stable and reliable

**Native (optional, higher performance):**
- **IPOPT** — Fastest for large NLP/AC-OPF, requires installation
- **HiGHS** — Fast LP/MIP, modern and efficient
- **CBC** — Robust MIP support

**Recommendation:** Start with defaults. For large networks (1000+ buses), install IPOPT:
```bash
cargo xtask solver build ipopt --install
gat solver list  # Verify installation
```

### How do I install native solvers?

Native solvers provide better performance but require system dependencies:

```bash
# Install IPOPT (requires coinor-libipopt-dev on Ubuntu)
sudo apt install coinor-libipopt-dev  # Ubuntu/Debian
brew install ipopt                     # macOS

# Build and install GAT's IPOPT wrapper
cargo xtask solver build ipopt --install

# Verify installation
gat solver list
```

GAT automatically uses native solvers when available and falls back to pure-Rust solvers otherwise.

### Do I need native solvers?

**No.** GAT includes pure-Rust solvers (L-BFGS, Clarabel) that work everywhere:
- ✅ No system dependencies
- ✅ Cross-platform
- ✅ Good performance for most cases

Native solvers are optional enhancements for:
- Very large networks (1000+ buses)
- Production deployments requiring maximum speed
- MIP problems (unit commitment, TEP)

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

**Can't find your format?** [Open an issue](https://github.com/monistowl/gat/issues) with a sample file.

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

### Why doesn't my power flow converge?

Common causes and fixes:

| Symptom | Likely Cause | Fix |
|---------|--------------|-----|
| Diverges immediately | Bad initial guess | Use flat start (V=1.0, θ=0) |
| Oscillates forever | Ill-conditioned network | Check for very high/low impedances |
| "No solution" | Infeasible operating point | Reduce load or add generation |

**Debugging checklist**:
1. ✓ Is total generation ≥ total load?
2. ✓ Are all buses connected (no islands)?
3. ✓ Are impedances reasonable (not 0 or extremely high)?
4. ✓ Is there a slack bus defined?

### Which OPF method should I use?

| Method | Command | When to Use |
|--------|---------|-------------|
| DC-OPF | `gat opf dc` | Planning, screening, fast LMP estimation |
| Fast AC | `gat opf ac` | Quick linear approximation |
| AC-NLP | `gat opf ac-nlp` | Full nonlinear, highest accuracy |
| SOCP | benchmark `--method socp` | Research, convex relaxation |

**Decision tree**: Need speed? → DC-OPF. Need accuracy? → AC-NLP. Need global optimum guarantee? → SOCP.

### How do I run batch analysis?

Use GAT's **manifest system** for batch automation:

```toml
# manifest.toml
[runs]
base_case = { network = "grid.arrow", name = "Base Case" }
contingency_n1 = { network = "grid.arrow", contingencies = true }

[analysis]
analysis_type = "power_flow"
method = "ac"
```

Then run:
```bash
# First import your grid to Arrow format
gat import matpower --m grid.m -o grid.arrow

# Describe and run the manifest
gat runs describe manifest.toml
gat runs resume manifest.toml
```

See [Manifests](/internals/cli-architecture/#manifest-driven-workflows) for more.

### Can I parallelize analysis across multiple machines?

Not out-of-the-box, but it's easy to script:

```bash
# Import grid first, then run contingencies in parallel
gat import matpower --m grid.m -o grid.arrow
for i in {1..100}; do
  gat pf ac grid.arrow --contingency $i --out contingency_$i.parquet &
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
  https://github.com/monistowl/gat/releases/download/v0.5.0/install-modular.sh \
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

- **Discussions:** [Ask anything](https://github.com/monistowl/gat/discussions) — best for questions
- **Issues:** [Report bugs](https://github.com/monistowl/gat/issues) — for bugs and feature requests
- **Documentation:** [Full docs](/docs/) — for detailed guides

### How do I report a bug?

[Open an issue](https://github.com/monistowl/gat/issues) with:
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
5. **Check the code** — [Review the solver implementation](https://github.com/monistowl/gat/blob/main/crates/gat-algo/src/solver)

[Open an issue](https://github.com/monistowl/gat/issues) with your analysis and we'll investigate.

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
- **Discussions:** [GitHub Discussions](https://github.com/monistowl/gat/discussions)
- **Issues:** [GitHub Issues](https://github.com/monistowl/gat/issues)
- **Contributors:** See [GitHub Contributors](https://github.com/monistowl/gat/graphs/contributors)

### How often is GAT updated?

Currently:
- **Release cycle:** ~Monthly
- **Security patches:** As needed
- **Major features:** Every 3-6 months

Follow [releases](https://github.com/monistowl/gat/releases) for updates.

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

**Didn't find your question?** [Start a discussion](https://github.com/monistowl/gat/discussions) or [open an issue](https://github.com/monistowl/gat/issues)!
