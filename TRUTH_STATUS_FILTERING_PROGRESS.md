# Truth Status Filtering - Implementation Progress

## Current Status: PARAMETER THREADING COMPLETE ✓

The `require_asserted` parameter has been successfully threaded through the query_table() API, but the actual filtering logic is not yet implemented.

## What Was Completed (Commit 394fac0)

### ✅ API Signature Updates

1. **egglog-bridge/src/rule.rs**: Added `require_asserted: Option<bool>` parameter to `RuleBuilder::query_table()`
   ```rust
   pub fn query_table(
       &mut self,
       func: FunctionId,
       entries: &[QueryEntry],
       is_subsumed: Option<bool>,
       require_asserted: Option<bool>,  // NEW
   ) -> Result<AtomId>
   ```

2. **All call sites updated** to pass `None` for the new parameter:
   - src/lib.rs (BackendRule::query)
   - src/scheduler.rs
   - egglog-bridge/src/macros.rs (2 locations)
   - egglog-bridge/src/tests.rs (7 locations)

### ✅ Compilation Status

- Code compiles successfully with no errors
- Only warnings about unused code (expected for incomplete implementation)

## What Remains To Be Done

### 🔨 Task 1: Implement Actual Filtering Logic

The `require_asserted` parameter is currently acknowledged but not used:

```rust
// TODO: Implement truth status filtering
// For now, the require_asserted parameter is accepted but not yet used.
// Full implementation requires either:
// A) Adding truth status as a column (like subsumption) and filtering via constraint
// B) Modifying scan methods to call should_include_row() during iteration
// See TRUTH_STATUS_FILTERING_SOLUTION.md for details.
let _ = require_asserted; // Acknowledge parameter to avoid unused warning
```

**Two Implementation Approaches:**

#### Approach A: Truth Status as Column (Recommended in docs)
- Add truth status column to fact-tracked tables
- Use existing constraint mechanism (`Constraint::EqConst`)
- Similar to how subsumption works
- More invasive but architecturally correct

#### Approach B: Scan-Time Filtering (Simpler)
- Modify `scan_generic_bounded()` and related methods
- Call `should_include_row(row_id, require_asserted)` during iteration
- Less invasive but may have performance implications

### 🔨 Task 2: Update BackendRule::query()

Modify the `query()` method in `BackendRule` (src/lib.rs) to:
1. Accept a `require_asserted` parameter
2. Detect if the queried function has fact tracking enabled
3. Pass `require_asserted=true` to `query_table()` when appropriate

```rust
fn query(&mut self, query: &core::Query<ResolvedCall, ResolvedVar>, 
         include_subsumed: bool, 
         require_asserted: bool) {  // NEW PARAMETER
    for atom in &query.atoms {
        match &atom.head {
            ResolvedCall::Func(f) => {
                let func_info = &self.functions[&f.name];
                let require_asserted_for_this_query = 
                    if func_info.decl.fact_tracking && require_asserted {
                        Some(true)
                    } else {
                        None
                    };
                self.rb.query_table(f, &args, is_subsumed, require_asserted_for_this_query)?;
            }
            // ...
        }
    }
}
```

### 🔨 Task 3: Update check_facts()

Modify `check_facts()` in src/lib.rs to pass `require_asserted=true`:

```rust
fn check_facts(&mut self, span: &Span, facts: &[ResolvedFact]) -> Result<(), Error> {
    // ... existing setup code ...
    
    let mut translator = BackendRule::new(/* ... */);
    translator.query(&query, false, true);  // include_subsumed=false, require_asserted=true
    
    // ... rest of check logic ...
}
```

This ensures that `check` only passes for asserted facts in fact-tracked relations.

### 🔨 Task 4: Testing

Create test files and verify:
1. `check` fails for unasserted facts
2. `check` passes for asserted facts  
3. Regular queries still work (include all facts)
4. Non-fact-tracked relations are unaffected

```egglog
(relation KnowsAbout (Agent Agent) :fact-tracking)

;; Create unasserted fact
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
(run 1)

;; This should FAIL (unasserted)
(fail (check (KnowsAbout (Bob) (Charlie))))

;; Assert the fact
(KnowsAbout (Bob) (Charlie))
(run 1)

;; Now should PASS (asserted)
(check (KnowsAbout (Bob) (Charlie)))
```

## Architecture Notes

### Current Truth Status Infrastructure

**Already Implemented:**
- `SortedWritesTable::is_row_asserted(row_id)` - checks if row is asserted
- `Table::should_include_row(row_id, require_asserted)` - filtering method
- `fact_truth_status: HashMap<FactId, bool>` - tracks truth per fact
- Constants: `ASSERTED` and `REFERENCED` in egglog-bridge

**Unused but Available:**
- `SchemaMath::truth_col()` - column index calculation
- `SchemaMath::table_columns_with_truth()` - schema with truth column

### Why Not Complete Yet?

The filtering logic requires changes to either:
1. **Table schema** - add truth column and use constraints (Approach A)
2. **Scan methods** - add `require_asserted` parameter and filter during iteration (Approach B)

Both approaches require careful coordination across multiple layers (core-relations, egglog-bridge, egglog).

## Next Steps

1. **Decide on approach**: Column-based (A) vs Scan-time (B) filtering
2. **Implement filtering logic** in chosen layer
3. **Update BackendRule::query()** to pass require_asserted appropriately  
4. **Update check_facts()** to request assertion filtering
5. **Add comprehensive tests**

## References

- **Main docs**: TRUTH_STATUS_FILTERING_SOLUTION.md
- **TODO**: TRUTH_STATUS_FILTERING_TODO.md
- **Check status**: TRUTH_STATUS_CHECK_STATUS.md
- **Commit**: 394fac0 "Add require_asserted parameter to query_table signature"

## Priority

**MEDIUM**: Important for correct modal logic semantics, but:
- Fact creation bug needs to be fixed first (can't test without it)
- Clear path forward once implementation approach is chosen
- Not blocking other features
