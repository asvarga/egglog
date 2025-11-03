# (fact RelationName ...) Syntax Implementation Status

## ✅ Completed Components

### 1. Parser & Macro System
- **File**: `src/lib.rs` (lines ~458-473)
- **Status**: ✅ COMPLETE
- Implemented `SimpleMacro` for `fact` keyword
- Transforms `(fact RelationName arg1 arg2 ...)` → `(fact-ref "RelationName" arg1 arg2 ...)`
- Properly extracts relation name and arguments
- Returns `Call` expression with "fact-ref" primitive

### 2. Type Checking
- **File**: `src/constraint.rs` (lines ~871-960)
- **Status**: ✅ COMPLETE
- Special handling for "fact-ref" primitive
- Validates relation exists and is accessible
- Type checks arguments against relation schema
- Returns `FactRef` sort as output type
- Error messages include proper span information

### 3. Core Resolution
- **File**: `src/core.rs` (lines ~83-102)
- **Status**: ✅ COMPLETE
- Modified `from_resolution` to handle fact-ref calls
- Looks up relation by name at resolution time
- Creates `ResolvedCall::Primitive` with `FactRefPrimitive`
- Preserves string literal for relation name

### 4. FactRefPrimitive Architecture
- **File**: `src/lib.rs` (lines ~285-330)
- **Status**: ✅ COMPLETE  
- Created `FactRefPrimitive` struct with helper function delegation
- Registered as a reserved primitive in `type_info`
- Uses external helper function to access EGraph internals
- Proper lifecycle management with Arc<Mutex<Option<ExternalFunctionId>>>

### 5. Helper Function Implementation
- **File**: `src/lib.rs` (lines ~373-440)
- **Status**: ⚠️ PARTIAL - Lookup works, creation TODO
- **What Works**:
  * Extracts relation name from first argument (as Boxed<String>)
  * Looks up TableId from table_id_map
  * Checks if row exists in table via `get_row`
  * Returns FactRef if row has assigned fact ID
- **What's TODO**:
  * Creating new unasserted facts (currently returns None)
  * Requires architectural changes for fact ID allocation during rule execution

### 6. Table ID Mapping
- **File**: `src/lib.rs` (line ~319, declare_function)
- **Status**: ✅ COMPLETE
- Added `table_id_map` field to `EGraph` struct
- Populated during function declaration
- Shared with helper via Arc<Mutex<IndexMap>>
- Added `get_table_id()` method to egglog-bridge::EGraph

## ⚠️ Current Limitations

### 1. Fact Tracking Must Be Enabled
**Problem**: Regular relations don't have fact tracking enabled by default.

**Impact**: The helper function returns None because:
- `table.get_fact_id_for_row(row_id)` returns None
- Tables created via `(relation ...)` have `truth_enabled = false`
- Need `enable_truth_tracking()` called on table

**Solutions**:
- [ ] Add egglog syntax: `(relation Edge (i64 i64) :fact-tracking)`
- [ ] Enable fact tracking by default for all relations
- [ ] Add command: `(enable-fact-tracking Edge)`

### 2. Cannot Create New Unasserted Facts
**Problem**: Architectural constraint - fact IDs are allocated during table merge, but we're executing during rule application.

**Current Behavior**: Returns None if fact doesn't exist

**Possible Solutions**:
1. **Eager Allocation**: Add a fact ID counter (like other counters) and allocate IDs immediately
   - Pros: Simple, works with current architecture
   - Cons: May waste IDs if facts are never asserted

2. **Predicted Values**: Use ExecutionState's predicted values mechanism
   - Pros: Consistent with existing patterns
   - Cons: Complex, may not fit fact reference semantics

3. **Staged Creation**: Stage fact creation and return "promise" that resolves after merge
   - Pros: Most correct semantically
   - Cons: Most complex, breaks immediate availability

4. **ExecutionState Method**: Add `create_unasserted_fact_ref` to ExecutionState
   - Pros: Clean API, proper encapsulation
   - Cons: Requires changes to core-relations

### 3. Database::create_unasserted_fact_ref Incomplete
**File**: `core-relations/src/free_join/mod.rs` (line ~815)  
**Status**: Stub implementation

The method exists but currently returns None. It needs to:
1. Look up or create the row for the given key
2. Assign a stable FactId (if row doesn't have one)
3. Mark fact as "referenced" but not "asserted"
4. Return FactRef { table_id, fact_id }

## 📋 TODO List (Priority Order)

### High Priority
1. **Enable Fact Tracking**
   - [ ] Add `:fact-tracking` flag to relation syntax
   - [ ] Implement flag parsing in relation declaration
   - [ ] Call `enable_truth_tracking()` during table creation

2. **Test Current Implementation**
   - [ ] Create Rust-level test with fact tracking manually enabled
   - [ ] Verify lookup works for existing facts with IDs
   - [ ] Document expected behavior

### Medium Priority
3. **Implement Fact Creation**
   - [ ] Choose architecture (eager allocation recommended)
   - [ ] Add fact ID counter to EGraph/Database
   - [ ] Implement `create_unasserted_fact_ref` in Database
   - [ ] Bridge from ExecutionState to Database method
   - [ ] Update helper function to create facts

4. **Add Fact Operations**
   - [ ] Implement `(assert-fact ref)` primitive
   - [ ] Implement `(retract-fact ref)` primitive
   - [ ] Implement `(is-fact-asserted ref)` primitive
   - [ ] Add tests for modal logic operations

### Low Priority
5. **Documentation & Examples**
   - [ ] Write comprehensive user documentation
   - [ ] Create modal logic examples
   - [ ] Add tutorial for first-class facts
   - [ ] Document performance characteristics

6. **Optimization**
   - [ ] Cache table ID lookups
   - [ ] Optimize fact ID allocation
   - [ ] Add benchmarks for fact operations

## 🧪 Testing Strategy

### Unit Tests Needed
- [ ] Parser: `(fact Edge 1 2)` transforms correctly
- [ ] Type checking: Validates relation schema
- [ ] Type checking: Error on unknown relation
- [ ] Type checking: Error on wrong argument types
- [ ] Fact lookup: Works with tracking enabled
- [ ] Fact creation: Creates unasserted facts

### Integration Tests Needed  
- [ ] Modal logic: Beliefs about facts
- [ ] Nested facts: Facts about facts
- [ ] Assertion/retraction: Truth status changes
- [ ] Multiple references: Same fact, different refs

### Performance Tests Needed
- [ ] Fact creation overhead
- [ ] Lookup performance vs direct relation access
- [ ] Memory usage for fact tracking

## 🎯 Next Steps

**Immediate** (to unblock further work):
1. Implement `:fact-tracking` flag for relations
2. Test current lookup implementation with tracking enabled

**Short term** (for basic functionality):
3. Implement eager fact ID allocation
4. Complete `create_unasserted_fact_ref` implementation

**Medium term** (for full modal logic):
5. Add assert/retract/query operations
6. Create comprehensive examples and tests

## 📊 Progress Summary

| Component | Status | Notes |
|-----------|--------|-------|
| Parser/Macro | ✅ 100% | Complete |
| Type Checking | ✅ 100% | Complete |
| Core Resolution | ✅ 100% | Complete |
| Primitive Registration | ✅ 100% | Complete |
| Helper Function | ⚠️ 60% | Lookup works, creation TODO |
| Fact Tracking | ❌ 0% | Needs egglog syntax |
| Fact Creation | ❌ 0% | Architectural work needed |
| Modal Operations | ❌ 0% | Depends on above |
| Documentation | ⚠️ 20% | Status doc only |
| Tests | ⚠️ 10% | Test file created |

**Overall Progress**: ~50% complete

The foundation is solid. The remaining work is primarily about:
1. Enabling fact tracking (small, targeted addition)
2. Implementing fact creation (medium complexity, architectural decision needed)
3. Adding modal logic operations (straightforward once creation works)
