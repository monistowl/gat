# Phase 3: Reliability Metrics & CANOS Framework - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete LOLE/EUE Monte Carlo framework, Deliverability score, CANOS multi-area extension, and integrate FLISR/VVO/outage coordination with reliability tracking.

**Architecture:** Monte Carlo sampling of generation/transmission outages, AC/DC OPF for each scenario, aggregate reliability metrics, extend ADMS workflows with reliability impact tracking.

**Tech Stack:** Rust 2021, good_lp (Clarabel), polars, rayon (parallel), anyhow, tokio

---

## Phase 3: Reliability Metrics (5 Tasks)

### Task 10: Set Up LOLE/EUE Monte Carlo Sampling Framework

**Goal:** Create the Monte Carlo sampling infrastructure for generating outage scenarios and computing LOLE/EUE metrics.

**Files:**
- Create: `crates/gat-algo/src/reliability_monte_carlo.rs` (NEW)
- Modify: `crates/gat-algo/src/lib.rs` (add module)
- Test: `crates/gat-algo/tests/reliability_monte_carlo.rs` (NEW)

**Key Concepts:**
- LOLE = Loss of Load Expectation (hours/year when demand > supply)
- EUE = Energy Unserved (MWh/year of unmet demand)
- Monte Carlo: Sample random outage scenarios, simulate each, aggregate results

**Implementation Approach:**
1. Define `OutageScenario` struct (which generators/lines are offline)
2. Implement `OutageGenerator` with Weibull failure rates
3. For each scenario: run AC/DC OPF, track shortfalls
4. Aggregate: LOLE = count(scenarios with shortfall) / total scenarios
5. Aggregate: EUE = sum(unmet MWh) / hours per year

**Estimated LOC:** 300-400
**Estimated Tests:** 8-10
**Estimated Time:** 3-4 hours

---

### Task 11: Implement Deliverability Score

**Goal:** Create composite reliability metric combining LOLE, voltage violations, and thermal overloads.

**Files:**
- Modify: `crates/gat-algo/src/reliability_monte_carlo.rs`
- Test: `crates/gat-algo/tests/reliability_monte_carlo.rs`

**Formula:**
```
DeliverabilityScore = 100 * [1 - w_lole * (LOLE/LOLE_max)
                                 - w_voltage * (violations/max_violations)
                                 - w_thermal * (overloads/max_overloads)]
```

**Key Features:**
- Weighted combination of reliability indicators
- Configurable weights
- Score range 0-100
- Interpreted as reliability percentage

**Estimated LOC:** 150-200
**Estimated Tests:** 4-5
**Estimated Time:** 2 hours

---

### Task 12: Implement CANOS Multi-Area Framework

**Goal:** Extend LOLE/EUE to multi-area systems with inter-area transmission and zone-to-zone impacts.

**Files:**
- Create: `crates/gat-algo/src/canos_multiarea.rs` (NEW)
- Modify: `crates/gat-algo/src/lib.rs`
- Test: `crates/gat-algo/tests/canos_multiarea.rs` (NEW)

**Key Concepts:**
- CANOS = Coordinated Automatic Network Operating System
- Multi-area: transmission corridors connect control areas
- Zone-to-zone LOLE: reliability from area A's perspective given outages in B, C, D
- Import/export limits: maximum power transfer capacity

**Implementation:**
1. Extend `OutageScenario` to include multi-area outages
2. Per-area LOLE calculation (zone-specific reliability)
3. Inter-area flow tracking during scenarios
4. Corridor constraint enforcement
5. Visualization: LOLE heatmap by area and time-of-year

**Estimated LOC:** 350-450
**Estimated Tests:** 6-8
**Estimated Time:** 4-5 hours

---

### Task 13: Integrate FLISR/VVO/Outage Coordination

**Goal:** Update ADMS workflows (FLISR, VVO, outage scheduling) to track and optimize reliability metrics.

**Files:**
- Modify: `crates/gat-adms/src/lib.rs` (existing FLISR/VVO modules)
- Modify: `crates/gat-algo/src/canos_multiarea.rs` (orchestration)
- Test: `crates/gat-adms/tests/integration_with_reliability.rs` (NEW)

**FLISR Enhancement:**
- Each switching operation recalculates post-restoration LOLE/EUE
- Track time-to-restore, load restored
- Compare: LOLE before fault vs. after FLISR

**VVO Enhancement:**
- Extend objective: minimize losses + maintain voltage margins
- Reliability-aware: don't over-optimize if it reduces Deliverability below threshold
- Trade-off: aggressiveness vs. reliability

**Outage Coordination:**
- Schedule maintenance across system to minimize peak LOLE
- Multi-area coordination: no two neighbors in maintenance during peak demand
- Algorithm: greedy or simulated annealing

**Estimated LOC:** 400-500
**Estimated Tests:** 8-10
**Estimated Time:** 5-6 hours

---

### Task 14: Validate Against Benchmarks & Complete Phase 3

**Goal:** Validate reliability metrics against published NERC/WECC benchmarks, run full integration tests, finalize Phase 3.

**Files:**
- Test: `crates/gat-algo/tests/reliability_benchmarks.rs` (NEW)
- Create: `docs/reliability_validation_results.md`

**Benchmarks:**
- NERC reliability metrics by region (LOLE typically 0.5-3 hrs/year)
- WECC publications on zone-to-zone reliability
- Published FLISR effectiveness studies
- VVO impact studies from industry

**Validation Approach:**
1. Compare LOLE to published ranges
2. Sensitivity analysis: ±10% demand, ±10% generation
3. FLISR effectiveness: post-restoration LOLE < 10% pre-fault
4. Outage scheduling: chosen schedule reduces annual EUE ≥ 5% vs. random
5. Multi-area: zone-to-zone impacts realistic

**Integration Tests:**
- Full workflow: import grid → run scenarios → compute reliability → optimize FLISR
- Multi-area workflow: 3-5 area system with corridors
- Temporal: seasonal variation (winter peak vs. summer low)

**Estimated LOC:** 250-350 (tests + docs)
**Estimated Tests:** 12-15
**Estimated Time:** 4-5 hours

---

## Phase 3 Summary

**Total Tasks:** 5
**Total Estimated LOC:** 1,450-1,900
**Total Estimated Tests:** 38-48
**Total Estimated Time:** 18-24 hours solo

**Deliverables:**
- ✅ LOLE/EUE computation with Monte Carlo sampling
- ✅ Deliverability score (composite metric)
- ✅ CANOS multi-area framework
- ✅ FLISR/VVO/outage coordination integration
- ✅ Validation against industry benchmarks
- ✅ Full integration tests
- ✅ Documentation and results

**Upon completion:** v0.3 will be feature-complete for all three phases, ready for merge to main and release.

---

## Ready to Begin Task 10?

This plan is structured for subagent-driven execution. Each task is self-contained and can be completed independently, with tests validating correctness before moving to the next task.

**Task 10 (LOLE/EUE Monte Carlo Framework) is next.**
