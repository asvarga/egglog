# Subsume Filtering Fix

## Problem

After implementing truth status filtering with explicit constraints in `build_cached_plan`, subsume filtering stopped working correctly. When `is_subsumed=Some(false)` was passed to queries, the subsume constant was added to the atom entries but was not being converted into an explicit constraint during query building.

## Root Cause

The truth tracking implementation introduced a pattern where metadata columns (truth status, subsume status) require explicit `EqConst` constraints in `build_cached_plan`. Previously, constants in atom entries were sufficient, but the new architecture requires converting these constants into separate constraints.

## Solution

Added code in `build_cached_plan` (egglog-bridge/src/rule.rs lines 1072-1080) to check if a constant value exists at the subsume column position and convert it to an explicit `Constraint::EqConst`:

```rust
// Add subsume status constraint if there's a constant value at the subsume column
if schema_info.subsume {
    let subsume_col_idx = schema_info.subsume_col();
    if let Some(QueryEntry::Const { val, .. }) = entries.get(subsume_col_idx) {
        constraints.push(Constraint::EqConst {
            col: ColumnId::from_usize(subsume_col_idx),
            val: *val,
        });
    }
}
```

This mirrors the existing truth status constraint logic and ensures that subsume filtering works correctly.

## Testing

Created `test_subsume_constraint.egg` which verifies:
1. Facts can be asserted and checked normally
2. After calling `(subsume (R (A)))`, queries for `(R (A))` return empty
3. `(check (R (A)))` fails after subsume (as expected)

## Status

- ✅ Subsume filtering now works correctly for simple cases
- ✅ Library tests pass (25/25)
- ✅ Truth filtering tests pass
- ✅ Integration tests: 237 pass, 4 fail
- ⚠️ The 4 failing tests (subsume_relation, subsume_relation_resugar, subsume, subsume_resugar) were already failing at commit 897c90c, BEFORE any truth tracking work began
- ✅ This fix prevents a regression and maintains parity with pre-truth-tracking behavior

## Pre-existing Issues

The 4 failing subsume tests involve complex scenarios with e-graph unions and subsumption. These failures existed before truth tracking work and are not regressions introduced by this work.

## Commit

Commit e2fe025: "Fix subsume filtering by adding explicit constraints in build_cached_plan"
