# Truth Status Checking for `check` Command - Implementation Status

## Overview

The `check` command should verify that facts are not only present in the database, but also ASSERTED (truth_status=true) for fact-tracked relations. This is critical for modal logic where we distinguish between "fact exists" and "fact is asserted."

## Current Status: PARTIALLY IMPLEMENTED ⚠️

### What Works ✅

1. **Detection of Fact-Tracked Relations**
   - `check_facts()` loops through each fact
   - Correctly extracts function name from `ResolvedFact` (GenericFact enum)
   - Properly accesses `function.decl.fact_tracking` to check if tracking is enabled

2. **Existence Checking**
   - For fact-tracked relations, runs a query to verify the fact exists
   - If fact doesn't exist, `check` correctly fails

3. **Infrastructure in Place**
   - `should_include_row()` method in SortedWritesTable (core-relations/src/table/mod.rs:643)
   - `fact_truth_status` HashMap tracks truth status per row
   - `is_row_asserted()` checks if a specific row is asserted

### What's Missing ❌

**Truth Status Verification**: The query execution doesn't yet filter by truth status, so `check` will pass even for unasserted facts.

**Example of Current Incorrect Behavior:**
```egglog
(relation R (i64 i64) :fact-tracking)

;; Create unasserted fact (when that feature works)
(let f (fact R 1 2))  ;; Creates fact but doesn't assert it

;; This check should FAIL (fact exists but is unasserted)
;; But currently PASSES because we only check existence
(check (R 1 2))
```

## Why It's Hard

The query execution architecture is complex:
- Queries go through `BackendRule::query()` → `query_table()` → `add_atom_with_timestamp_and_func()`
- Query execution happens in core-relations with free-join planning
- Truth status filtering needs to be integrated at the core-relations level

Simply checking "if query matched" doesn't tell us WHICH specific row matched, so we can't check its truth status post-query.

## Implementation Plan

To properly implement truth status checking, we need to modify the query execution layer:

### Step 1: Add Truth Status Parameter to Query Table
**File**: `egglog-bridge/src/rule.rs`
**Method**: `query_table()` (line 469)

Change signature from:
```rust
pub fn query_table(
    &mut self,
    func: FunctionId,
    entries: &[QueryEntry],
    is_subsumed: Option<bool>,
) -> Result<AtomId>
```

To:
```rust
pub fn query_table(
    &mut self,
    func: FunctionId,
    entries: &[QueryEntry],
    is_subsumed: Option<bool>,
    require_asserted: Option<bool>,  // NEW PARAMETER
) -> Result<AtomId>
```

### Step 2: Pass Through to Internal Query Mechanism
**File**: `egglog-bridge/src/rule.rs`
**Method**: `add_atom_with_timestamp_and_func()` (line 382)

Add parameter and store in schema/metadata so it can be used during query execution.

### Step 3: Integrate with Query Execution
**File**: `core-relations/src/free_join/execute.rs`

During query execution, when fetching rows from a table, call `should_include_row()` with the `require_asserted` parameter.

This is similar to how subsumption filtering works but for truth status.

### Step 4: Update BackendRule Translator
**File**: `src/lib.rs`
**Method**: `BackendRule::query()` (line 1856)

Pass `require_asserted` parameter when calling `query_table()`:
```rust
fn query(&mut self, query: &core::Query<ResolvedCall, ResolvedVar>, include_subsumed: bool, require_asserted: bool) {
    for atom in &query.atoms {
        match &atom.head {
            ResolvedCall::Func(f) => {
                let f = self.func(f);
                let args = self.args(&atom.args);
                let is_subsumed = match include_subsumed {
                    true => None,
                    false => Some(false),
                };
                // Check if this function has fact tracking
                let require_asserted = if self.functions[&func_type.name].decl.fact_tracking && require_asserted {
                    Some(true)
                } else {
                    None
                };
                self.rb.query_table(f, &args, is_subsumed, require_asserted).unwrap();
            }
            // ...
        }
    }
}
```

### Step 5: Update check_facts() to Use New Parameter
**File**: `src/lib.rs`
**Method**: `check_facts()` (line 1077)

Remove the per-fact loop and just pass `require_asserted=true` to the translator:
```rust
fn check_facts(&mut self, span: &Span, facts: &[ResolvedFact]) -> Result<(), Error> {
    // Build the query with all facts
    let rule = ast::ResolvedRule { ... };
    let core_rule = rule.to_canonicalized_core_rule(...)?;
    let query = core_rule.body;
    
    // Create translator with truth status checking enabled
    let mut translator = BackendRule::new(...);
    translator.query(&query, false, true);  // include_subsumed=false, require_asserted=true
    // ... rest of check logic
}
```

## Alternative Simpler Approach

If modifying the query execution is too complex, we could:

1. Extract the actual values from the fact arguments
2. Look up the row in the table directly
3. Check its truth status using `is_row_asserted()`

This would bypass the query system but would be more fragile and not scale as well.

## Current Code Location

The partial implementation is in:
- **File**: `src/lib.rs`
- **Method**: `check_facts()` starting at line 1063
- **Commit**: 48526bf "Add partial truth status checking for check command"

## Testing

Once implemented, test with:
```egglog
(relation R (i64 i64) :fact-tracking)

;; Assert a fact
(R 1 2)
(check (R 1 2))  ;; Should pass

;; Create unasserted fact (once that works)
(let f (fact R 3 4))
(fail (check (R 3 4)))  ;; Should fail - fact exists but unasserted

;; Assert it
(assert-fact f)
(check (R 3 4))  ;; Now should pass
```

## Priority

**Medium-High**: This is important for correct modal logic semantics, but:
- The fact creation feature needs to work first (can't test unasserted facts without it)
- Not blocking other development
- Clear path forward once we have time to implement it

## Related Issues

- Fact creation (CONTINUATION_PROMPT.md) - needs to work first to test this
- Query execution architecture - complex but well-designed
- Subsumption filtering - good model for how to add this feature
