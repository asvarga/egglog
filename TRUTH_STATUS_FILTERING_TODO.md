# Truth Status Filtering - Implementation TODO

## Problem

Currently, `check` commands verify that facts **exist** in fact-tracked relations, but do NOT verify that they are **asserted** (truth_status = true). This breaks modal logic semantics where we need to distinguish between "a fact exists" and "a fact is asserted/true".

## Current Behavior (INCORRECT)

```egglog
(relation KnowsAbout (Agent Agent) :fact-tracking)
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
(check (KnowsAbout (Bob) (Charlie)))  ; PASSES (but should FAIL!)
```

The fact `(KnowsAbout (Bob) (Charlie))` was created as **unasserted** (truth_status = false) when referenced in the belief. The `check` should fail because the fact is not asserted, but currently it passes because the fact exists in the table.

## Desired Behavior (CORRECT)

```egglog
(relation KnowsAbout (Agent Agent) :fact-tracking)
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
(check (KnowsAbout (Bob) (Charlie)))  ; Should FAIL - fact is unasserted
(KnowsAbout (Bob) (Charlie))           ; Assert the fact
(check (KnowsAbout (Bob) (Charlie)))  ; Now should PASS - fact is asserted
```

## Implementation Approaches

### Approach 1: Filter During Scan (Preferred)

Modify the table scan mechanism to filter rows based on truth status:

1. **Add `require_asserted` parameter to scans**
   - Add to `Table::scan_generic_bounded()` 
   - Add to `TableWrapper::scan_bounded()` etc.
   - Thread through the query execution pipeline

2. **Filter rows during iteration**
   - In `SortedWritesTable::scan_generic_bounded()`, check `should_include_row(row_id, require_asserted)`
   - Skip rows where the fact is unasserted when `require_asserted = true`

3. **Pass flag from query builder**
   - Add `require_asserted` parameter to `RuleBuilder::query_table()`
   - For `check` commands, pass `require_asserted = true`
   - For regular queries, pass `require_asserted = false` (include all facts)

**Pros**: Clean, efficient, integrates with existing query mechanism
**Cons**: Requires changes across multiple layers (core-relations, egglog-bridge, egglog)

### Approach 2: Post-Process Query Results (Simpler but Less Efficient)

After the query runs, check truth status for each matched row:

1. **Capture matched rows during query execution**
   - Modify the external function callback to record matched rows
   
2. **Post-process in `check_facts()`**
   - For each fact in a fact-tracked relation, look up the row
   - Check `is_fact_asserted()` and fail if false

**Pros**: Simpler, doesn't require core-relations changes
**Cons**: Less efficient, doesn't help with rules (only `check`)

### Approach 3: Add Truth Status as a Virtual Column (Complex)

Treat truth status like the subsumption column:

1. **Add truth status column to table schema**
   - Modify `SchemaMath` to include truth column
   - Store truth_status as a regular column value

2. **Use constraints for filtering**
   - Query with constraint `truth_col = ASSERTED`
   - Leverage existing constraint mechanism

**Pros**: Uses existing constraint infrastructure
**Cons**: Truth status is currently fact-level (HashMap), not row-level (column)
         Would require restructuring the fact tracking implementation

## Recommended Implementation Plan

**Phase 1: Core-Relations**
1. Add `should_include_row(row_id, require_asserted)` to `Table` trait ✅ (DONE)
2. Implement for `SortedWritesTable` ✅ (DONE)
3. Add `require_asserted` parameter to scan methods
4. Call `should_include_row()` during row iteration

**Phase 2: Egglog-Bridge**
1. Add `require_asserted` parameter to `RuleBuilder::query_table()`
2. Thread through to core-relations scan calls

**Phase 3: Egglog**
1. Modify `check_facts()` to pass `require_asserted = true`
2. Modify regular query building to pass `require_asserted = false`

**Phase 4: Testing**
1. Update `minimal_unasserted_demo.egg` to use `fail` for unasserted checks
2. Add comprehensive tests for truth status filtering
3. Verify modal logic semantics are correct

## Current Status

- ✅ Added `is_row_asserted()` to `SortedWritesTable`
- ✅ Added `should_include_row()` to `Table` trait
- ✅ Implemented `should_include_row()` for `SortedWritesTable`
- ⏳ TODO: Add `require_asserted` parameter to scan methods
- ⏳ TODO: Thread through egglog-bridge
- ⏳ TODO: Update `check_facts()` to use filtering

## Notes

- Truth status is stored in `fact_truth_status: HashMap<FactId, bool>`
- Each row with fact tracking has a FactId in `fact_id_map: HashMap<RowId, FactId>`
- Subsumption uses a similar pattern but stores the flag as a column
- We could migrate truth status to a column, but current HashMap approach works
