# Truth Status Filtering - Implementation Complete (API Layer)

## Status: API FULLY WIRED ✅ | CORE FILTERING TODO ⏳

All API changes for truth status filtering are complete. The `require_asserted` parameter is now properly threaded from `check_facts()` through `BackendRule::query()` to `RuleBuilder::query_table()`. The system correctly identifies fact-tracked relations and requests assertion filtering.

**What remains**: Implementing the actual row filtering in the core-relations layer.

## Completed Work (Commits 394fac0, 85bbc3c)

### ✅ Task 1: Add require_asserted to query_table
**Commit**: 394fac0

- Added `require_asserted: Option<bool>` parameter to `RuleBuilder::query_table()`
- Updated all 12 call sites across the codebase
- Parameter accepted but not yet used (pending core-relations implementation)

### ✅ Task 2: Update BackendRule::query
**Commit**: 85bbc3c  

Modified `BackendRule::query()` to:
```rust
fn query(&mut self, query: &core::Query<ResolvedCall, ResolvedVar>, 
         include_subsumed: bool, 
         require_asserted: bool) {
    for atom in &query.atoms {
        match &atom.head {
            ResolvedCall::Func(f) => {
                // ...
                let function = &self.functions[&f.name];
                let require_asserted_for_query = 
                    if function.decl.fact_tracking && require_asserted {
                        Some(true)
                    } else {
                        None
                    };
                self.rb.query_table(func_id, &args, is_subsumed, require_asserted_for_query)?;
            }
        }
    }
}
```

**Key Logic**: Only passes `require_asserted=true` to `query_table()` for fact-tracked relations. Non-fact-tracked relations always get `None`.

### ✅ Task 3: Update check_facts() calls

Updated all `query()` call sites with appropriate `require_asserted` values:

| Call Site | include_subsumed | require_asserted | Reason |
|-----------|-----------------|------------------|---------|
| `add_rule()` | false | false | Regular rules see all facts |
| `check_facts()` (per-fact) | false | **true** | Check requires assertion |
| `check_facts()` (final) | false | **true** | Check requires assertion |
| `scheduler query` | true | false | Scheduler sees all facts |

## Current Architecture

### Data Flow

```
check_facts()
    ↓ require_asserted=true
BackendRule::query()
    ↓ Detects fact_tracking
    ↓ Sets require_asserted_for_query = Some(true) for fact-tracked relations
RuleBuilder::query_table()
    ↓ Accepts require_asserted parameter
    ↓ TODO: Pass to query execution
Query Execution (core-relations)
    ↓ TODO: Call should_include_row(row_id, require_asserted)
    ↓ TODO: Filter out unasserted facts
Result: Only asserted facts matched
```

### What Works Now

1. ✅ `check` commands request assertion filtering
2. ✅ System detects which relations have fact tracking
3. ✅ Only fact-tracked relations get filtering requests
4. ✅ Non-fact-tracked relations are unaffected
5. ✅ Regular rules (non-check) see all facts

### What Doesn't Work Yet

❌ **Actual row filtering not implemented**

The `require_asserted` parameter is accepted and properly threaded through the API, but nothing in the core-relations layer actually uses it yet. The query execution still returns all rows (both asserted and unasserted).

## Remaining Work: Core Filtering Implementation

Only ONE task remains to complete truth status filtering:

### Task 4: Implement Core Filtering

**Two possible approaches:**

#### Approach A: Column-Based Filtering (Recommended in docs)

1. Add `truth_enabled` to `FunctionInfo`
2. Add truth status column to table schema
3. Update `SchemaMath` to handle truth column
4. Pass constraint `EqConst { col: truth_col, val: ASSERTED }`
5. Existing constraint mechanism filters rows

**Pros**: Architecturally consistent, works with existing systems
**Cons**: More invasive, requires schema changes

#### Approach B: Scan-Time Filtering (Simpler)

1. Add `require_asserted` parameter to `scan_generic_bounded()`
2. Call `should_include_row(row_id, require_asserted)` during iteration
3. Skip rows where fact is unasserted

**Pros**: Simpler, less invasive
**Cons**: Doesn't leverage existing constraint infrastructure

### Infrastructure Already in Place

```rust
// In core-relations/src/table/mod.rs

fn should_include_row(&self, row_id: RowId, require_asserted: bool) -> bool {
    if !require_asserted || !self.truth_enabled {
        return true;  // No filtering
    }
    // Filter by truth status
    match self.is_row_asserted(row_id) {
        Some(true) => true,   // Asserted - include
        Some(false) => false, // Unasserted - exclude
        None => true,         // No fact tracking - include
    }
}

fn is_row_asserted(&self, row_id: RowId) -> Option<bool> {
    if let Some(fact_id) = self.fact_id_map.get(&row_id) {
        self.fact_truth_status.get(fact_id).copied()
    } else {
        None  // Row has no fact tracking
    }
}
```

**These methods exist and work correctly!** They just need to be called from the query execution layer.

## Testing Plan

Once core filtering is implemented:

```egglog
(datatype Agent (Alice) (Bob) (Charlie))
(relation KnowsAbout (Agent Agent) :fact-tracking)

;; Create unasserted fact
(let f (fact KnowsAbout (Bob) (Charlie)))
(Believes (Alice) f)
(run 1)

;; Should FAIL - fact exists but is unasserted
(fail (check (KnowsAbout (Bob) (Charlie))))

;; Assert the fact
(KnowsAbout (Bob) (Charlie))
(run 1)

;; Now should PASS - fact is asserted
(check (KnowsAbout (Bob) (Charlie)))

;; Retract it
(retract-fact f)
(run 1)

;; Should FAIL again - fact exists but is unasserted
(fail (check (KnowsAbout (Bob) (Charlie))))
```

## Impact

### What This Enables

Once complete, this fixes a critical bug in modal logic semantics:

**BEFORE** (incorrect):
- `check` passes if fact exists (regardless of truth status)
- Can't distinguish between "believed but not true" and "actually true"

**AFTER** (correct):
- `check` only passes if fact is asserted
- Properly implements modal logic: existence ≠ truth
- Beliefs about propositions vs actual truth are distinguished

### Performance Considerations

- Negligible overhead for non-fact-tracked relations (early exit in should_include_row)
- For fact-tracked relations: One HashMap lookup per row during scan
- Comparable to subsumption filtering (which we already do)

## Summary

**API Layer**: ✅ COMPLETE
- Parameters threaded correctly
- Fact-tracked relations detected
- Check commands request filtering
- Regular rules unaffected

**Core Layer**: ⏳ TODO
- Need to implement actual row filtering
- Infrastructure exists (should_include_row)
- Choose approach (column vs scan-time)
- Integrate into query execution

**Estimated Effort**: 2-4 hours for scan-time approach, 4-8 hours for column approach

## References

- **Progress doc**: TRUTH_STATUS_FILTERING_PROGRESS.md
- **Solution design**: TRUTH_STATUS_FILTERING_SOLUTION.md
- **TODO details**: TRUTH_STATUS_FILTERING_TODO.md
- **Check status**: TRUTH_STATUS_CHECK_STATUS.md
- **Commits**: 394fac0, 85bbc3c

## Next Session

Start with reviewing this document and deciding on filtering approach (A or B).
Then implement the chosen approach in core-relations.
Should be straightforward now that all the plumbing is in place!
