# Import Robustness Findings - Spike 1 Baseline

**Date:** 2025-11-26
**Status:** Complete (with fix applied)

## Summary

Ran baseline tests against the existing test corpus. **Critical bug found in PSS/E parser.**

## Test Corpus

| Format | Files | Source |
|--------|-------|--------|
| MATPOWER PGLib | 7 files (14-2869 buses) | pglib-opf benchmarks |
| MATPOWER Edge Cases | 5 files (5-30 buses) | MATPOWER repo |
| PSS/E ICSEG | 4 files (14-118 buses) | Illinois ICSEG |
| PSS/E sample | 1 file (2 buses) | Internal test file |
| CIM | 1 file (simple.rdf) | Internal test file |

## Results

### MATPOWER - All Pass ✅

| File | Status | Notes |
|------|--------|-------|
| pglib_opf_case14_ieee.m | ✅ | Clean import |
| pglib_opf_case30_ieee.m | ✅ | Clean import |
| pglib_opf_case57_ieee.m | ✅ | Clean import |
| pglib_opf_case118_ieee.m | ✅ | Clean import |
| pglib_opf_case300_ieee.m | ✅ | Clean import |
| pglib_opf_case1354_pegase.m | ✅ | Clean import |
| pglib_opf_case2869_pegase.m | ✅ | Clean import |
| case5.m | ✅ | Clean import |
| case9.m | ✅ | Clean import |
| case9target.m | ✅ | Clean import |
| case24_ieee_rts.m | ✅ | Clean import |
| case_ieee30.m | ✅ | Clean import |

### PSS/E - Critical Bug Found ❌

| File | Status | Notes |
|------|--------|-------|
| sample.raw | ✅ | Works (uses old format) |
| ieee-14-bus.raw | ❌ **DATA LOSS** | Produces empty network |
| ieee-39-bus.raw | ❌ **DATA LOSS** | Produces empty network |
| ieee-57-bus.raw | ❌ **DATA LOSS** | Produces empty network |
| IEEE_118_bus.raw | ❌ **DATA LOSS** | Produces empty network |

**Root Cause:** Parser expects old-style section markers like `BUS DATA FOLLOWS` but ICSEG files use v33+ format with `0 / END OF BUS DATA, BEGIN LOAD DATA`.

**Evidence:**
- MATPOWER ieee14 → 11,723 bytes Arrow output (14 buses, 11 loads, 5 gens, 20 branches)
- PSS/E ieee14 → 3,523 bytes Arrow output (empty network, header only)

### CIM - Pass ✅

| File | Status | Notes |
|------|--------|-------|
| simple.rdf | ✅ | Clean import |
| entsoe/ | ⏭️ | Directory empty, skipped |

## Observations

### Silent Failure Patterns

1. **PSS/E v33+ format completely unsupported** - Files import with exit code 0 but produce empty networks
2. **MATPOWER skips disabled elements silently** - Generators/branches with status=0 are skipped without warning
3. **No import statistics** - User has no way to verify correct element counts

### Current Parser Limitations

1. **MATPOWER Parser** (`matpower.rs`)
   - Silently skips generators with `gen_status == 0`
   - Silently skips branches with `br_status == 0`
   - Hard error if generator references unknown bus (good!)
   - No summary statistics on import

2. **PSS/E Parser** (`psse.rs`)
   - Only supports old format section markers
   - Silent skip on malformed lines (returns `None`)
   - Uses `unwrap_or(0.0)` for parse failures (silent defaults)
   - No validation of expected vs actual element counts

## Recommended Actions

### P0 - Critical

1. **Fix PSS/E v33+ format support** - Parse `0 / END OF ... DATA` section markers
2. **Add import statistics** - Report: "Imported X buses, Y branches, Z generators, W loads"

### P1 - Important

3. **Add warning collection** - Track skipped elements with reasons
4. **Add `--strict` mode** - Fail on first warning instead of collecting
5. **Add element count validation** - Compare expected vs actual (if format specifies counts)

### P2 - Nice to Have

6. **Auto-detect PSS/E version** - Sniff file header for format version
7. **Encoding detection** - Handle BOM, Windows line endings, non-ASCII names

## Fix Applied

### PSS/E v33+ Parser Fix (same session)

**Changes made to `crates/gat-io/src/importers/psse.rs`:**

1. Added `detect_psse_version()` - sniffs header for version number in field 3
2. Added `check_v33_section_marker()` - parses "0 / END OF X, BEGIN Y" markers
3. Added v33-specific line parsers with correct column positions
4. Parser now auto-detects format and uses appropriate parsing strategy

**Verification Results:**

| File | Before Fix | After Fix |
|------|-----------|-----------|
| ieee-14-bus.raw | 0 buses (empty) | 14 buses, 17 branches, 11 loads, 5 gens |
| ieee-39-bus.raw | 0 buses (empty) | 39 buses, 34 branches, 31 loads, 10 gens |
| ieee-57-bus.raw | 0 buses (empty) | 57 buses, 65 branches, 42 loads, 7 gens |
| IEEE_118_bus.raw | 0 buses (empty) | 118 buses, 177 branches, 99 loads, 54 gens |
| sample.raw (old) | 2 buses, 1 branch | 2 buses, 1 branch (unchanged) |

**All tests pass:** `cargo test --package gat-io` - 10/10 passed

### Known Limitations (post-fix)

1. ~~**Transformers not parsed** - TRANSFORMER DATA section is skipped~~ **FIXED** (see below)
2. **Import statistics not surfaced to user** - the `Parsed X buses, Y branches...` message is println, not structured output
3. **No warnings for skipped elements** - still silently skips malformed lines

### Transformer Parsing Added (same session)

**Changes made to `crates/gat-io/src/importers/psse.rs`:**

1. Added `parse_psse_transformer_v33()` - parses multi-line transformer records (4 lines for 2-winding)
2. Updated main parsing loop to collect transformer lines and detect record boundaries
3. Transformers are converted to `PsseBranch` with `tap_ratio` and `phase_shift_rad` values

**Verification Results (with transformers):**

| File | Before Xfmr | After Xfmr | Transformers |
|------|-------------|------------|--------------|
| ieee-14-bus.raw | 17 branches | 20 branches | +3 |
| ieee-39-bus.raw | 34 branches | 46 branches | +12 |
| ieee-57-bus.raw | 65 branches | 80 branches | +15 |
| IEEE_118_bus.raw | 177 branches | 186 branches | +9 |
| sample.raw (old) | 1 branch | 1 branch | unchanged |

**All tests pass:** `cargo test --package gat-io` - 10/10 passed

## Next Steps

1. ~~**Spike 2:** Fix PSS/E v33+ parser (critical path)~~ **DONE**
2. ~~**Spike 2:** Add transformer parsing to PSS/E importer~~ **DONE**
3. **Spike 3:** Build ImportDiagnostics infrastructure
4. Re-run baseline periodically as more test files are added

---

## Appendix: File Format Differences

### Old PSS/E Format (sample.raw)
```
BUS DATA FOLLOWS
1,'BUS 1',138.0,...
END OF BUS DATA
BRANCH DATA FOLLOWS
1,2,1,0.01,0.1,...
END OF BRANCH DATA
```

### PSS/E v33+ Format (ICSEG files)
```
0,    100.00, 33, 0, 0, 60.00       / PSS/E version in header
...
    1,'Bus 1       ', 138.0000,3,...
0 / END OF BUS DATA, BEGIN LOAD DATA
    2,'1 ',1,   1,   1,    21.700,...
0 / END OF LOAD DATA, BEGIN FIXED SHUNT DATA
```

Key differences:
- v33+ uses `0 /` as section terminator
- v33+ has version number in header (field 3)
- v33+ has combined end/begin markers
- v33+ has more columns per record
