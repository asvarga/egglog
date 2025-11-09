# Truth Status Filtering Solution

## Problem Statement

The `check` command currently verifies that facts **exist** in tables, but does not verify that they are **asserted** (truth_status = true). This is incorrect for modal logic semantics where we need to distinguish between:
- **Asserted facts**: Facts that are claimed to be true
- **Referenced facts**: Facts that exist in the table but are not claimed to be true (e.g., belief facts)

### Example
```egglog
(relation KnowsAbout (Agent Agent) :fact-tracking)
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))

(check (KnowsAbout (Bob) (Charlie))) ;; Should FAIL but currently PASSES
```

The fact `(KnowsAbout (Bob) (Charlie))` exists in the table (created as referenced/unasserted), but `check` should fail because it's not asserted.

## Root Cause

Truth status is currently stored in a `HashMap<FactId, bool>` at the table level, not as a column in the table rows. This means:
1. Truth status cannot be filtered using the existing constraint mechanism (which operates on columns)
2. Query execution doesn't have direct access to truth status during scans
3. The `check` command can't distinguish asserted from referenced facts

## Solution Approaches

### Approach A: Convert Truth Status to Column (Recommended)

**Pros:**
- Architecturally consistent with subsumption (which uses a column)
- Works with existing constraint mechanism (`Constraint::EqConst`)
- Can filter efficiently during table scans
- Extends naturally to other query types (not just `check`)

**Cons:**
- Requires adding a column to fact-tracked tables
- More invasive changes across multiple layers
- Need to update row insertion/update logic

**Implementation Steps:**
1. Add `truth_enabled: bool` to `FunctionInfo` (egglog-bridge)
2. Add truth status column to table schema when `fact_tracking` enabled
3. Update `SchemaMath` to account for truth column (similar to subsume column)
4. Modify row insertion to set truth status column value
5. Add `is_asserted: Option<bool>` parameter to `query_table()`
6. Create constraint: `EqConst { col: truth_col, val: ASSERTED }`
7. Update `check_facts()` to pass `is_asserted = Some(true)`

**Files to modify:**
- `egglog-bridge/src/lib.rs`: Add truth column to schema
- `egglog-bridge/src/rule.rs`: Add `is_asserted` parameter to `query_table()`
- `src/lib.rs`: Update `check_facts()` and `BackendRule::query()`

### Approach B: Post-Query Filtering (Quick Fix)

**Pros:**
- Simpler implementation
- Works with current HashMap storage
- Minimal changes required

**Cons:**
- Only works for `check` commands
- Less efficient (filters after query runs)
- Doesn't extend to general queries
- Requires capturing matched rows (current `check` mechanism doesn't expose this)

**Challenges:**
- Current `check_facts()` uses a side-channel to detect if ANY match occurred
- Doesn't capture which specific rows matched
- Would need to restructure check mechanism to enumerate matches

## Current Status

- **Foundational methods added:**
  - `SortedWritesTable::is_row_asserted()` - checks truth status of a row
  - `Table::should_include_row()` - trait method for filtering
  
- **Architecture note:**
  - Truth status column infrastructure exists but is unused (`#[allow(dead_code)]`)
  - Methods `SchemaMath::truth_col()` and `table_columns_with_truth()` are present

- **Blocking issue:**
  - Need to decide between Approach A (proper) vs Approach B (quick)
  - Approach A requires significant changes but is correct
  - Approach B requires restructuring `check_facts` to capture matches

## Recommendation

**Implement Approach A** (truth status as column) because:
1. It's architecturally correct and consistent with subsumption
2. The infrastructure already exists (unused truth column methods)
3. It will work correctly for all use cases, not just `check`
4. Modal logic will likely need this filtering in rules too, not just checks

**Implementation Priority:**
1. HIGH: Add truth column to fact-tracked tables
2. HIGH: Update `query_table()` to accept `is_asserted` parameter
3. HIGH: Modify `check_facts()` to pass `is_asserted = Some(true)`
4. MEDIUM: Add integration tests for asserted vs unasserted checks
5. LOW: Optimize by making truth filtering a "fast" constraint

## Testing Plan

After implementation:
```egglog
(relation KnowsAbout (Agent Agent) :fact-tracking)

;; Create unasserted fact
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
(run 1)

;; This should FAIL (unasserted fact)
(fail (check (KnowsAbout (Bob) (Charlie))))

;; Assert the fact
(KnowsAbout (Bob) (Charlie))
(run 1)

;; Now this should PASS (asserted fact)
(check (KnowsAbout (Bob) (Charlie)))
```

## References

- Truth status constants: `ASSERTED` and `REFERENCED` in `egglog-bridge/src/lib.rs:2006-2007`
- Subsumption precedent: `is_subsumed` parameter in `query_table()`
- Schema calculation: `SchemaMath::truth_col()` and `table_columns_with_truth()`
- Current limitation: Documented TODO in `src/lib.rs` `check_facts()`
