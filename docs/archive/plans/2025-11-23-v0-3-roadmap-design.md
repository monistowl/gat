# v0.3 Roadmap Design

**Date:** 2025-11-23
**Status:** Design Complete
**Focus:** Feature Completeness & API Stability
**Approach:** Depth-first feature completion with lightweight dependency audit

---

## Executive Summary

v0.3 prioritizes **hardening existing features and achieving feature completeness** over expanding breadth. All advertised commands must be production-ready; all stubs and placeholders must ship as complete implementations. This is the strategic moment for breaking API changes before the user base grows.

**Core principle:** Make any necessary breaking changes now, lock the API surface, and ensure every `gat` command shipped is reliable enough for scripts and automation.

---

## Overall Strategy

### Depth-First Feature Completion

Rather than parallel feature work, we complete each major area fully before moving to the next. This ensures no half-finished features ship and allows early features to inform later ones.

**Sequencing:**
1. **CIM RDF Ingestion + Data API Fetchers** — Build better data sources (EIA, Ember) to enable robust testing downstream
2. **AC OPF Implementation** — Foundation for distribution workflows (DIST/DERMS/ADMS)
3. **Reliability Metrics Framework** — Complete LOLE/EUE/CANOS multi-area + all downstream workflows (FLISR, VVO, outage coordination)

### API Stability & Breakage Budget

v0.3 is the last release where we make breaking changes freely:
- Audit all public module exports in `gat-core`, `gat-io`, `gat-algo`, `gat-ts`, `gat-dist`, `gat-derms`, `gat-adms`
- Remove unused exports
- Rename commands/flags for consistency if needed
- Add clear deprecation markers for anything that will change in v0.4
- Lock down CLI interface — no changes to command names, flags, or output formats without major version bump

**Success criteria:** Every `gat <command>` that ships in v0.3 is fully implemented, error-handled, documented, and part of the stable API.

### Lightweight Dependency Audit (Parallel)

Concurrent with feature work, catalog all external dependencies to identify easy wins for post-v0.3 cleanup:
- Flag unmaintained packages (last update 12+ months ago)
- Identify single-function crates under 500 LOC (inlining candidates)
- Map solver lock-in points (understand `good_lp` constraints)
- Assess TUI framework stability (`tuirealm`)

**Deliverable:** Single markdown document with risk matrix and v0.3 quick-win candidates.

**Scope:** Audit only. No major refactoring in v0.3. Enables post-v0.3 deep cleanup.

---

## Feature 1: CIM RDF Ingestion & Data API Fetchers

### Current State
- `gat-io` has minimal CIM support (topology + metadata only)
- No real-world data fetchers (everything is pre-baked test fixtures)
- Limited ability to test against diverse grid models

### Goals
1. Complete ENTSO-E CIM RDF model ingestion (full constraints, limits, operational parameters)
2. Add lightweight API downloaders for EIA.gov and Ember Climate data
3. Output modern data formats (Arrow/Parquet) for seamless integration with `gat-ts` workflows
4. Establish high-confidence test fixtures from real-world data

### Architecture

#### CIM RDF Completion
- Extend `gat-io::cim` module with full ENTSO-E CIM 16+ model mapping:
  - **EquipmentContainer**: Substations, voltage levels, bays
  - **ConductingEquipment**: Generators, loads, transformers, lines with full constraints
  - **OperationalLimits**: Thermal, voltage, frequency limits per equipment
  - **Measurements**: Real-time telemetry points (for time-series integration)
- High-level validation layer:
  - Warn on missing required fields (e.g., equipment without limits)
  - Catch malformed RDF structure early
  - Detailed error messages for debugging
- Round-trip testing: CIM → gat internal types → Arrow → CIM (lossless where possible)

#### Data API Fetchers
Two lightweight downloaders, minimal dependencies (prefer `ureq` over heavy HTTP stacks):

**EIA.gov Fetcher** (`gat-io::sources::eia`):
- Query EIA's public API (no authentication needed for basic data)
- Retrieve: Grid topology (transmission, sub-transmission), generator fleet (capacity, fuel type, location), load/demand data (hourly, by state/region)
- Output: Arrow tables with bus/branch/generator/load records
- Integration: Directly feed into `gat-core` types for simulation

**Ember Climate Fetcher** (`gat-io::sources::ember`):
- Query Ember's public dataset (carbon intensity, renewable energy mix, real-time grid status)
- Retrieve: Hourly carbon intensity (global + regional), renewable generation %, grid frequency
- Output: Time-series Arrow format compatible with `gat-ts` workflows
- Use case: Temporal scenario generation (hourly demand + carbon intensity forecasts)

**Design principles:**
- Minimal external HTTP library (use `ureq` if not already in closure, else `reqwest` with minimal features)
- Mock API responses in CI tests (never fetch live data during tests)
- Graceful degradation: If API is unavailable, provide clear error; allow offline mode with pre-cached snapshots
- Deterministic output: Same query always returns same data structure (for test reproducibility)

### Implementation Phases

1. **Phase 1: CIM RDF Full Model** (1-2 weeks)
   - Extend RDF parser to extract constraints, limits, measurements
   - Add validation layer with structured error types
   - Write round-trip tests with 3-5 public CIM fixtures

2. **Phase 2: EIA Data Fetcher** (1 week)
   - Implement EIA API client wrapper
   - Map EIA records to `gat` types
   - Write integration tests with mocked API responses
   - Pre-cache sample datasets for CI

3. **Phase 3: Ember Data Fetcher** (1 week)
   - Implement Ember API client
   - Output time-series Arrow format
   - Integration tests with `gat-ts` workflows

### Testing & Validation

- **CIM validation:** Round-trip consistency with public ENTSO-E RDF samples
- **API fetchers:** Mock API responses in tests; pre-cache real data snapshots for offline testing
- **Integration:** Can ingest EIA topology + Ember carbon data, produce valid simulation scenarios
- **Benchmarks:** Ingestion time for large RDF files (10k+ bus systems)

### Success Criteria
- ✅ All `gat import cim` tests pass with full RDF model
- ✅ `gat dataset eia` and `gat dataset ember` commands fetch live data and produce valid outputs
- ✅ Integration test: Ingest EIA grid + Ember time-series, run DC OPF successfully
- ✅ Zero external API calls during CI tests (mocked)

---

## Feature 2: AC OPF Implementation

### Current State
- `gat-algo` has AC OPF placeholder (returns error or unimplemented)
- AC OPF is critical for distribution automation (DIST, DERMS, ADMS workflows)
- Users currently downgrade to DC OPF with accuracy loss

### Goals
1. Implement working AC OPF for transmission and distribution grids
2. Support multiple solver backends (Clarabel primary, Ipopt optional)
3. Production-ready error handling (infeasibility, numerical issues, timeouts)
4. Validated against industry benchmarks

### Architecture

#### Formulation: Penalty Method

Convert AC OPF to smooth, convex-friendly problem via penalty method:

```
minimize: Σ c_g * P_g + λ_p * Σ (V - V_nom)² + λ_p * Σ (Q_g - Q_nom)²
subject to:
  - Power balance equations (real + reactive)
  - Generator output limits
  - Transmission/transformer thermal limits
  - Voltage magnitude bounds [V_min, V_max]
  - Reactive power bounds [Q_min, Q_max]
  - Transmission losses (approximated in objective)
```

**Why penalty method?** Smooth relaxation avoids discrete solver complexity; empirically converges fast for distribution networks; easy fallback to DC if AC times out.

#### Solver Integration (Pluggable)

1. **Primary:** Clarabel (quadratic cone solver via `good_lp`)
   - Fast, reliable for most grids
   - Default in all feature combinations
2. **Optional:** Ipopt (general nonlinear via external solver)
   - Higher fidelity for challenging cases
   - Requires system binary (like HiGHS)
3. **Fallback:** DC OPF with user warning if AC times out or infeasible

#### Constraints Modeling

- **Voltage:** min/max per bus (typically 0.95–1.05 pu)
- **Thermal:** Line/transformer MVA limits (both directions)
- **Generation:** Real/reactive power bounds per generator
- **Demand:** Fixed load satisfaction (no demand shedding in base version)
- **Distribution-specific:** Radial topology checks, inverter reactive capability (DER)

#### Error Handling & Diagnostics

Structured error types for robust fallback:

```rust
pub enum AcOpfError {
  Infeasible(String),           // Demand exceeds supply
  Unbounded,                    // Pathological formulation
  SolverTimeout(Duration),      // Exceeded time limit
  NumericalIssue(String),       // Singular Jacobian, etc.
  DataValidation(String),       // Missing limits, bad topology
}
```

When AC OPF fails:
- Log detailed diagnostics (constraint violations, solver iterations)
- Fallback to DC OPF with warning to user
- Return structured error, not panic

### Implementation Phases

1. **Phase 1: Penalty Method + Clarabel** (2-3 weeks)
   - Implement AC power balance equations (Jacobian, polar formulation)
   - Penalty method formulation (voltage + reactive penalties)
   - Integrate with existing `good_lp` + Clarabel backend
   - Unit tests: convex relaxations, toy problems

2. **Phase 2: Constraint Handling & Validation** (1 week)
   - Add all constraint types (thermal, voltage, generation)
   - Validation layer: catch missing/invalid limits before solve
   - Structured error types and fallback logic

3. **Phase 3: Benchmark & Optimization** (1 week)
   - Validate against IEEE 30-bus, 118-bus, 300-bus benchmarks
   - Compare results to published OPF solutions (PSS/E, MATPOWER)
   - Optimize penalty weights and solver tolerances for speed/accuracy trade-off

4. **Phase 4: Optional Ipopt Backend** (1 week, lower priority)
   - Implement Ipopt callable interface (system binary required)
   - Feature gate: `solver-ipopt` (like HiGHS, COIN-CBC)

### Testing & Validation

**Golden benchmarks:**
- IEEE 30-bus: known OPF solution cost + flows
- IEEE 118-bus: larger transmission system
- MATPOWER test cases (IEEE 57, 300, 2383-bus)
- Real grids from newly ingested EIA data

**Test scenarios:**
- Uncongested case (one OPF solution)
- Congestion management (multiple solutions, check cost sensitivity)
- Infeasible case (demand > capacity, graceful degradation to DC)
- Numerical edge cases (very small generator, extreme cost ratios)
- Distribution radial networks (from newly ingested data)

**Validation approach:**
- Compare AC OPF cost to DC OPF cost (AC should be ≤ DC)
- Check all constraints satisfied to solver tolerance
- Measure solve time on representative grids (should be <5s for 1000-bus)

### Success Criteria
- ✅ `gat opf ac` works end-to-end on IEEE benchmarks with ±0.1% cost vs. published
- ✅ Graceful fallback to DC OPF on infeasibility with clear user message
- ✅ All constraint violations detected and reported before solve
- ✅ Solve time <5s for transmission grids, <1s for distribution (<500 bus)
- ✅ Comprehensive error handling (no panics, detailed diagnostics)

---

## Feature 3: Reliability Metrics & ADMS Workflows

### Current State
- ADMS has partial LOLE/EUE implementations (incomplete formulas, missing CANOS framework)
- VVO/FLISR/outage coordination exist but don't track reliability impact
- No validated benchmarks against industry standards (NERC, WECC)

### Goals
1. Complete LOLE/EUE/Deliverability Score implementations with Monte Carlo sampling
2. Implement CANOS multi-area reliability framework
3. Integrate FLISR/VVO/outage coordination with reliability impact tracking
4. Validate against published reliability metrics (NERC/WECC benchmarks)

### Architecture

#### Reliability Metrics: LOLE & EUE

**LOLE (Loss of Load Expectation):** Hours per year when demand exceeds available supply

Formula:
```
LOLE = Σ_scenarios P(scenario) * duration_hours(scenario)
```

where scenario = {generation outages, transmission outages, demand realization}

**EUE (Energy Unserved):** MWh per year of unmet demand

Formula:
```
EUE = Σ_scenarios P(scenario) * ∫ shortfall(t) dt
```

**Deliverability Score:** Composite metric (0–100) combining multiple failure modes:

```
score = 100 * [1 - w_lole * LOLE/LOLE_max
               - w_voltage * violations/max_violations
               - w_thermal * overloads/max_overloads]
```

**Implementation approach: Monte Carlo Sampling**

1. Generate N outage scenarios (2000–5000 samples typical):
   - Random subset of generators offline (Weibull failure rate per unit)
   - Random subset of transmission lines offline (failure + repair time)
   - Demand realization from historical/forecasted data

2. For each scenario:
   - Run AC OPF (or DC OPF if AC infeasible)
   - Track unmet demand, voltage violations, thermal violations
   - Store results in scenario matrix

3. Aggregate across scenarios:
   - LOLE = count(scenarios with shortfall) * hours per scenario / total hours per year
   - EUE = sum(unmet MWh) aggregated per year
   - Deliverability = composite with weights

**Validation:**
- Compare LOLE results to published metrics from NERC/WECC (order of magnitude check)
- Benchmark against PSS/E's LOLE module on industry test cases
- Sensitivity analysis: How much does LOLE change with 10% demand increase?

#### CANOS Multi-Area Framework

Extend beyond single-area reliability to interconnected systems:

**Multi-area model:**
- Each area has local generation, demand, controllable reserves
- Inter-area transmission corridors connect areas
- Outage in one area affects neighbor reliability via corridor congestion

**Features:**
1. **Zone-to-zone LOLE:** Reliability from area A's perspective given outages in neighboring areas
2. **Import/export limits:** How much can area A import/export before losing reliability?
3. **Coordinated outage scheduling:** Schedule maintenance across areas to minimize peak LOLE (not during high-demand periods)

**Implementation:**
- Extend scenario generation to sample multi-area outages
- Run multi-area AC OPF for each scenario
- Track LOLE by area + inter-area flows
- Visualization: LOLE heatmap by area and time-of-year

#### FLISR (Fault Location Isolation Service Restoration)

Automated switching sequences to restore service after faults:

**Current state:** Switching sequences exist but don't track reliability impact.

**v0.3 enhancement:**
- Each FLISR action (switch operation) recalculates post-restoration LOLE/EUE
- Compare: reliability before fault vs. after fault vs. after FLISR restoration
- Metrics: time-to-restore, load restored, peak restoration cost
- Validate: FLISR decisions improve reliability vs. baseline

#### VVO (Volt/VAR Optimization)

Real-time reactive power dispatch minimizing losses while maintaining reliability:

**Current state:** Exists but standalone from reliability framework.

**v0.3 enhancement:**
- Extend AC OPF objective: minimize real losses + maintain voltage margins (reliability)
- Trade-off: aggressive VVO reduces losses but tightens voltage margins
- Reliability-aware: Don't optimize VVO if it reduces Deliverability Score below threshold
- Integration: VVO results feed into LOLE/EUE calculations

#### Outage Coordination

Schedule planned maintenance across the system and multi-area zones:

**Goal:** Minimize peak LOLE/EUE impact during maintenance season.

**Approach:**
1. Generate candidate maintenance schedules (random or greedy)
2. For each schedule, simulate one year with staggered outages
3. Track cumulative EUE over year
4. Choose schedule minimizing EUE (or peak daily LOLE)

**Multi-area twist:** Coordinate across control areas — no two neighbors should be in maintenance during high-demand periods.

### Implementation Phases

1. **Phase 1: LOLE/EUE Monte Carlo** (2 weeks)
   - Implement scenario generation (outage sampling via Weibull)
   - OPF evaluation loop (run OPF per scenario, track shortfalls)
   - Aggregation: LOLE/EUE formulas
   - Unit tests: known LOLE benchmarks

2. **Phase 2: Deliverability Score & Validation** (1 week)
   - Implement composite scoring function
   - Benchmark against NERC/WECC published metrics
   - Sensitivity analysis (demand ±10%, supply ±10%)

3. **Phase 3: CANOS Multi-Area** (1.5 weeks)
   - Extend scenario generation for multi-area outages
   - Per-area LOLE calculation
   - Inter-area transmission impact tracking
   - Visualization: LOLE heatmap by area

4. **Phase 4: FLISR + VVO + Outage Coordination Integration** (2 weeks)
   - Integrate FLISR with reliability recalculation (post-restoration LOLE)
   - Extend VVO objective with reliability constraints
   - Outage scheduling optimizer (greedy or simulated annealing)
   - Integration tests: full workflows (fault → FLISR → reliability recovery)

### Testing & Validation

**Golden benchmarks:**
- NERC reliability metrics (LOLE ranges for different regions: 0.5–3 hrs/year typical)
- WECC publications (published LOLE by control area)
- PSS/E case library (compare LOLE calculations)

**Test scenarios:**
- Small radial distribution system (100 bus, known LOLE target)
- Transmission system (300+ bus, multiple areas)
- Seasonal demand variation (winter peak, summer low)
- Fault scenarios: single line outage, generator trip, cascade

**Validation criteria:**
- LOLE within ±10% of published benchmarks
- Sensitivity: LOLE doubles with 50% reduction in reserve margin (expected)
- FLISR: post-restoration LOLE <10% of pre-fault LOLE
- Outage coordination: chosen schedule reduces annual EUE by ≥5% vs. random

### Success Criteria
- ✅ `gat analytics reliability` command produces LOLE/EUE with ±10% accuracy vs. published
- ✅ Deliverability Score correlates with industry reliability definitions
- ✅ CANOS multi-area framework tracks zone-to-zone reliability impact
- ✅ FLISR, VVO, outage coordination integrate with reliability metrics
- ✅ Comprehensive test coverage with public benchmarks + multi-area scenarios

---

## Dependency Audit Strategy

### Scope
Catalog all external dependencies; identify easy wins; plan post-v0.3 cleanup.

**No refactoring in v0.3.** Audit only.

### Audit Checklist

For each external crate, document:
1. **Last update date** — Flag if >12 months old
2. **Maintenance status** — Active org? Maintained? Archived?
3. **Size & complexity** — LOC, number of dependencies, feature gates
4. **Risk level** — Critical path vs. optional? Breaking changes likely?
5. **Inlining feasibility** — Could we vendor this code easily?
6. **Replacement candidate** — If we removed it, what's the alternative?

### Easy Wins to Evaluate in v0.3

1. **iocraft (170 LOC)**
   - Terminal utilities (raw mode, keyboard input)
   - **Action:** Check if can be inlined into `gat-tui` directly or forked
   - **Blocker:** Might be actively maintained; assess first

2. **power_flow_data & caseformat** (PSS/E RAW, MATPOWER parsers)
   - **Action:** Check maintenance status; if unmaintained, consider vendoring or forking
   - **Risk:** If these break on new Rust version, we're stuck
   - **Alternative:** Rewrite critical parsers in-house (both are mature formats)

3. **good_lp (LP/MILP wrapper)**
   - **Action:** Map the lock-in: what prevents direct solver integration?
   - **Scope:** Not a v0.3 refactor; just understand constraints for post-v0.3 planning
   - **Question:** Does `good_lp` support recent Clarabel/HiGHS versions?

4. **tuirealm (TUI framework, 3.2)**
   - **Action:** Verify active maintenance; assess if rework to `ratatui` is necessary
   - **Concern:** If tuirealm stalls, is migration path clear?
   - **Scope:** Likely beyond v0.3 scope; just document status

### Deliverable

**docs/dependency-audit-v0.3.md:**
- Table of all external deps (name, version, last update, risk level)
- Risk/complexity matrix (which deps are safe? which are risky?)
- v0.3 quick-wins (iocraft, power_flow_data, caseformat assessment)
- Post-v0.3 candidates (good_lp deep dive, tuirealm migration planning)

### Timeline
- Complete audit during feature work (parallel effort)
- 1–2 review sessions with decision points
- Final document by v0.3 feature freeze

---

## API Stability & Breaking Changes

### Public API Audit (All Library Crates)

**gat-core:**
- Review all public exports (graph_utils, solver, ID wrappers)
- Document intended public surface (what's exported vs. private)
- Remove unused exports
- Add `#[doc(hidden)]` to internal types if needed

**gat-io:**
- Lock down file format adapters (PSS/E, MATPOWER, CIM)
- Ensure error types are consistent (use `anyhow` or structured enums?)
- Finalize schema for Arrow/Parquet output

**gat-algo:**
- Solver interface should be stable (no breaking changes to `Solver` trait)
- Error handling: use structured error types, not generic strings
- Feature-gating: document which solvers are available in each feature combo

**gat-ts, gat-dist, gat-derms, gat-adms:**
- Public entry points should be documented with examples
- Remove internal/experimental APIs from public exports

### CLI Stability

**Rules for v0.3 → v0.4:**
- ✅ Can add new subcommands
- ✅ Can add new flags (backward compatible)
- ❌ Cannot rename commands/flags
- ❌ Cannot change output format (unless under new `--format` flag)
- ❌ Cannot change argument order or meaning

**Enforcement:**
- All command signatures locked in v0.3 `Cargo.toml` version bump
- Integration tests validate command interface (names, flags, output structure)
- CHANGELOG explicitly marks any breaking changes (should be none by v0.3 release)

---

## Success Criteria & Validation

### Feature Completeness
- ✅ CIM RDF full model ingestion working with public fixtures
- ✅ EIA + Ember data fetchers functional and integrated
- ✅ AC OPF end-to-end with validated results on benchmarks
- ✅ LOLE/EUE/Deliverability with ±10% accuracy vs. published
- ✅ CANOS multi-area framework implemented and tested
- ✅ FLISR/VVO/outage coordination integrated with reliability metrics

### API Stability
- ✅ All public module exports audited and finalized
- ✅ No unused exports; clear public vs. private distinction
- ✅ Structured error types throughout (no generic strings)
- ✅ CLI command signatures locked and tested
- ✅ CHANGELOG documents all breaking changes (plan for none)

### Dependency Audit
- ✅ Complete audit document with risk matrix
- ✅ Easy wins (iocraft, parsers) assessed
- ✅ Roadmap for post-v0.3 cleanup (good_lp, tuirealm)

### Documentation & Testing
- ✅ All new features documented with examples
- ✅ Golden benchmarks for AC OPF, LOLE, multi-area
- ✅ Integration tests across data sources + workflows
- ✅ No external API calls in CI tests (mocked)
- ✅ Zero panics in error paths (all structured error handling)

---

## Timeline & Resource Allocation

**Suggested pacing (no hard deadline):**
- **Weeks 1–3:** CIM RDF + EIA/Ember data fetchers
- **Weeks 4–6:** AC OPF implementation + benchmarking
- **Weeks 7–10:** Reliability metrics + CANOS framework
- **Weeks 11–12:** API audit + dependency audit + docs
- **Week 13:** Buffer for test failures, edge cases, validation

**Parallel:**
- Dependency audit (1–2 sessions, 4–6 hours total)
- CLI/library API audit (2–3 sessions, 6–8 hours total)

**Estimated total effort:** 12–14 weeks solo; scales with team size.

---

## Next Steps

1. ✅ Brainstorm & design validation (complete)
2. → Create detailed implementation plan with exact file paths and task breakdown
3. → Set up git worktree for isolated v0.3 development
4. → Begin with CIM RDF ingestion (Phase 1)

Ready to move to implementation planning?
