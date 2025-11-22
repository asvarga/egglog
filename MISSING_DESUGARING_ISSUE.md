# Critical Finding: Missing Relation Desugaring

## Date: November 22, 2025

## Discovery
The bug in `test_reference_then_assert.egg` and `test_modal_belief.egg` is NOT primarily a truth tracking issue. It's a **missing desugaring step** for relation assertions.

## Root Cause
When you write:
```egglog
(R (A) (B))
```

This is being treated as `Action::Expr` (expression evaluation), NOT as a Set action.

**What should happen:**
```egglog
(R (A) (B))  →  (set (R (A) (B)) ())
```

**What actually happens:**
```egglog
(R (A) (B))  →  Let actions for evaluating the expression
```

## Evidence
```bash
# Debug output shows only Let actions, no Set actions:
[DEBUG actions] Action type: Let
[DEBUG actions] Action type: Let
[DEBUG actions] Action type: Let
```

## Workaround
Use explicit `set` syntax:
```egglog
(set (R (A) (B)) ())
```

## Test Verification
✅ `test_explicit_set.egg` - PASSES when using `(set (R (A) (B)) ())`
❌ `test_reference_then_assert.egg` - FAILS when using `(R (A) (B))`  
❌ `test_modal_belief.egg` - FAILS when using `(R (A) (B))`

## Impact
This affects ALL attempts to assert relations at the top level. The tests were written assuming this would work, but the desugaring wasn't implemented.

## Fix Needed
Add automatic desugaring in the parser/resolver so that when a relation (function returning Unit) is used as a top-level action, it's automatically converted to a Set action with a Unit value.

**Location:** Likely in `src/typechecking.rs` or the action resolution logic.

## Truth Column Fix Status
The truth column fix I implemented (setting truth=REFERENCED for unasserted facts) is CORRECT and NECESSARY. But it can't be fully tested until this desugaring issue is fixed.

## Next Steps
1. Implement automatic relation desugaring for top-level actions
2. OR document that relations must use explicit `set` syntax  
3. Update all test files to use correct syntax
