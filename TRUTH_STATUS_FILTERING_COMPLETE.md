# Truth Status Filtering Implementation - Complete

## Summary
Truth status filtering has been successfully implemented. The `(check ...)` command now correctly filters for **asserted facts only** in fact-tracked relations, enabling proper modal logic semantics.

## Implementation Details

### Architecture
Truth tracking is implemented via a column-based approach, parallel to subsumption:
- **SchemaMath** contains `truth_tracking: bool` field
- **FunctionInfo** contains `fact_tracking: bool` field  
- **Truth column** stores ASSERTED (1) or REFERENCED (0) status
- **Constraints** filter for ASSERTED during query planning

### Key Components

#### 1. Column Infrastructure (Commit 364b336)
- Added `truth: Option<T>` field to `RowVals` struct
- Updated `write_table_row()` to populate truth column
- Updated all RowVals construction sites (lib.rs and rule.rs)
- Truth status defaults to ASSERTED for insert/set/add operations
- Preserved during merges (take max) and subsume operations

#### 2. Schema Integration (Commit 897c90c)
- Refactored from ad-hoc parameters to proper SchemaMath structure
- Added `truth_tracking` field to SchemaMath (parallel to `subsume`)
- Added `fact_tracking` field to FunctionInfo
- Updated 15+ SchemaMath construction sites

#### 3. Constraint Generation (Commit 34396e4, 93d4e20)
- Added `require_asserted` parameter to `query_table()`
- Extended Query.atoms tuple to include `require_asserted: bool`
- Generate `Constraint::EqConst { col: truth_col(), val: ASSERTED }` in `build_cached_plan()`
- Added `will_enable_truth_tracking` to FunctionConfig for upfront column allocation

### Files Modified
- `egglog-bridge/src/lib.rs`: Core infrastructure, RowVals, FunctionConfig
- `egglog-bridge/src/rule.rs`: Query building, constraint generation
- `src/lib.rs`: FunctionConfig construction with fact_tracking flag
- `src/scheduler.rs`: Scheduler table configuration

## Usage

### Declaring Fact-Tracked Relations
```egglog
(relation R (i64 i64) :fact-tracking)
```

### Checking Facts
```egglog
;; Assert a fact
(R 1 2)
(run 1)

;; Check passes for asserted facts
(check (R 1 2))

;; Check fails for non-existent facts
(fail (check (R 3 4)))
```

### Modal Logic Use Case
```egglog
(datatype Agent)
(relation Believes (Agent FactRef) :fact-tracking)

;; Create unasserted fact reference
(let f (fact R 1 2))

;; Belief about unasserted fact
(Believes (Agent "Alice") f)

;; Check fails - fact is referenced but not asserted
(fail (check (R 1 2)))

;; Now assert it
(R 1 2)

;; Check passes - fact is now asserted
(check (R 1 2))
```

## Testing

### Test Files
- `simple_truth_test.egg`: Basic assertion and checking
- `comprehensive_truth_test.egg`: Multiple scenarios including rules

### Test Results
✅ All tests pass:
- Basic truth filtering
- Mixed tracked/untracked relations
- Rules with fact-tracked relations
- Derived facts from rules

### Known Limitations
- Inline fact creation in complex expressions may have unrelated issues
- Core filtering functionality is complete and working

## Performance Considerations
- Truth column adds one Value per row to fact-tracked relations
- Constraint generation has minimal overhead
- Column-based approach is efficient for filtering

## Future Enhancements
Potential improvements (not required for current functionality):
1. Optimize constraint generation for common patterns
2. Add statistics on asserted vs referenced fact ratios
3. Support bulk truth status updates

## Commits
1. `897c90c`: Refactor truth tracking into SchemaMath structure
2. `364b336`: Add truth status column infrastructure
3. `34396e4`: Implement truth status filtering constraints
4. `93d4e20`: Fix table arity mismatch for truth tracking

## Status
✅ **COMPLETE** - Ready for use in modal logic applications
