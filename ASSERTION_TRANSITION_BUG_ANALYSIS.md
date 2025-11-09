# Truth Status Filtering and Assertion Transition - Final Status

## Summary of Work

This commit documents the implementation of truth status filtering infrastructure and the discovery of a fundamental architectural issue with the assertion transition mechanism.

## What Was Implemented

### 1. Truth Status Update Infrastructure
- Added `is_row_asserted()` method to check if a row corresponds to an asserted fact
- Added `should_include_row()` trait method for filtering rows by truth status
- Added truth status update in `update_fact_mapping()` to mark facts as asserted when rows are updated
- Added truth status checks in both sorted and unsorted insertion paths to handle unasserted → asserted transitions

### 2. Documentation
- Created TRUTH_STATUS_BUG_REPORT.md documenting the assertion transition bug
- Created TRUTH_STATUS_FILTERING_SOLUTION.md outlining implementation approaches
- Created TRUTH_STATUS_FILTERING_TODO.md with implementation plan

## Critical Bug Discovered

**The Unification Problem**: When an unasserted fact is created and later explicitly asserted, the assertion fails due to e-graph value unification.

### Root Cause
1. Unasserted fact `(KnowsAbout (Bob) (Alice))` is created via `(Believes (Alice) (fact KnowsAbout (Bob) (Alice)))`
2. At creation time: Bob=Value(1), Alice=Value(0)
3. Row is inserted into hash table with key `[Value(1), Value(0)]`
4. E-graph unifies Alice and Bob (both become Value(0))
5. Later assertion `(KnowsAbout (Bob) (Alice))` looks for key `[Value(0), Value(0)]`
6. Hash lookup fails - different keys!
7. New row is created instead of finding existing unasserted row
8. Result: Two rows exist (one unasserted with old IDs, one asserted with canonical IDs)

### Evidence
```
DEBUG: create_or_lookup_unasserted_fact called with key=[Value(1), Value(0)]
DEBUG [sorted]: Processing insertion, key=[Value(0), Value(0)], ...
DEBUG [sorted]: No existing row found, creating new
```

## Why Current Fix Doesn't Work

The truth status update code added in this commit is CORRECT in logic:
- When merge returns false and fact is unasserted, update truth_status to true
- When fact mappings are updated, mark as asserted

However, the code is never executed because:
- The existing row is never found due to ID mismatch
- A new row is always created instead
- The unasserted row remains with old IDs and truth_status=false

## Proper Solution

The fundamental issue is that `create_or_lookup_unasserted_fact()` bypasses the normal insertion and rebuilding mechanisms. It directly inserts rows with current Value IDs, but those IDs can become stale after unification.

### Recommended Fix
**Use pending insertion queue for unasserted facts:**
1. Instead of directly manipulating tables in `create_or_lookup_unasserted_fact()`
2. Queue the fact for insertion through normal mechanisms
3. Tag the insertion as "unasserted" so truth_status is set to false
4. This ensures the row participates in rebuilding when values are unified
5. Future assertions will find the row correctly after rebuilding

### Alternative: Immediate Rebuild
- After creating unasserted fact, trigger immediate rebuild
- This ensures IDs are canonical before hash table insertion
- Less efficient but simpler to implement

## Testing

Test case that demonstrates the bug:
```egglog
(datatype Agent (Alice) (Bob))
(relation KnowsAbout (Agent Agent) :fact-tracking)
(Believes (Alice) (fact KnowsAbout (Bob) (Alice)))  ; Creates unasserted
(run 1)
(KnowsAbout (Bob) (Alice))  ; Should update to asserted, but creates duplicate
(run 1)
(check (KnowsAbout (Bob) (Alice)))  ; FAILS but should PASS
```

## Next Steps

1. Remove debug output from this commit
2. Implement proper fix using pending insertion queue approach
3. Ensure unasserted facts participate in table rebuilding
4. Re-test assertion transition
5. Implement truth status filtering in `check` command (original goal)

## Files Modified

- `core-relations/src/table/mod.rs`: Added truth status update logic
- `core-relations/src/table_spec.rs`: Added `should_include_row()` trait method
- `src/lib.rs`: Minor changes to check_facts
- Created documentation files

## Architectural Insights

This bug reveals an important architectural principle:
- **Any data inserted into tables must participate in rebuilding**
- Bypassing normal insertion mechanisms breaks e-graph invariants
- Value IDs are not stable across unification operations
- Hash table lookups require canonical IDs, which only rebuilding provides
