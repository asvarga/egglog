# Modal Logic First-Class Facts - Status Report
**Date**: November 9, 2025  
**Branch**: first-class-facts  
**Status**: Core Feature Working, One Bug Remaining

## Original Goal ✅ ACHIEVED

Enable modal epistemic logic where you can say:
```egglog
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
```
**WITHOUT** making `(KnowsAbout (Bob) (Charlie))` necessarily true.

### This Now Works! 

The implementation successfully supports:
- **Referencing facts without asserting them** ✅
- **Distinguishing belief from truth** ✅
- **Nested fact references** ✅
- **First-class fact values (FactRef)** ✅

## Test Verification

### Working Example (`test_modal_belief.egg`)
```egglog
(datatype Person (Alice) (Bob) (Charlie))
(relation KnowsAbout (Person Person) :fact-tracking)
(relation Believes (Person FactRef) :fact-tracking)

; Alice believes Bob knows about Charlie (creates REFERENCED fact)
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))

; ✅ Belief exists
(check (Believes (Alice) (fact KnowsAbout (Bob) (Charlie))))

; ✅ KnowsAbout is NOT asserted (only referenced)
(fail (check (KnowsAbout (Bob) (Charlie))))
```

**Result**: Both checks pass! The modal logic works correctly.

## Known Bug 🐛

### Issue: Referenced → Asserted Transition Broken

When a fact is first created as REFERENCED (via `fact` expression), later asserting it directly does NOT update its truth status to ASSERTED.

**Test Case** (`test_reference_then_assert.egg`):
```egglog
; Step 1: Reference fact without asserting
(Holder (fact R (A) (B)))

; Step 2: Verify NOT asserted
(fail (check (R (A) (B))))  ; ✅ PASSES

; Step 3: Now assert it explicitly  
(R (A) (B))

; Step 4: Should be asserted now
(check (R (A) (B)))  ; ❌ FAILS - Bug here!
```

### Root Cause Analysis

**Location**: `egglog-bridge/src/lib.rs`, line 1724, `TableWrapper::insert()`

```rust
pub fn insert(&mut self, state: &mut ExecutionState, row: impl Iterator<Item = Value>) {
    // ... 
    self.table_math.write_table_row(
        &mut self.scratch,
        RowVals {
            timestamp: ts,
            proof: None,
            subsume: self.table_math.subsume.then_some(NOT_SUBSUMED),
            truth: self.table_math.truth_tracking.then_some(ASSERTED), // ← Always ASSERTED
            ret_val: None,
        },
    );
    state.stage_insert(self.table, &self.scratch);  // ← Doesn't update existing rows
}
```

**The Problem**: 
1. `insert()` always creates rows with `truth: ASSERTED`
2. `stage_insert()` likely ignores duplicate keys (doesn't update existing rows)
3. When a row already exists with `truth: REFERENCED`, calling `insert()` doesn't update it

**Expected Behavior**:
- If row doesn't exist → insert with ASSERTED ✅
- If row exists with REFERENCED → UPDATE to ASSERTED ❌ (not implemented)
- If row exists with ASSERTED → no change needed ✅

## Implementation Summary

### What's Complete ✅

1. **Truth Status Column Infrastructure**
   - Column added to fact-tracked tables
   - ASSERTED (1) vs REFERENCED (0) values
   - Column layout: `[keys, return, timestamp, proof?, subsume?, truth?]`

2. **Truth Status Filtering**
   - `(check ...)` filters for ASSERTED facts only
   - Constraint-based filtering in query planner
   - Works correctly for querying

3. **Fact Creation Mechanisms**
   - `(fact R x y)` creates REFERENCED fact ✅
   - Direct `(R x y)` creates ASSERTED fact ✅
   - FactRef values work in all contexts ✅

4. **API Support**
   - `create_unasserted_fact_ref()` - create REFERENCED facts
   - `create_fact_ref()` - lookup existing facts
   - FactRef is a first-class base value type

5. **Syntax**
   - `:fact-tracking` flag on relations
   - `fact` primitive for creating references
   - `FactRef` sort available in type system

### What's Broken ❌

1. **Truth Status Updates**
   - Cannot transition fact from REFERENCED → ASSERTED
   - Needs update logic in insert/stage_insert

## File Architecture

### Key Files

**Core Implementation**:
- `egglog-bridge/src/lib.rs` - TableWrapper, insert/subsume/create_fact_ref methods
- `egglog-bridge/src/rule.rs` - Query building, constraint generation
- `src/lib.rs` - FactRefPrimitive, EGraph integration
- `src/constraint.rs` - Type checking for `fact` expressions

**Configuration**:
- `egglog-bridge/src/lib.rs` - SchemaMath struct (truth_tracking flag)
- Schema column calculations (truth_col, subsume_col, etc.)

**Documentation**:
- `TRUTH_STATUS_FILTERING_COMPLETE.md` - Truth filtering implementation
- `FIRST_CLASS_FACTS_PROGRESS.md` - Overall progress report
- `SUBSUME_FILTERING_FIX.md` - Subsume constraint fix
- `SESSION_SUMMARY.md` - Recent session summary

### Test Files

**Working Tests**:
- `test_modal_belief.egg` - Main modal logic test (shows bug at end)
- `test_reference_then_assert.egg` - Isolated bug reproduction
- `comprehensive_truth_test.egg` - Truth filtering tests
- `test_subsume_constraint.egg` - Subsume filtering tests

**Related Tests**:
- `minimal_unasserted_demo.egg`
- `test_unasserted_fact_creation.egg`
- `test_nested_facts.egg`
- `test_fact_as_argument.egg`

## Test Results

### Current Status
- **Library tests**: 25/25 ✅
- **Integration tests**: 237/241 ✅ (98.3%)
- **4 failures**: Pre-existing subsume tests (unrelated to truth tracking)
- **Truth filtering**: All passing ✅
- **Modal logic**: Partially working (reference works, re-assertion doesn't)

## Fix Strategy

### Recommended Approach

**Option 1: Update-or-Insert Semantics** (Recommended)
1. In `TableWrapper::insert()`, check if row exists
2. If exists and has REFERENCED status, UPDATE truth column to ASSERTED
3. If exists and already ASSERTED, no-op
4. If doesn't exist, INSERT with ASSERTED

**Implementation Sketch**:
```rust
pub fn insert(&mut self, state: &mut ExecutionState, row: impl Iterator<Item = Value>) {
    self.scratch.clear();
    self.scratch.extend(row);
    
    if self.table_math.truth_tracking {
        // Check if row exists
        if let Some(existing_row) = state.get_table(self.table).get_row(&self.scratch) {
            let current_truth = existing_row.vals[self.table_math.truth_col()];
            if current_truth == REFERENCED {
                // UPDATE: transition from REFERENCED to ASSERTED
                // Need to update the truth column value
                // This may require new API in core-relations
            }
            return; // Don't re-insert
        }
    }
    
    // Normal insert path (doesn't exist yet)
    let ts = Value::from_usize(state.read_counter(self.timestamp));
    self.table_math.write_table_row(/* ... */);
    state.stage_insert(self.table, &self.scratch);
}
```

**Option 2: Merge Function Approach**
- Define a merge function for truth status: `max(v1, v2)` (ASSERTED wins)
- Let core-relations handle updates automatically
- Simpler but may need core-relations support

**Option 3: Explicit Assert API**
- Add `(assert-fact (fact R x y))` command
- Keep insert semantics unchanged
- More explicit but requires user awareness

### Places to Modify

1. **`egglog-bridge/src/lib.rs`**: 
   - `TableWrapper::insert()` method (line 1724)
   - May need helper to check existing truth status

2. **`core-relations/src/`**:
   - May need to add update API if not available
   - Check `stage_insert` vs `stage_update` semantics

3. **Tests**:
   - Add test for REFERENCED → ASSERTED transition
   - Verify idempotence (ASSERTED → ASSERTED is no-op)

## Usage Guide

### Current Working Syntax

```egglog
; Declare fact-tracked relations
(relation KnowsAbout (Person Person) :fact-tracking)
(relation Believes (Person FactRef) :fact-tracking)

; Create referenced fact (NOT asserted)
(Holder (fact KnowsAbout (Bob) (Charlie)))

; Create asserted fact (directly)
(KnowsAbout (Alice) (Bob))

; Check only returns asserted facts
(check (KnowsAbout (Alice) (Bob)))      ; ✅ passes
(fail (check (KnowsAbout (Bob) (Charlie)))) ; ✅ passes (not asserted)

; Beliefs about facts
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
(check (Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))) ; ✅ passes
```

### Known Limitations

1. **Cannot re-assert referenced facts** (the bug)
2. No explicit `(assert-fact ...)` or `(retract-fact ...)` commands yet
3. No query syntax for "all facts" (asserted + referenced)
4. Cannot query by truth status explicitly

## Context for Next Session

### What to Know

1. **Core feature works**: Modal logic reference without assertion is functional
2. **One specific bug**: Truth status doesn't update on re-insertion
3. **No regressions**: All previous tests still pass (237/241)
4. **Pre-existing issues**: 4 subsume tests were already broken before this work

### Quick Start Commands

```bash
# Test modal logic (shows bug at end)
cargo run --release -- test_modal_belief.egg

# Isolated bug reproduction
cargo run --release -- test_reference_then_assert.egg

# Verify truth filtering still works
cargo run --release -- comprehensive_truth_test.egg

# Run all tests
cargo test --release --lib        # Library tests
cargo test --release --test files # Integration tests
```

### Debugging Tips

1. **Check truth column values**: Add debug output in `TableWrapper::insert()`
2. **Trace stage_insert**: See if it updates or ignores existing rows
3. **Check merge semantics**: Look for merge function definitions in core-relations
4. **Verify row lookup**: Use `state.get_table(table).get_row(key)` to inspect existing rows

## Commits in This Session

1. **e2fe025**: Fix subsume filtering by adding explicit constraints
2. **95f9667**: Add subsume filtering fix documentation and update progress report

## Summary

**The good news**: Your original goal is achieved! You can now express modal logic with facts that exist but aren't asserted as true.

**The remaining work**: Fix the truth status update logic so re-asserting a referenced fact properly marks it as asserted. This is a localized fix in the insert logic, not a fundamental architectural issue.

**Recommendation**: Start next session by implementing the update-or-insert semantics in `TableWrapper::insert()`. The test cases are already written and will confirm when the fix works.
