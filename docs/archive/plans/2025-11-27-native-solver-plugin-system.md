# Native Solver Plugin System for gat

**Date:** 2025-11-27
**Status:** Draft (not for immediate execution)
**Author:** Brainstorming session

---

## Overview & Goals

A plugin architecture enabling gat to use high-performance C/C++ optimization solvers (IPOPT, CBC, HiGHS, Bonmin, Couenne, SuiteSparse) while:

1. **Preserving safety defaults** — Pure-Rust solvers (Clarabel, L-BFGS) work out-of-box with zero native code
2. **Explicit opt-in** — Each native solver requires per-solver acceptance with safety warning
3. **Easy distribution** — Pre-built binaries fetched from GitHub releases via `gat install`
4. **Fallback to source** — Users can build from vendored source if binaries unavailable
5. **Process isolation** — Native solvers run as subprocesses, crashes don't kill gat
6. **Configurable behavior** — User controls solver preferences, fallback behavior, global kill switch

### Non-goals (for this phase)

- GPU acceleration (CUDA solvers)
- Cloud/distributed solving
- Julia/Python solver wrappers (future extension)

### New Capabilities Unlocked

| Solver | Capability |
|--------|------------|
| IPOPT | Production-grade interior-point NLP |
| Bonmin | Mixed-integer nonlinear programming (MINLP) |
| Couenne | Global optimization with certificates |
| SuiteSparse | Fast sparse factorization for Newton methods |
| SYMPHONY/BCP | Parallel branch-and-cut for large MIP |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              gat (main binary)                              │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐                    │
│  │  gat-algo     │  │  gat-cli      │  │  gat-io       │                    │
│  │  (solvers)    │  │  (commands)   │  │  (Arrow I/O)  │                    │
│  └───────┬───────┘  └───────┬───────┘  └───────────────┘                    │
│          │                  │                                               │
│          ▼                  ▼                                               │
│  ┌───────────────────────────────────────┐  ┌────────────────────────────┐  │
│  │  gat-solver-dispatch                  │  │  gat install/list/update   │  │
│  │  - Solver selection logic             │  │  - Fetch from GitHub       │  │
│  │  - Config-based defaults              │  │  - Verify checksums        │  │
│  │  - Compute bounds checking            │  │  - First-run prompts       │  │
│  │  - Fallback handling                  │  │  - Manage ~/.gat/          │  │
│  └───────────────┬───────────────────────┘  └────────────────────────────┘  │
│                  │                                                          │
└──────────────────┼──────────────────────────────────────────────────────────┘
                   │
                   │ Arrow IPC over stdin/stdout
                   ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                        ~/.gat/bin/ (solver binaries)                         │
├────────────────┬────────────────┬────────────────┬────────────────┬──────────┤
│   gat-ipopt    │   gat-cbc      │   gat-highs    │   gat-bonmin   │   ...    │
│   (Rust+FFI)   │   (Rust+FFI)   │   (Rust+FFI)   │   (Rust+FFI)   │          │
├────────────────┼────────────────┼────────────────┼────────────────┼──────────┤
│   libipopt     │   libcbc       │   libhighs     │   libbonmin    │          │
│   (C/C++)      │   (C++)        │   (C++)        │   (C++)        │          │
└────────────────┴────────────────┴────────────────┴────────────────┴──────────┘
```

### Key Boundaries

1. **Safety boundary** — Process isolation between gat and native solvers
2. **Data boundary** — Arrow IPC schema defines the contract
3. **Version boundary** — Lockstep versioning (gat 0.5.0 ↔ gat-ipopt 0.5.0)

### Pure-Rust Path (always available)

```
gat-algo → Clarabel (conic) or argmin/L-BFGS (NLP)
```

### Native Path (opt-in)

```
gat-algo → gat-solver-dispatch → subprocess → gat-ipopt → IPOPT
```

---

## Directory Structure & Configuration

### ~/.gat/ Directory Layout

```
~/.gat/
├── config.toml              # User preferences
├── solvers.toml             # Solver state (managed by gat)
├── bin/                     # Solver binaries
│   ├── gat-ipopt
│   ├── gat-cbc
│   └── ...
├── lib/                     # Shared libraries (platform-dependent)
│   ├── libipopt.so.3        # Linux
│   └── libipopt.3.dylib     # macOS
├── cache/                   # Downloads & build artifacts
│   ├── downloads/
│   │   └── gat-ipopt-0.5.0-x86_64-linux-gnu.tar.gz
│   └── build/               # For --from-source builds
└── logs/                    # Debug logs (opt-in)
    └── gat-ipopt-2024-01-15.log
```

### ~/.gat/config.toml

```toml
[solver]
# Global kill switch - disable all native solvers temporarily
native_enabled = true

# Fallback behavior: "error", "fallback", "prompt"
on_failure = "error"
fallback_solver = "clarabel"

# Per-problem-type defaults
[solver.defaults]
ac_opf = "ipopt"       # NLP → IPOPT if installed, else L-BFGS
dc_opf = "highs"       # LP → HiGHS if installed, else Clarabel
socp = "clarabel"      # Conic → Clarabel (pure Rust, always available)
mip = "cbc"            # MIP → CBC if installed, else error
minlp = "bonmin"       # MINLP → Bonmin if installed, else error

[solver.compute_limits]
# Warn before launching solves estimated to exceed these
max_estimated_minutes = 60
max_memory_gb = 16
```

### ~/.gat/solvers.toml (managed by gat)

```toml
[ipopt]
version = "0.5.0"
installed_at = "2024-01-15T10:30:00Z"
binary_path = "~/.gat/bin/gat-ipopt"
accepted_risk = true           # User accepted safety warning
accepted_at = "2024-01-15T10:30:00Z"

[cbc]
version = "0.5.0"
installed_at = "2024-01-15T10:32:00Z"
binary_path = "~/.gat/bin/gat-cbc"
accepted_risk = true
accepted_at = "2024-01-15T10:32:00Z"
```

---

## CLI Commands

### Solver Management

```bash
# Install a solver (fetches pre-built binary)
gat install ipopt
gat install ipopt cbc highs          # Multiple at once
gat install ipopt --accept-native-risk   # Skip prompt (for CI)
gat install ipopt --from-source      # Build from vendor/ instead of fetching
gat install --all                    # Install all available solvers

# Remove a solver
gat uninstall ipopt
gat uninstall --all

# Update solvers to match current gat version
gat update ipopt
gat update --all

# List solver status
gat list                             # Installed + available
gat list --installed                 # Only installed
gat list --available                 # Only not-yet-installed
gat list --json                      # Machine-readable output
```

### Example `gat list` Output

```
$ gat list

Native Solvers:
  INSTALLED
    ipopt     0.5.0   ✓ ready     NLP interior-point
    cbc       0.5.0   ✓ ready     MIP branch-and-cut

  AVAILABLE
    highs     0.5.0               LP/MIP (fetch with: gat install highs)
    bonmin    0.5.0               MINLP branch-and-bound
    couenne   0.5.0               Global optimization
    symphony  0.5.0               Parallel MIP

Built-in (pure Rust):
    clarabel  0.11.1  ✓ always    Conic (SOCP, SDP)
    lbfgs     -       ✓ always    NLP penalty method

Native solvers: enabled (set native_enabled=false in config to disable)
```

### First-Run Install Experience

```
$ gat install ipopt

┌─────────────────────────────────────────────────────────────┐
│  ⚠️  Native Solver Warning                                  │
├─────────────────────────────────────────────────────────────┤
│  ipopt is a C/C++ library without Rust's memory safety     │
│  guarantees. Bugs in native code could cause:              │
│    • Crashes (segfaults)                                   │
│    • Undefined behavior                                    │
│    • Security vulnerabilities                              │
│                                                            │
│  gat runs native solvers in isolated subprocesses to       │
│  limit impact, but cannot guarantee safety.                │
│                                                            │
│  Learn more: https://gat.dev/docs/native-solvers           │
└─────────────────────────────────────────────────────────────┘

Install ipopt? [y/N]: y

📦 Fetching gat-ipopt v0.5.0 for x86_64-unknown-linux-gnu...
   Source: https://github.com/gat-project/gat/releases/download/v0.5.0/
✓ Downloaded gat-ipopt-0.5.0-x86_64-linux-gnu.tar.gz (4.2 MB)
✓ Verified checksum (SHA256: a1b2c3...)
✓ Extracted to ~/.gat/bin/gat-ipopt
✓ Registered in ~/.gat/solvers.toml

ipopt is now available. Test with: gat solve --solver ipopt <network>
```

---

## Arrow IPC Protocol

### Communication Flow

```
┌─────────┐    spawn     ┌─────────────┐
│   gat   │─────────────▶│  gat-ipopt  │
│         │              │             │
│         │──── stdin ──▶│             │  Arrow IPC: Problem
│         │              │             │
│         │◀── stdout ───│             │  Arrow IPC: Solution
│         │              │             │
│         │◀── stderr ───│             │  Text: Logs/errors
│         │              │             │
│         │◀── exit ─────│             │  Exit code: 0=success
└─────────┘              └─────────────┘
```

### Problem Schema (gat → solver)

```
// Arrow IPC Record Batch
message ProblemBatch {
    // Metadata (as Arrow schema metadata)
    problem_type: string      // "ac_opf", "dc_opf", "mip", "minlp"
    protocol_version: int32   // 1 (for future compatibility)
    base_mva: float64
    tolerance: float64
    max_iterations: int32

    // Bus data (Arrow arrays)
    bus_id: int64[]
    bus_name: utf8[]
    bus_v_min: float64[]
    bus_v_max: float64[]
    bus_p_load: float64[]     // MW
    bus_q_load: float64[]     // MVAr

    // Generator data
    gen_id: int64[]
    gen_bus_id: int64[]
    gen_p_min: float64[]
    gen_p_max: float64[]
    gen_q_min: float64[]
    gen_q_max: float64[]
    gen_cost_c0: float64[]    // $/hr
    gen_cost_c1: float64[]    // $/MWh
    gen_cost_c2: float64[]    // $/MW²h

    // Branch data
    branch_from: int64[]
    branch_to: int64[]
    branch_r: float64[]       // p.u.
    branch_x: float64[]       // p.u.
    branch_b: float64[]       // p.u. (charging)
    branch_rate: float64[]    // MVA
    branch_tap: float64[]
    branch_shift: float64[]   // radians
}
```

### Solution Schema (solver → gat)

```
message SolutionBatch {
    // Metadata
    status: string            // "optimal", "infeasible", "timeout", "error"
    objective: float64        // Total cost ($/hr)
    iterations: int32
    solve_time_ms: int64
    error_message: utf8       // If status == "error"

    // Bus results
    bus_id: int64[]
    bus_v_mag: float64[]      // p.u.
    bus_v_ang: float64[]      // degrees
    bus_lmp: float64[]        // $/MWh

    // Generator results
    gen_id: int64[]
    gen_p: float64[]          // MW
    gen_q: float64[]          // MVAr

    // Branch results (optional)
    branch_id: int64[]
    branch_p_from: float64[]  // MW
    branch_q_from: float64[]  // MVAr
    branch_p_to: float64[]
    branch_q_to: float64[]
}
```

### Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success (check status in solution for optimality) |
| 1 | Invalid input (malformed Arrow, missing fields) |
| 2 | Solver error (license, numerical issues) |
| 3 | Timeout |
| 139 | Segfault (SIGSEGV) — native crash |

---

## Build Infrastructure

### Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            cargo xtask build-solvers                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   Local Development                      CI / GitHub Releases               │
│   ──────────────────                     ───────────────────                │
│                                                                             │
│   cargo xtask build-solvers ipopt        nix build .#gat-ipopt             │
│            │                                      │                         │
│            ▼                                      ▼                         │
│   ┌─────────────────┐                   ┌─────────────────┐                 │
│   │    coinbrew     │                   │   Nix flake     │                 │
│   │  (vendor/*.zip) │                   │  (hermetic)     │                 │
│   └────────┬────────┘                   └────────┬────────┘                 │
│            │                                      │                         │
│            ▼                                      ▼                         │
│   ~/.gat/bin/gat-ipopt                  gat-ipopt-0.5.0-x86_64-linux.tar.gz│
│   (local install)                       (upload to GitHub releases)         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### xtask Commands

```bash
# Build a specific solver from vendor/ source
cargo xtask build-solvers ipopt
cargo xtask build-solvers ipopt cbc highs    # Multiple

# Build all solvers
cargo xtask build-solvers --all

# Build for specific target (cross-compile)
cargo xtask build-solvers ipopt --target aarch64-apple-darwin

# Clean build artifacts
cargo xtask clean-solvers

# Package for release (creates tarball)
cargo xtask package-solvers ipopt --version 0.5.0
```

### Coinbrew Integration (local builds)

```rust
// xtask/src/build_solvers.rs

fn build_ipopt(target: &str) -> Result<PathBuf> {
    let vendor_dir = project_root().join("vendor");
    let build_dir = home_dir().join(".gat/cache/build/ipopt");

    // Extract vendored source
    extract_zip(&vendor_dir.join("Ipopt-stable-3.14.zip"), &build_dir)?;

    // Run coinbrew (handles ThirdParty deps like Mumps, ASL)
    let status = Command::new("bash")
        .current_dir(&build_dir)
        .arg("coinbrew")
        .arg("build")
        .arg("Ipopt")
        .arg("--prefix").arg(home_dir().join(".gat"))
        .arg("--tests=none")  // Skip tests for faster build
        .status()?;

    ensure!(status.success(), "coinbrew failed");

    // Build the Rust wrapper binary
    cargo_build("gat-ipopt", target)?;

    Ok(home_dir().join(".gat/bin/gat-ipopt"))
}
```

### Platform Targets

| Target | Use Case |
|--------|----------|
| `x86_64-unknown-linux-gnu` | Servers, CI, most Linux desktops |
| `x86_64-unknown-linux-musl` | Alpine containers, static binaries |
| `aarch64-unknown-linux-gnu` | ARM servers (Graviton, Ampere) |
| `x86_64-apple-darwin` | Intel Macs |
| `aarch64-apple-darwin` | Apple Silicon (M1/M2/M3) |
| `x86_64-pc-windows-msvc` | Windows desktops |

---

## Solver Dispatch Logic

### Selection Flow

```rust
// gat-algo/src/solver_dispatch.rs

pub fn select_solver(
    problem_type: ProblemType,
    config: &GatConfig,
    requested: Option<SolverId>,
) -> Result<SolverChoice, SolverError> {

    // 1. Check global kill switch
    if !config.solver.native_enabled {
        return select_pure_rust_solver(problem_type);
    }

    // 2. User explicitly requested a solver
    if let Some(solver) = requested {
        return validate_and_select(solver, problem_type, config);
    }

    // 3. Check user's configured default for this problem type
    if let Some(solver) = config.solver.defaults.get(problem_type) {
        if is_installed(solver) {
            return Ok(SolverChoice::Native(*solver));
        }
        // Configured solver not installed - warn and fall through
        warn!("Configured solver {} not installed, using fallback", solver);
    }

    // 4. Problem-matched defaults (built-in hierarchy)
    match problem_type {
        ProblemType::AcOpf => {
            if is_installed(SolverId::Ipopt) {
                Ok(SolverChoice::Native(SolverId::Ipopt))
            } else {
                Ok(SolverChoice::PureRust(PureRustSolver::Lbfgs))
            }
        }
        ProblemType::DcOpf | ProblemType::Lp => {
            if is_installed(SolverId::Highs) {
                Ok(SolverChoice::Native(SolverId::Highs))
            } else {
                Ok(SolverChoice::PureRust(PureRustSolver::Clarabel))
            }
        }
        ProblemType::Socp => {
            Ok(SolverChoice::PureRust(PureRustSolver::Clarabel)) // Always Clarabel
        }
        ProblemType::Mip => {
            if is_installed(SolverId::Cbc) {
                Ok(SolverChoice::Native(SolverId::Cbc))
            } else if is_installed(SolverId::Highs) {
                Ok(SolverChoice::Native(SolverId::Highs))
            } else {
                Err(SolverError::NoSolverAvailable {
                    problem_type,
                    hint: "Install a MIP solver: gat install cbc",
                })
            }
        }
        ProblemType::Minlp => {
            if is_installed(SolverId::Bonmin) {
                Ok(SolverChoice::Native(SolverId::Bonmin))
            } else {
                Err(SolverError::NoSolverAvailable {
                    problem_type,
                    hint: "Install MINLP solver: gat install bonmin",
                })
            }
        }
    }
}
```

### Compute Bounds Checking

```rust
pub fn check_compute_bounds(
    problem: &Problem,
    solver: SolverId,
    config: &GatConfig,
) -> Result<(), ComputeWarning> {
    let estimate = estimate_solve_time(problem, solver);

    if estimate.minutes > config.solver.compute_limits.max_estimated_minutes {
        return Err(ComputeWarning::TimeExceeded {
            estimated: estimate,
            limit: config.solver.compute_limits.max_estimated_minutes,
            suggestion: format!(
                "This solve may take ~{} hours. Consider:\n\
                 • Using a faster solver (--solver highs)\n\
                 • Reducing problem size\n\
                 • Running on a more powerful machine\n\
                 • Override with --force",
                estimate.minutes / 60
            ),
        });
    }

    Ok(())
}
```

### Fallback Handling

```rust
pub async fn solve_with_fallback(
    problem: &Problem,
    primary: SolverChoice,
    config: &GatConfig,
) -> Result<Solution, SolverError> {

    let result = execute_solver(problem, &primary).await;

    match result {
        Ok(solution) => Ok(solution),
        Err(e) => {
            match config.solver.on_failure {
                OnFailure::Error => Err(e),

                OnFailure::Fallback => {
                    warn!("{} failed: {}. Retrying with {}...",
                          primary, e, config.solver.fallback_solver);
                    let fallback = SolverChoice::PureRust(config.solver.fallback_solver);
                    execute_solver(problem, &fallback).await
                }

                OnFailure::Prompt => {
                    eprintln!("{} failed: {}", primary, e);
                    eprint!("Retry with {}? [Y/n]: ", config.solver.fallback_solver);
                    if prompt_yes_no()? {
                        let fallback = SolverChoice::PureRust(config.solver.fallback_solver);
                        execute_solver(problem, &fallback).await
                    } else {
                        Err(e)
                    }
                }
            }
        }
    }
}
```

---

## Implementation Phases

### Phase 1: Foundation with IPOPT (pilot)

Build the complete pipeline end-to-end with IPOPT as proof of concept:

**Deliverables:**

- `gat-solver-common` crate
  - Arrow IPC schema (problem + solution)
  - Subprocess spawning & communication
  - Error types & exit code handling
  - Timeout & graceful termination

- `gat-ipopt` wrapper binary
  - Rust wrapper using ipopt-sys
  - Arrow IPC parsing
  - Solution serialization
  - Logging to stderr

- `gat-algo` integration
  - SolverDispatch trait
  - Subprocess execution path
  - Fallback logic
  - Compute bounds estimation

- CLI commands
  - `gat install ipopt`
  - `gat uninstall ipopt`
  - `gat list`
  - `gat update ipopt`

- Config system
  - `~/.gat/` directory structure
  - `config.toml` parsing
  - `solvers.toml` state management
  - First-run prompt & acceptance tracking

- Build infrastructure
  - `cargo xtask build-solvers ipopt`
  - coinbrew integration
  - Basic Nix flake for Linux x86_64

- GitHub release pipeline
  - Actions workflow for ipopt
  - Checksum generation
  - Release asset upload

### Phase 2: Complete Solver Coverage

Replicate the pattern for remaining solvers:

- `gat-cbc` (MIP)
- `gat-highs` (LP/MIP)
- `gat-bonmin` (MINLP) — requires new -sys crate
- `gat-couenne` (Global) — requires new -sys crate
- `gat-symphony` (Parallel MIP) — requires new -sys crate

### Phase 3: SuiteSparse Integration

Different pattern — library, not standalone solver:

- `suitesparse-sys` crate
- Integration into gat-algo Newton solvers
- Feature flag: `solver-suitesparse`
- Optional linking in gat-ipopt for faster factorization

### Phase 4: Platform Matrix Completion

Expand CI to all targets:

- `x86_64-unknown-linux-gnu` (Phase 1)
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

### Phase 5: Polish & Documentation

- User guide: "Using Native Solvers"
- Troubleshooting guide
- Benchmark comparisons
- `--from-source` build instructions
- Performance tuning guide

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Coinbrew fragility** | Build failures on some platforms | Nix flakes for CI releases; coinbrew only for local dev |
| **ABI breakage in -sys crates** | IPOPT update breaks gat-ipopt | Lockstep versioning; pin upstream in vendor/ |
| **Arrow IPC schema drift** | Old solver binaries incompatible | Protocol version field; clear error messages |
| **Cross-compilation complexity** | Can't build Windows from Linux | Use GitHub Actions matrix with native runners |
| **Large binary sizes** | Users avoid downloading | Compress with zstd; strip symbols; consider solver-specific downloads |
| **License compliance** | COIN-OR licenses vary (EPL, CPL) | Document license for each solver; include in release artifacts |
| **Subprocess overhead** | Performance regression vs in-process | Benchmark; consider daemon mode for batch solves |
| **User confusion** | "Which solver do I need?" | Clear `gat list` output; problem-matched defaults; good docs |

---

## Open Questions

To resolve during implementation:

1. **Daemon mode** — Worth implementing for batch workloads, or subprocess-per-solve is fast enough?
2. **Partial downloads** — Resume interrupted downloads, or re-fetch from scratch?
3. **Solver-specific options** — How to pass IPOPT-specific parameters (mu_init, etc.) through the IPC?
4. **Multi-solve batching** — Single subprocess call for N independent solves?

---

## Decision Summary

| Decision | Choice |
|----------|--------|
| Primary goal | Distribution + new features |
| Distribution model | Fetch from GitHub, fallback to source build |
| Install location | `~/.gat/` (self-contained) |
| Safety gating | Per-solver first-run prompt, `--accept-native-risk`, global toggle |
| Solver defaults | User-configurable, problem-matched out-of-box |
| Plugin architecture | Subprocess IPC with process isolation |
| IPC format | Arrow IPC |
| Wrapper binaries | Rust + FFI, one per solver |
| CLI UX | Cargo-style (`gat install`, `gat list`, etc.) |
| Versioning | Lockstep with gat version |
| Fallback behavior | Configurable (default: hard error) |
| Build infrastructure | xtask + coinbrew (local), Nix (CI) |
| Platform targets | Linux (glibc+musl), macOS (Intel+ARM), Windows |
| Implementation order | Breadth-first with IPOPT as pilot |

---

## Crate Structure

```
gat/
├── crates/
│   ├── gat-algo/              # Main algorithms (unchanged)
│   ├── gat-solver-common/     # Shared: Arrow IPC, problem schema, error types
│   │   └── src/lib.rs
│   └── solvers/
│       ├── gat-ipopt/         # Rust binary wrapping ipopt-sys
│       │   ├── Cargo.toml     # depends on ipopt-sys, gat-solver-common
│       │   └── src/main.rs
│       ├── gat-cbc/           # Rust binary wrapping coin_cbc_sys
│       ├── gat-highs/         # Rust binary wrapping highs-sys
│       ├── gat-bonmin/        # New -sys crate + wrapper
│       ├── gat-couenne/       # New -sys crate + wrapper
│       └── gat-suitesparse/   # SuiteSparse for sparse LA
└── vendor/                    # Source zips (for local builds)
```
