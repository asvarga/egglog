# Truth Status Filtering - Metadata Plumbing Complete

## Status: METADATA WIRED ✅ | QUERY FILTERING TODO ⏳

**Date**: 2025-11-09  
**Commits**: 72ae122, c2420b9

All metadata plumbing for truth status filtering is now complete. The `require_asserted` parameter flows correctly from high-level `check` commands all the way down to individual query atoms, where it's stored for use during query execution.

## What's Complete

### ✅ Full API and Metadata Chain

```
User Code: (check (R 1 2))
    ↓
check_facts() [src/lib.rs]
    require_asserted=true
    ↓
BackendRule::query() [src/lib.rs]
    Detects fact_tracking
    Sets require_asserted_for_query = Some(true)
    ↓
RuleBuilder::query_table() [egglog-bridge/src/rule.rs]
    Accepts require_asserted: Option<bool>
    ↓
add_atom_with_timestamp_and_func() [egglog-bridge/src/rule.rs]
    Accepts require_asserted parameter
    ↓
Query.atoms [egglog-bridge/src/rule.rs]
    Stores 4-tuple: (TableId, Vec<QueryEntry>, SchemaMath, Option<bool>)
    ↓
Query Execution
    ⚠️ TODO: Use require_asserted to filter rows
```

### ✅ Code Changes

#### Commit c2420b9: "Thread require_asserted through query atom metadata"

**Modified Files:**
1. `egglog-bridge/src/rule.rs`:
   - Line 127: Changed `atoms` type from 3-tuple to 4-tuple
   - Line 382: Added `require_asserted: Option<bool>` parameter to `add_atom_with_timestamp_and_func`
   - Line 436: Push 4-tuple with `require_asserted` to `Query.atoms`
   - Line 496: Pass `require_asserted` from `query_table` to atom creation
   - Lines 1038, 1067, 1085: Updated all atom destructuring patterns

2. `egglog-bridge/src/lib.rs`:
   - Lines 1226, 1232, 1269: Updated rebuild rule call sites to pass `None` for require_asserted

**Compilation Status**: ✅ Clean (only expected unused warnings)

## What Remains

### ⏳ Task 5: Implement Query Filtering

The `require_asserted` flag is now stored with each atom, but the query planner/executor doesn't use it yet. We need to implement the actual filtering.

#### Option A: Column-Based Filtering (Architecturally Better)

**Approach**: Add truth status as a table column (similar to subsumption column)

**Changes Required:**

1. **Extend SchemaMath** (egglog-bridge/src/lib.rs):
   ```rust
   struct SchemaMath {
       tracing: bool,
       subsume: bool,
       truth_tracking: bool,  // NEW
       func_cols: usize,
   }
   ```

2. **Add truth column methods**:
   ```rust
   impl SchemaMath {
       fn truth_col(&self) -> usize {
           assert!(self.truth_tracking);
           // After timestamp, proof (if tracing), subsume (if enabled)
           let mut col = self.func_cols + 1;
           if self.tracing { col += 1; }
           if self.subsume { col += 1; }
           col
       }
       
       fn table_columns(&self) -> usize {
           self.func_cols + 1 /* timestamp */
               + if self.tracing { 1 } else { 0 }
               + if self.subsume { 1 } else { 0 }
               + if self.truth_tracking { 1 } else { 0 }  // NEW
       }
   }
   ```

3. **Populate truth column during row insertion** (egglog-bridge/src/lib.rs):
   - In table insert operations, set truth column to ASSERTED (Value(1)) or REFERENCED (Value(0))
   - Update `write_table_row` to accept optional `truth` parameter

4. **Generate constraints during query building** (egglog-bridge/src/rule.rs):
   ```rust
   // In build_cached_plan or add_rules_from_cached
   for (table, entries, schema_info, require_asserted) in &self.atoms {
       let mut constraints = vec![];
       
       if let Some(true) = require_asserted {
           if schema_info.truth_tracking {
               let truth_col = ColumnId::from_usize(schema_info.truth_col());
               constraints.push(Constraint::EqConst {
                   col: truth_col,
                   val: ASSERTED,  // Only match asserted rows
               });
           }
       }
       
       add_atom(&mut qb, *table, entries, &constraints, &mut inner)?;
   }
   ```

**Pros**:
- Uses existing constraint infrastructure
- Consistent with subsumption approach
- Efficient (leverages indexed columns)

**Cons**:
- More invasive (schema changes)
- Requires updating table insertion logic
- Larger tables (extra column per row)

#### Option B: Scan-Time Filtering (Simpler)

**Approach**: Filter during `scan_generic_bounded` by calling `should_include_row()`

**Changes Required:**

1. **Add require_asserted to scan parameters** (core-relations/src/table_spec.rs):
   ```rust
   fn scan_generic_bounded(
       &self,
       subset: SubsetRef,
       start: Offset,
       n: usize,
       cs: &[Constraint],
       require_asserted: bool,  // NEW
       f: impl FnMut(RowId, &[Value]),
   ) -> Option<Offset>;
   ```

2. **Filter in SortedWritesTable** (core-relations/src/table/mod.rs):
   ```rust
   fn scan_generic_bounded(
       &self,
       subset: SubsetRef,
       start: Offset,
       n: usize,
       cs: &[Constraint],
       require_asserted: bool,
       mut f: impl FnMut(RowId, &[Value]),
   ) -> Option<Offset> {
       subset.iter_bounded(start.index(), start.index() + n, |row| {
           let Some(entry) = self.data.get_row(row) else { return; };
           
           // NEW: Check truth status if filtering enabled
           if !self.should_include_row(row, require_asserted) {
               return;  // Skip unasserted facts
           }
           
           if cs.is_empty() || self.get_if(cs, row).is_some() {
               f(row, entry);
           }
       })
       .map(Offset::from_usize)
   }
   ```

3. **Thread require_asserted through query execution**:
   - Pass from Query.atoms → QueryBuilder → scan calls
   - Update all scan call sites

**Pros**:
- Less invasive (no schema changes)
- Simpler implementation
- Leverages existing `should_include_row()` infrastructure

**Cons**:
- Doesn't use constraint system
- May be less efficient (extra check per row)
- Breaks existing scan interface

### Recommendation

**Start with Option B (Scan-Time)** for these reasons:
1. Simpler and faster to implement
2. Validates the overall approach
3. Can always refactor to Option A later
4. The infrastructure (`should_include_row`, `is_row_asserted`) already exists and is tested

## Testing Plan

Once filtering is implemented:

```egglog
(datatype Agent (Alice) (Bob))
(relation R (Agent Agent) :fact-tracking)

;; Create unasserted fact
(let f (fact R (Alice) (Bob)))
(run 1)

;; TEST 1: Check should FAIL for unasserted fact
(fail (check (R (Alice) (Bob))))

;; TEST 2: Assert the fact
(R (Alice) (Bob))
(run 1)
(check (R (Alice) (Bob)))  ;; Should PASS

;; TEST 3: Retract it
(retract-fact f)
(run 1)
(fail (check (R (Alice) (Bob))))  ;; Should FAIL again
```

## Files to Modify Next

### For Option B (Scan-Time):

1. **core-relations/src/table_spec.rs**:
   - Add `require_asserted` to `Table::scan_generic_bounded` signature
   - Update trait definition

2. **core-relations/src/table/mod.rs**:
   - Implement filtering in `SortedWritesTable::scan_generic_bounded`
   - Call `should_include_row(row, require_asserted)`

3. **core-relations/src/uf/mod.rs**:
   - Update UF table implementations (likely no filtering needed)

4. **core-relations/src/query.rs**:
   - Thread `require_asserted` from Query atoms to scan calls
   - Extract from atom tuple during query execution

5. **egglog-bridge/src/rule.rs**:
   - Pass truth tracking info to core-relations
   - May need to store FunctionInfo.fact_tracking somewhere accessible

## Performance Considerations

- **Non-fact-tracked relations**: Zero overhead (early return in `should_include_row`)
- **Fact-tracked relations**: One HashMap lookup per row during scan
- **Compared to subsumption**: Similar overhead (also one check per row)
- **Optimization opportunity**: Cache truth status in row metadata

## Summary

**Metadata Layer**: ✅ COMPLETE
- Parameter flows from check → query → atoms
- Stored correctly in atom tuples
- All call sites updated
- Code compiles cleanly

**Execution Layer**: ⏳ TODO
- Need to implement actual row filtering
- Choose between column-based (A) or scan-time (B)
- Option B recommended for initial implementation
- Infrastructure already exists and tested

**Estimated Effort**: 
- Option B: 2-3 hours
- Option A: 4-6 hours

## Next Steps

1. Implement Option B (scan-time filtering)
2. Test with minimal .egg file
3. If performance is acceptable, ship it
4. If not, refactor to Option A

## References

- **API completion**: TRUTH_STATUS_API_COMPLETE.md
- **Overall progress**: TRUTH_STATUS_FILTERING_PROGRESS.md
- **Solution design**: TRUTH_STATUS_FILTERING_SOLUTION.md
- **Commits**: 72ae122 (summary), c2420b9 (metadata plumbing)
