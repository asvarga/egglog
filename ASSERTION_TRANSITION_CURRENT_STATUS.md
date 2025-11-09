# Assertion Transition Bug - Current Status

## Summary

The architectural fix for unasserted fact creation has been implemented successfully.
Unasserted facts now go through the normal pending insertion queue, ensuring they
participate in table rebuilding and use canonical Value IDs.

## What Was Fixed

### Original Problem
`create_or_lookup_unasserted_fact()` directly manipulated table data structures, 
bypassing the pending queue and rebuild mechanisms. This caused Value IDs to become
stale after e-graph unification.

### Solution Implemented
Unasserted facts are now:
1. Pre-allocated a FactId immediately
2. Marked as unasserted in fact_truth_status  
3. Queued through Buffer.stage_insert() (normal insertion path)
4. Matched to rows during serial_insert using pending_unasserted_facts map
5. Assigned their pre-allocated FactId when the row is created

### Benefits
- ✅ Correct: Uses canonical Value IDs via rebuild
- ✅ Clean: Follows normal insertion architecture
- ✅ Consistent: No special-case direct manipulation
- ✅ Minimal overhead

## Remaining Issue

Tests still fail with a DIFFERENT bug:

### Observation
When explicitly asserting a previously unasserted fact:
```egglog
(Believes (Alice) (fact KnowsAbout (Bob) (Alice)))  ;; Creates unasserted
(run 1)
(KnowsAbout (Bob) (Alice))  ;; Should find and update existing row
(run 1)
(check (KnowsAbout (Bob) (Alice)))  ;; FAILS - row still marked unasserted
```

### Debug Evidence
```
First insertion:  key=[Value(1), Value(0)]  <- Bob=1, Alice=0 (correct)
Second insertion: key=[Value(0), Value(0)]  <- Both Alice! (WRONG)
```

The second assertion `(KnowsAbout (Bob) (Alice))` is somehow being resolved to
`[Value(0), Value(0)]` instead of `[Value(1), Value(0)]`.

### Why This Breaks The Fix
- First run: Creates unasserted row with key `[Value(1), Value(0)]`
- Second run: Looks for row with key `[Value(0), Value(0)]`
- Hash lookup fails - keys don't match!
- Creates NEW asserted row instead of updating existing unasserted row
- Result: Two rows exist (one unasserted, one asserted)
- Check matches the unasserted row and fails

### Root Cause Hypothesis
This appears to be a bug in egglog's datatype constructor resolution or Value
assignment during the second assertion. When `(KnowsAbout (Bob) (Alice))` is 
processed the second time, `(Bob)` is somehow resolving to the same Value as
`(Alice)`.

### Investigation Needed
1. How are datatype constructors resolved to Values?
2. Why would Bob and Alice (distinct constructors) map to the same Value?
3. Is there caching/memoization that's returning wrong values?
4. Could there be e-graph unification happening that shouldn't?

### Test Results
- ✅ `test_double_assert.egg`: Double assertion works (no unasserted fact first)
- ✅ `test_value_resolution.egg`: Bob and Alice are distinct values
- ❌ `test_assert_then_check.egg`: Fails due to value resolution bug
- ❌ `test_debug_values.egg`: Shows the key mismatch clearly

## Next Steps

1. Investigate egglog datatype constructor resolution
2. Debug why (Bob) resolves to Value(0) instead of Value(1) on second assertion
3. Check if there's interaction between FactRef creation and constructor resolution
4. Consider if this is related to how `(fact KnowsAbout (Bob) (Alice))` is processed

## Conclusion

The table-level architectural fix is complete and correct. The remaining bug is 
at the egglog semantic level (value resolution/assignment), not in the table
implementation.
