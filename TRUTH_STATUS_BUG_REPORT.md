# Truth Status Filtering - Current Status

## Summary

Truth status filtering for `check` is **partially working** but has discovered a critical bug in the assertion transition logic.

## What Works ✅

1. **Unasserted fact creation**: When using `(fact KnowsAbout (Bob) (Charlie))` inside a `Believes` relation, the fact IS properly created as unasserted (`truth_status = false`)

2. **Check correctly rejects unasserted facts**: The `check` command DOES fail on unasserted facts as expected!

3. **Direct assertion**: Asserting a fact directly (without it existing as unasserted first) works fine

## Critical Bug Discovered ❌

**Problem**: When a fact exists as unasserted and you try to assert it explicitly, the assertion doesn't properly update the truth status.

**Test case that fails**:
```egglog
(relation KnowsAbout (Agent Agent) :fact-tracking)
(Believes (Alice) (fact KnowsAbout (Bob) (Alice)))  ;; Creates unasserted fact
(run 1)
(KnowsAbout (Bob) (Alice))  ;; Try to assert it
(run 1)
(check (KnowsAbout (Bob) (Alice)))  ;; FAILS but should PASS
```

**Test case that works**:
```egglog
(relation KnowsAbout (Agent Agent) :fact-tracking)
(KnowsAbout (Bob) (Alice))  ;; Assert directly (no unasserted fact first)
(run 1)
(check (KnowsAbout (Bob) (Alice)))  ;; PASSES correctly
```

## Root Cause Analysis

The issue is in how fact insertion handles existing rows:

1. `create_or_lookup_unasserted_fact()` creates a row with `truth_status = false`
2. When you later assert `(KnowsAbout (Bob) (Alice))`, the insertion logic sees that the row already exists
3. The system should UPDATE the truth_status from `false` to `true`, but it doesn't
4. The row remains unasserted, so `check` fails

## Required Fix

The insertion logic needs to be updated to handle the unasserted → asserted transition:

**Location**: `core-relations/src/table/mod.rs` - row insertion/update logic

**Required behavior**:
- When inserting a row that already exists as unasserted:
  - Keep the existing row
  - Update `fact_truth_status` HashMap: `truth_status[fact_id] = true`
  - Do NOT create a duplicate row

**Current behavior**:
- The insertion logic likely returns early when it finds an existing row
- It doesn't check if the row is unasserted and needs truth_status updated

## Implementation Plan

1. Find where rows are inserted (likely in `SortedWritesTable` insert methods)
2. Check if row exists AND has a fact_id (meaning it's fact-tracked)
3. If row exists as unasserted (`truth_status[fact_id] == false`):
   - Update to asserted: `self.fact_truth_status.insert(fact_id, true)`
4. Otherwise proceed with normal insertion logic

## Good News!

The truth status filtering mechanism is working correctly - `check` properly rejects unasserted facts. We just need to fix the assertion transition bug, and then the entire modal logic system will work as intended.

## Testing After Fix

```egglog
;; Should all pass after fix:
(relation KnowsAbout (Agent Agent) :fact-tracking)

;; Create unasserted
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
(run 1)
(fail (check (KnowsAbout (Bob) (Charlie))))  ;; Should fail - unasserted

;; Assert it
(KnowsAbout (Bob) (Charlie))
(run 1)
(check (KnowsAbout (Bob) (Charlie)))  ;; Should pass - now asserted
```
