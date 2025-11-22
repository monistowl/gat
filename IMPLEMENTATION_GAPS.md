# GAT Implementation Gaps & Blockers

**Purpose:** Identify specific, actionable items remaining from the roadmap that are either not yet implemented, partially implemented, or blocked by other work.

**Current Status:** All unit tests passing (56/56), no build errors.

---

## Critical Issues (Block Agent Productivity)

### Issue 1: Ambiguous In-Progress Status (5 issues)
**Impact:** Unclear what work remains; duplicates may exist; blocks planning

**Details:**
- **gat-0z1**: "gat scenarios crate/CLI" — marked in_progress but crate/commands appear complete
- **gat-cal**: "gat analytics reliability" — marked in_progress but gat-yvt (similar name) is closed
- **gat-kko**: "gat analytics elcc" — marked in_progress but gat-3at (same functionality) is closed
- **gat-xvi**: "BMCL geo join/featurize" — marked in_progress but gat-tya/gat-tvl are closed

**Required Action:**
```bash
# For each in_progress issue, determine:
# 1. Is there a closed issue doing the same thing?
# 2. What specifically is remaining?
# 3. Should it be closed, or is there refinement work?

bd list --json | jq '.[] | select(.status == "in_progress")'
# Then review each against closed issues
```

**Recommendation:**
- If feature is complete, close with reason "Completed and merged to main"
- If incomplete, add concrete subtasks or blockers
- Remove duplicates and consolidate

---

### Issue 2: MCP Server Integration Incomplete
**Impact:** Agents cannot introspect command/schema definitions; reduces automation value

**Crate:** `gat-mcp-docs`
**Current State:**
- Generates static Markdown docs
- Has MCP protocol stubs but no actual tools/resources exported

**Missing Pieces:**
1. MCP tool definitions for major CLI commands (pf, batch, analytics, featurize, etc.)
2. MCP resource definitions for Parquet schemas (metadata about output formats)
3. Server endpoints to list available schemas + their fields
4. Integration test showing agent can call `gat` commands via MCP

**What to Implement:**
```rust
// In gat-mcp-docs or new gat-mcp-server crate:
// 1. Define Tool for each major subcommand:
//    - Tool(name="gat-batch-pf", description="...", inputSchema={...})
//    - Tool(name="gat-analytics-reliability", ...)
// 2. Provide Resource type for schemas:
//    - Resource(uri="schema://gat/parquet/pf_result", content={...})
// 3. Implement MCP server endpoint (/tools, /resources, /call)
```

**Estimated Effort:** Medium (2-3 days for basic coverage)

---

### Issue 3: Scenario Validation Incomplete
**Issue ID:** gat-0z1
**Impact:** Scenario expansion may fail on complex templates; limits CANOS usability

**Current State:**
- Basic YAML/JSON parsing works
- Scenario spec types defined
- Materialization to per-scenario grids works

**Likely Gaps:**
1. Template expansion (cartesian products, conditional scenarios) not fully tested
2. Validation error messages may be unclear
3. No integration test showing full pipeline: spec → materialize → batch pf → analytics

**What to Implement:**
1. Add comprehensive validation tests for scenario spec edge cases:
   - Empty outage lists
   - Missing load_scale defaults
   - Duplicate scenario IDs
   - Invalid time_slice formats
2. Add integration test: scenario spec → batch pf → reliability metrics
3. Document expected validation error messages

**Estimated Effort:** Small (1-2 days for testing/docs)

---

## Medium-Priority Gaps (Nice-to-Have)

### Gap 1: Schema Auto-Documentation
**Crate:** `gat-schemas`
**Current State:**
- Type definitions exist (Rust structs with serde)
- Manual Markdown docs in docs/ directory
- No auto-generated schema reference

**Missing:**
1. JSON Schema generation from Rust types (use `schemars` crate)
2. Auto-generated reference docs (one page per Parquet table format)
3. MCP resource linking (agents can ask "what's in pf_result.parquet?")

**What to Implement:**
```bash
# In gat-schemas/build.rs or via CLI tool:
# Generate JSON Schema for each public struct
# Output to docs/schemas/*.json
# Create index.md linking all schemas
```

**Estimated Effort:** Small-Medium (1-2 days)

---

### Gap 2: End-to-End Pipeline Integration Test
**Impact:** Cannot verify data lineage; hard to onboard new agents

**Missing Test:**
1. Load realistic grid (MATPOWER format)
2. Define scenario spec (JSON/YAML)
3. Run `gat scenarios materialize` → verify per-scenario grids exist
4. Run `gat batch pf` → verify outputs match expected schema
5. Run `gat analytics reliability` → verify metrics computed
6. Run `gat featurize gnn` → verify node/edge tables valid
7. Assert data passes through with correct shapes/types

**Location:** `crates/gat-cli/tests/integration_test.rs` or new `tests/e2e_pipeline.rs`

**Estimated Effort:** Small-Medium (2-3 days)

---

### Gap 3: Comprehensive CLI Documentation
**Impact:** Users/agents need clear reference for all subcommands + options

**Missing:**
1. Single "CLI Reference" page with all commands grouped
2. For each command: purpose, inputs, outputs (with Parquet schema links), example usage
3. Common patterns (scenario spec format, batch manifest format)
4. Troubleshooting guide (common errors + solutions)

**Location:** `docs/guide/cli-reference.md`

**Estimated Effort:** Small (1-2 days for docs; no code)

---

## Low-Priority / Optional Gaps

### Optional 1: Advanced DERMS Features
**Current:** Heuristic price-responsive scheduling + stress testing
**Missing:**
- Optimal dispatch solver (MILP formulation in gat-derms crate)
- Real-time coordination with frequency/voltage signals
- Integration with VVO/FLISR from gat-adms

**When to implement:** If DERMS is core to use case; otherwise defer

---

### Optional 2: OpenDSS Compatibility (gat-dist)
**Current:** AC power flow + hosting capacity sweeps (MATPOWER-like)
**Missing:**
- Three-phase unbalanced power flow
- Detailed cable impedance modeling
- Harmonic analysis
- Direct OpenDSS importer

**When to implement:** If distribution engineering detail is critical

---

### Optional 3: BMCL Co-Simulation Engine
**Current:** Data fabric in place (geo join, featurize geo)
**Missing:**
- Agent behavior models (customer, prosumer, DER operator)
- Grid-feedback mechanism (how agent actions affect grid state)
- Orchestration loop (simulate N timesteps, update controls, repeat)

**When to implement:** If BMCL research is active; otherwise leave as data API

---

### Optional 4: Stochastic OPF for RA Modeling
**Current:** Deterministic OPF used in reliability metrics
**Missing:**
- Robust OPF formulation (hedging against uncertain load/renewable)
- Chance-constrained OPF (probabilistic feasibility)
- Sample approximation / scenario-based stochastic OPF

**When to implement:** If RA accreditation requires uncertainty quantification

---

## Action Plan to Close Gaps

### Phase 1 (This Session): Triage & Resolve In-Progress Issues
**Time Estimate:** 2-4 hours
1. Review each in_progress issue; check for duplicates
2. Update issue status (close completed, replan if not)
3. Commit `.beads/issues.jsonl` with any changes

**Commands:**
```bash
bd list --json | jq '.[] | select(.status == "in_progress") | {id, title}'
# Then for each:
bd update <id> --status closed --reason "Completed"  # if done
bd update <id> --description "Specific remaining work" # if ongoing
```

---

### Phase 2 (Next Session): Implement MCP Server
**Time Estimate:** 2-3 days
1. Define MCP Tool for each major CLI subcommand
2. Define MCP Resource for each Parquet schema
3. Implement MCP server endpoints
4. Test with sample agent call

**Crates to touch:**
- `gat-mcp-docs` or new `gat-mcp-server`
- `gat-schemas` (for schema definitions)
- `gat-cli` (ensure command introspection available)

---

### Phase 3 (Following): End-to-End Testing + Docs
**Time Estimate:** 2-3 days
1. Create integration test for full pipeline
2. Write CLI reference guide
3. Add JSON Schema generation + auto-docs

---

## Questions for Team/Stakeholders

1. **Scenario expansion:** Is the current validation sufficient, or do you need advanced template features (conditionals, generators)?
2. **MCP priority:** How critical is MCP server integration for your agent orchestration?
3. **DERMS/distribution:** Will you need three-phase modeling or OpenDSS import?
4. **BMCL scope:** Is co-simulation engine part of this release, or is the data fabric sufficient?
5. **RA modeling:** Do you need stochastic OPF, or is deterministic adequate?

---

## Summary Table

| Gap | Severity | Effort | Blocker? | Recommendation |
|-----|----------|--------|----------|-----------------|
| Ambiguous in_progress issues | 🔴 Critical | 2-4h | Yes | Triage & close now |
| MCP server incomplete | 🟡 Medium | 2-3d | Maybe | Do if agent automation is priority |
| Scenario validation edge cases | 🟡 Medium | 1-2d | No | Add tests for robustness |
| Schema auto-docs | 🟢 Low | 1-2d | No | Nice-to-have for API clarity |
| E2E pipeline test | 🟡 Medium | 2-3d | No | Improves confidence, helps onboarding |
| CLI reference guide | 🟢 Low | 1-2d | No | Helps users; docs-only |
| Advanced DERMS (optimal) | 🟢 Low | TBD | No | Defer unless core use case |
| OpenDSS / 3-phase dist | 🟢 Low | TBD | No | Defer unless critical |
| BMCL co-sim engine | 🟢 Low | TBD | No | Defer; data fabric is sufficient |
| Stochastic OPF | 🟢 Low | TBD | No | Defer unless RA accreditation needs it |

---

**Next Step:** Discuss which gaps to prioritize; begin with triage of in_progress issues.
