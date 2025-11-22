# Phase 2: Real Data Integration with gat-core

## Overview

Phase 2 implements real data access by creating `GatCoreQueryBuilder` that directly integrates with gat-core, replacing the `MockQueryBuilder` fixture data. This enables gat-tui to work with actual power system grids and provide real analytics.

## Goals

1. Add gat-core, gat-io, gat-algo as dependencies
2. Create `GatCoreQueryBuilder` implementing the `QueryBuilder` trait
3. Map gat-core types to gat-tui data structures
4. Implement grid loading from Arrow/Matpower/PSSE files
5. Provide real network analytics via gat-core graph functions
6. Support grid switching and caching
7. All tests passing with real data integration

## Architecture

```
gat-tui (Terminal UI)
    ↓
QueryBuilder trait (abstraction)
    ├─ MockQueryBuilder (fixtures, for testing)
    └─ GatCoreQueryBuilder (real data, for production)
        ├─ GridService (network management)
        │   ├─ gat-io (load networks from files)
        │   └─ gat-core (graph analysis)
        └─ Type mappers (gat-core → gat-tui types)
```

## Implementation Tasks

### Task 1: Add Dependencies and Type Bridges
**Expected Changes:**
- Update `Cargo.toml` to add gat-core, gat-io, gat-algo
- Create type mapping module in `services/type_mappers.rs`
- Extend `data.rs` with gat-core type bridges

**Files:**
- `crates/gat-tui/Cargo.toml`
- `crates/gat-tui/src/services/type_mappers.rs` (NEW)
- `crates/gat-tui/src/data.rs`

**Deliverables:**
- Compile without errors
- Type converters for: Network → DatasetEntry, GraphStats → SystemMetrics
- New types: GridReference, GridMetrics

### Task 2: Create GridService
**Expected Changes:**
- Create `services/grid_service.rs` with GridService struct
- Implement grid loading from files via gat-io
- Cache loaded networks in memory
- Provide methods: load_grid, get_grid, list_grids, get_grid_stats

**Files:**
- `crates/gat-tui/src/services/grid_service.rs` (NEW)
- `crates/gat-tui/src/services/mod.rs` (update exports)

**Deliverables:**
- GridService can load networks from Arrow files
- Networks cached by ID in Arc<Mutex<>>
- Error handling for missing files, corrupted data
- Tests: load_grid success/error, grid caching

### Task 3: Implement GatCoreQueryBuilder
**Expected Changes:**
- Create `services/gat_core_query_builder.rs` with GatCoreQueryBuilder struct
- Implement `QueryBuilder` trait using GridService
- Map gat-core Network/GraphStats to gat-tui types
- Handle concurrent access with Arc<Mutex<>>

**Files:**
- `crates/gat-tui/src/services/gat_core_query_builder.rs` (NEW)
- `crates/gat-tui/src/services/mod.rs` (update exports)

**Deliverables:**
- GatCoreQueryBuilder implements QueryBuilder trait
- All 6 methods: get_datasets, get_dataset, get_workflows, get_metrics, get_pipeline_config, get_commands
- Error handling for queries with no loaded grids
- Tests: basic queries, error cases

### Task 4: Update AppState to Support Grid Switching
**Expected Changes:**
- Add `current_grid_id` to AppState
- Add method to switch grids
- Invalidate caches when grid changes
- Update initialization to use GatCoreQueryBuilder

**Files:**
- `crates/gat-tui/src/models.rs`

**Deliverables:**
- Grid switching logic
- Cache invalidation
- AppState::new() uses GatCoreQueryBuilder
- Tests: grid switching, cache invalidation

### Task 5: Integration Tests with Sample Grids
**Expected Changes:**
- Add sample grid files (Arrow format or Matpower .m)
- Create integration tests loading real grids
- Test complete async flow with real data
- Verify data transformations

**Files:**
- `test_data/grids/*.arrow` or `test_data/grids/*.m` (NEW)
- `crates/gat-tui/src/services/tests/` (add grid tests)

**Deliverables:**
- 5+ integration tests with real grids
- Verify datasets load correctly
- Verify metrics calculated properly
- All tests passing

### Task 6: QueryBuilder Trait Enhancement (Optional)
**Expected Changes:**
- Add methods for more specific queries if needed
- Add caching/invalidation control
- Consider pagination for large grids

**Files:**
- `crates/gat-tui/src/services/query_builder.rs`

**Deliverables:**
- Enhanced trait if needed
- Backward compatible
- Tests for new methods

### Task 7: Documentation and Verification
**Expected Changes:**
- Add module documentation
- Document type mappers
- Create Phase 2 completion doc

**Files:**
- `docs/PHASE2_COMPLETION.md` (NEW)
- Module doc comments

**Deliverables:**
- Clear documentation of integration
- All tests passing (160+ expected)
- Release build successful
- Zero compiler errors

## Type Mapping Strategy

### DatasetEntry (gat-tui) ← Network (gat-core)

```rust
pub fn network_to_dataset_entry(id: &str, network: &Network) -> DatasetEntry {
    let stats = graph_stats(network).unwrap_or_default();
    DatasetEntry {
        id: id.to_string(),
        name: format!("Grid {}", id),
        status: DatasetStatus::Ready,
        source: "gat-core".to_string(),
        row_count: stats.node_count,
        size_mb: estimate_size(network),
        last_updated: SystemTime::now(),
        description: format!("Power grid with {} nodes, {} edges",
                           stats.node_count, stats.edge_count),
    }
}
```

### SystemMetrics (gat-tui) ← GraphStats (gat-core)

```rust
pub fn graph_stats_to_system_metrics(stats: &GraphStats) -> SystemMetrics {
    SystemMetrics {
        deliverability_score: (1.0 - stats.density) * 100.0,  // Inverse of density
        lole_hours_per_year: stats.connected_components as f64 * 8.76,  // Islands metric
        eue_mwh_per_year: (stats.node_count as f64 / 100.0) * 10.0,  // Scale metric
    }
}
```

## Error Handling

All operations return `Result<T, QueryError>`:
- `NotFound` - Grid file not found
- `ConnectionFailed` - Cannot read grid file
- `Timeout` - Grid too large (optional optimization)
- `ParseError` - Corrupted grid data
- `Unknown` - Other errors

## Testing Strategy

1. **Unit Tests** - Type converters, GridService methods
2. **Integration Tests** - Real grid loading, async flow
3. **Compatibility Tests** - MockQueryBuilder still works (backward compatibility)
4. **Performance Tests** - Grid loading time, memory usage

## Success Criteria

- ✅ GatCoreQueryBuilder compiles without errors
- ✅ Can load real grid files (Arrow format)
- ✅ All QueryBuilder methods work with real grids
- ✅ AsyncMessage flow works end-to-end
- ✅ 160+ tests passing
- ✅ Release build successful
- ✅ No compiler errors
- ✅ Backward compatible with MockQueryBuilder for tests

## Blockers & Unknowns

1. **Grid File Format** - Will use Arrow format (native to gat-io)
2. **Sample Grids** - Need to create or find sample grids in Arrow format
3. **Type Compatibility** - Some gat-core types may need wrapping
4. **Thread Safety** - Using Arc<Mutex<>> for shared NetworkManager

## Timeline

This is not a timeline, just task ordering:
1. Dependencies & type bridges
2. GridService implementation
3. GatCoreQueryBuilder implementation
4. AppState updates
5. Integration tests
6. Optional enhancements
7. Documentation & verification

## Future Considerations (Phase 3+)

- Lazy loading of large grids
- Network caching with LRU eviction
- Background grid indexing
- Real-time grid monitoring
- Solver integration for power flow visualization
- Grid comparison/diffing tools
