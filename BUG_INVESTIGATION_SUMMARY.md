# First-Class Facts: Reference-Then-Assert Bug - Investigation Summary

## Date: November 22, 2025

## Problem Statement
When a fact is created as REFERENCED (via `fact` expression), later asserting it directly doesn't update truth status to ASSERTED. Test `test_reference_then_assert.egg` fails.

## Root Cause Analysis

### Initial Hypothesis ✅ CONFIRMED
The truth column in row data was not being set correctly when creating unasserted facts. The `create_or_lookup_unasserted_fact` method was staging an insert with stale values for non-key columns, including the truth column.

### Fix Implemented
Modified `serial_insert` in `core-relations/src/table/mod.rs` (both sorted and unsorted paths) to detect pending unasserted facts and set their truth column to REFERENCED (0) before adding the row to the table.

```rust
// Before inserting, check if this is an unasserted fact
if self.truth_enabled {
    let key = &query[0..n_keys];
    if let Some(ref pending) = self.pending_unasserted_facts {
        if pending.contains_key(key) {
            // Make a copy and fix the truth column
            scratch_for_truth.extend_from_slice(query);
            scratch_for_truth[self.n_columns - 1] = Value::new_const(0); // REFERENCED
            row_to_insert = &scratch_for_truth;
        }
    }
}
```

### Current Status: PARTIAL FIX
Debug output confirms the truth column IS being set to REFERENCED (0) correctly. However, the test still fails.

### Secondary Issue Discovered: Value Canonicalization ⚠️
The test shows only 2 inserts being processed:
1. The unasserted fact creation via `(fact R (A) (B))`
2. The Holder relation insert

The third expected insert from `(R (A) (B))` assertion doesn't appear in the debug logs, suggesting one of:
1. The assertion isn't generating an insert
2. The insert is generated but keys don't match (Value ID mismatch)
3. The insert is being deduplicated before reaching serial_insert

Most likely cause: **Value canonicalization issue**. The Values used when creating the unasserted fact may differ from the Values used when asserting, causing key mismatch during lookup.

## Architecture Notes

### Two Representations of Truth Status
1. **Column Value** in row data - used for constraint-based filtering during queries
2. **HashMap** `fact_truth_status: HashMap<FactId, bool>` - authoritative source

Both must be kept in sync for the system to work correctly.

### Schema Layout
Columns: `[keys..., ret_val, timestamp, proof?, subsume?, truth?]`
- Truth column is LAST when `truth_enabled=true`
- Computed as: `func_cols + 1 (ts) + (tracing ? 1 : 0) + (subsume ? 1 : 0)`

## Files Modified
- `core-relations/src/table/mod.rs` - Added truth column fixup in `serial_insert`
- `egglog-bridge/src/lib.rs` - Added debug output (temporary)
- `src/lib.rs` - Added debug output in `check_facts` (temporary)

## Next Steps

### Option 1: Fix Value Canonicalization (Recommended)
Ensure Values are canonical when creating unasserted facts, or use fact_id-based lookup instead of key-based lookup for re-assertion.

### Option 2: Defer Row Creation
Don't create rows for unasserted facts immediately. Only allocate fact_id, then create row lazily when first needed.

### Option 3: Use Merge Function Properly
Investigate why the second insert isn't triggering a merge. May need to ensure both inserts reach the same merge cycle.

## Test Commands
```bash
# Main test (currently failing)
cargo run --release -- test_reference_then_assert.egg

# Working modal logic test  
cargo run --release -- test_modal_belief.egg

# Debug with output
cargo run --release -- test_reference_then_assert.egg 2>&1 | grep DEBUG
```

## Key Insight
The merge function and truth column infrastructure are CORRECT. The issue is earlier in the pipeline - either the second assertion isn't being processed, or it's not matching the existing row due to Value ID differences.
