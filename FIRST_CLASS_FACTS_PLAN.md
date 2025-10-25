# First-Class Facts Implementation Plan

## Overview
This plan implements first-class facts in egglog by:
1. Adding stable, unique IDs to tuples/rows in relation tables
2. Creating a `FactRef` BaseValue type that references these tuples
3. Enabling facts to be used as regular values in the egglog system

## Phase 1: Core Infrastructure

### 1.1 Define Fact Types
- [ ] **Create `FactId` type** in `core-relations/src/common.rs`
  - [ ] Define `FactId` as a numeric ID type using `define_id!` macro
  - [ ] Ensure `FactId` is stable across table operations (unlike `RowId`)

- [ ] **Create `FactRef` BaseValue** in `core-relations/src/base_values/mod.rs`
  - [ ] Define `FactRef` struct with `table_id: TableId` and `fact_id: FactId`
  - [ ] Implement `BaseValue` trait for `FactRef`
  - [ ] Consider `MAY_UNBOX` optimization if `FactRef` fits in 31 bits
  - [ ] Implement `Debug`, `Clone`, `Hash`, `Eq` traits

### 1.1a Truth Status System
- [ ] **Add Truth Status Column Support**
  - [ ] Extend `SchemaMath` in `egglog-bridge/src/lib.rs` to support truth status column
  - [ ] Add `asserted: bool` field to control whether table tracks truth status
  - [ ] Add `truth_col()` method to `SchemaMath` similar to `subsume_col()` and `proof_id_col()`
  - [ ] Update `table_columns()` to include truth column when enabled

### 1.2 Table Infrastructure Changes
- [ ] **Extend `SortedWritesTable`** in `core-relations/src/table/mod.rs`
  - [ ] Add `next_fact_id: FactId` field to track next available fact ID
  - [ ] Add `fact_id_map: HashMap<RowId, FactId>` to map internal rows to stable fact IDs
  - [ ] Add `fact_lookup: HashMap<FactId, RowId>` for reverse lookup
  - [ ] Add `truth_enabled: bool` field to enable/disable truth tracking
  - [ ] Update constructor to initialize fact tracking and truth status

- [ ] **Update Row Creation** in `SortedWritesTable`
  - [ ] Modify `add_row()` to assign and track fact IDs
  - [ ] Set default truth status (true for asserted facts, false for referenced-only facts)
  - [ ] Ensure fact IDs are preserved across table compactions/rehashing
  - [ ] Update parallel insertion logic to handle fact ID assignment and truth status

- [ ] **Update Table Compaction** in `SortedWritesTable`
  - [ ] Modify `rehash()` and `parallel_rehash()` to preserve fact ID mappings and truth status
  - [ ] Update `remove_stale()` logic to clean up fact ID mappings
  - [ ] Ensure fact IDs and truth status remain stable across all table operations

### 1.3 Table Interface Updates
- [ ] **Extend `Table` trait** in `core-relations/src/table_spec.rs`
  - [ ] Add `get_fact_id(&self, key: &[Value]) -> Option<FactId>` method
  - [ ] Add `get_row_by_fact_id(&self, fact_id: FactId) -> Option<Row>` method
  - [ ] Add `is_fact_asserted(&self, fact_id: FactId) -> Option<bool>` method
  - [ ] Add `assert_fact(&mut self, fact_id: FactId)` method
  - [ ] Add `retract_fact(&mut self, fact_id: FactId)` method
  - [ ] Update existing methods to handle truth status

- [ ] **Update `Row` struct** in `core-relations/src/table_spec.rs`
  - [ ] Add `fact_id: Option<FactId>` field to `Row`
  - [ ] Add `asserted: Option<bool>` field to `Row` (when truth tracking enabled)
  - [ ] Update all `Row` construction sites to include fact ID and truth status

## Phase 2: Integration with Value System

### 2.1 BaseValue Integration
- [ ] **Register FactRef in BaseValues** in `core-relations/src/lib.rs`
  - [ ] Export `FactRef` and `FactId` types
  - [ ] Ensure proper module organization

- [ ] **Integration Tests** in `core-relations/src/base_values/tests.rs`
  - [ ] Add roundtrip tests for `FactRef` values
  - [ ] Test `FactRef` interning and retrieval
  - [ ] Verify `FactRef` hashing and equality semantics

### 2.2 Database Integration
- [ ] **Update `Database`** in `core-relations/src/free_join/mod.rs`
  - [ ] Add methods to create and resolve `FactRef`s
  - [ ] Integrate fact resolution with table lookups
  - [ ] Add fact validation and error handling
  - [ ] Add truth status query methods

- [ ] **EGraph Integration** in `egglog-bridge/src/lib.rs`
  - [ ] Add `create_fact_ref(&mut self, table: TableId, key: &[Value], asserted: bool) -> Value`
  - [ ] Add `resolve_fact_ref(&self, fact_ref: Value) -> Option<Row>`
  - [ ] Add `is_fact_asserted(&self, fact_ref: Value) -> Option<bool>`
  - [ ] Add `assert_fact(&mut self, fact_ref: Value) -> Result<(), Error>`
  - [ ] Add `retract_fact(&mut self, fact_ref: Value) -> Result<(), Error>`
  - [ ] Register `FactRef` as a base value type
  - [ ] Update `TableAction` to support truth status operations

## Phase 3: Table Implementation Details

### 3.1 Fact ID Management
- [ ] **Implement Fact ID Assignment** in `SortedWritesTable`
  - [ ] Assign fact IDs in `serial_insert()` method
  - [ ] Assign fact IDs in `parallel_insert()` method
  - [ ] Handle fact ID assignment during merge operations
  - [ ] Ensure thread-safe fact ID generation

- [ ] **Handle Table Modifications**
  - [ ] Update fact mappings during row updates
  - [ ] Clean up fact mappings during row deletions
  - [ ] Handle fact ID preservation during table merges
  - [ ] Manage fact ID space efficiently (reuse deleted IDs?)

### 3.2 Lookup Operations
- [ ] **Implement Fact-Based Lookups**
  - [ ] Add efficient fact ID → row lookup
  - [ ] Add row key → fact ID lookup
  - [ ] Optimize lookup performance for common cases
  - [ ] Handle lookup errors gracefully

- [ ] **Update Query Operations**
  - [ ] Integrate fact lookups with query processing
  - [ ] Support fact references in query results
  - [ ] Handle fact resolution in joins and projections

## Phase 4: Advanced Features & Modal Logic Support

### 4.1 Modal Logic Operations
- [ ] **Truth vs Reference Distinction**
  - [ ] Add `query_asserted_facts(&self, table: TableId) -> Iterator<FactRef>` for truth queries
  - [ ] Add `query_all_facts(&self, table: TableId) -> Iterator<FactRef>` for existence queries
  - [ ] Support mixed queries (some facts asserted, others just referenced)
  - [ ] Add efficient indexes for truth status queries

- [ ] **Modal Operator Support**
  - [ ] Enable creation of unasserted fact references: `(fact-ref Color Sky Red false)`
  - [ ] Support belief/knowledge operators: `(Believes Alice fact-ref)`
  - [ ] Add assertion/retraction operations: `(assert fact-ref)`, `(retract fact-ref)`
  - [ ] Support counterfactuals and hypothetical reasoning

### 4.2 Error Handling and Validation
- [ ] **Fact Reference Validation**
  - [ ] Add `is_valid_fact_ref(&self, fact_ref: FactRef) -> bool`
  - [ ] Handle stale fact references gracefully
  - [ ] Provide clear error messages for invalid facts
  - [ ] Consider versioned fact IDs for additional safety

- [ ] **Truth Status Consistency**
  - [ ] Add debug assertions for truth status consistency
  - [ ] Validate truth status during table operations
  - [ ] Add tests for modal logic edge cases and error conditions
  - [ ] Ensure truth status is preserved across rebuilds

### 4.2 Performance Optimizations
- [ ] **Memory Layout Optimization**
  - [ ] Consider compact fact ID representation
  - [ ] Optimize fact mapping data structures
  - [ ] Benchmark fact lookup performance
  - [ ] Profile memory usage impact

- [ ] **FactRef Unboxing** (if applicable)
  - [ ] Implement `MAY_UNBOX = true` if `FactRef` fits in 31 bits
  - [ ] Implement `try_box()` and `try_unbox()` methods
  - [ ] Add unboxing tests and benchmarks
  - [ ] Measure performance impact of unboxing

## Phase 5: Testing and Documentation

### 5.1 Unit Tests
- [ ] **Core Functionality Tests**
  - [ ] Test fact ID assignment and stability
  - [ ] Test fact reference creation and resolution
  - [ ] Test table operations preserve fact IDs
  - [ ] Test concurrent access to fact mappings

- [ ] **Integration Tests**
  - [ ] Test fact references in queries
  - [ ] Test fact references across table operations
  - [ ] Test error handling and edge cases
  - [ ] Test performance with large fact sets

### 5.2 End-to-End Tests
- [ ] **EGraph Integration Tests** in `egglog-bridge/src/tests.rs`
  - [ ] Create facts and use them in rules
  - [ ] Test fact resolution in rule execution
  - [ ] Test fact references in external functions
  - [ ] Verify fact behavior in realistic scenarios

- [ ] **High-Level API Tests** in `src/` tests
  - [ ] Test fact creation from egglog syntax
  - [ ] Test fact usage in egglog programs
  - [ ] Test fact serialization and deserialization
  - [ ] Document fact usage patterns

### 5.3 Documentation
- [ ] **Code Documentation**
  - [ ] Document all new types and methods
  - [ ] Add usage examples for fact operations
  - [ ] Document performance characteristics
  - [ ] Document thread safety guarantees

- [ ] **User Documentation**
  - [ ] Add fact reference section to README
  - [ ] Create examples showing fact usage
  - [ ] Document limitations and caveats
  - [ ] Add migration guide for existing code

## Phase 6: Syntactic Sugar for Direct Fact References

### 6.1 Parser Extensions for Implicit Fact References
- [ ] **Extend Expression Parser** in `src/ast/mod.rs`
  - [ ] Add `ImplicitFactRef` expression type for nested expressions in modal contexts
  - [ ] Detect when expressions appear in "intensional" contexts (inside Believes, etc.)
  - [ ] Add parser rules to automatically wrap intensional expressions as fact references

- [ ] **Context-Aware Evaluation** in `src/lib.rs`
  - [ ] Add `IntensionalContext` tracking during rule/expression evaluation
  - [ ] When evaluating `(Believes Alice (Color Sky Red))`:
    - [ ] Detect `(Color Sky Red)` is in intensional context
    - [ ] Automatically create/lookup fact reference for `(Color Sky Red)`
    - [ ] Pass fact reference to `Believes` instead of trying to evaluate the expression
  - [ ] Handle nested intensional contexts properly

### 6.2 Intensional Context Management
- [ ] **Define Intensional Operators** 
  - [ ] Mark built-in operators like `Believes`, `Knows`, `Necessarily` as intensional
  - [ ] Allow user-defined functions to declare intensional arguments
  - [ ] Add syntax like `(function Believes (agent: Agent) (belief: intensional Fact) -> Bool)`

- [ ] **Automatic Fact Reference Creation**
  - [ ] When parsing `(Believes Alice (Color Sky Red))`:
    - [ ] Create unasserted fact reference for `(Color Sky Red)` if not exists
    - [ ] Rewrite to `(Believes Alice fact-ref-123)` internally
  - [ ] Ensure deterministic fact reference creation (same expression → same fact ref)
  - [ ] Handle complex nested expressions recursively

### 6.3 Pretty Printing and User Experience
- [ ] **Enhanced Display** in result formatting
  - [ ] When displaying results, show `(Color Sky Red)` instead of `fact-ref-123`
  - [ ] Add option to show internal fact references for debugging
  - [ ] Ensure round-trip: parse `(Believes Alice (Color Sky Red))` → display same

- [ ] **Error Messages**
  - [ ] Clear error messages when intensional/extensional context is misused
  - [ ] Helpful suggestions for modal logic usage patterns
  - [ ] Debug information about fact reference creation

## Phase 7: Cleanup and Polish

### 7.1 Code Review and Cleanup
- [ ] **Code Quality**
  - [ ] Review all implementations for correctness
  - [ ] Clean up debug prints and temporary code
  - [ ] Ensure consistent naming and style
  - [ ] Add missing error handling

- [ ] **Performance Validation**
  - [ ] Benchmark fact operations vs baseline
  - [ ] Profile memory usage impact
  - [ ] Optimize hot paths if needed
  - [ ] Validate scalability with large tables

### 7.2 Final Integration
- [ ] **Export New APIs** in `core-relations/src/lib.rs`
  - [ ] Export `FactId`, `FactRef` types
  - [ ] Export fact-related functions
  - [ ] Ensure proper visibility and documentation

- [ ] **Update Examples and Tests**
  - [ ] Update existing examples to show fact usage
  - [ ] Add new examples demonstrating modal logic with direct syntax
  - [ ] Ensure all tests pass
  - [ ] Update continuous integration

## Implementation Notes

### Key Design Decisions
1. **Stable IDs**: FactIds must survive table compaction, unlike RowIds
2. **BaseValue Choice**: Facts are immutable references, perfect for BaseValue
3. **Simple Lookup**: Direct HashMap for fact ID ↔ row ID mapping
4. **Truth Column**: Additional column (like subsume/proof_id) to track assertion status
5. **Modal Logic Support**: Truth vs existence distinction enables reasoning about unasserted propositions
6. **Thread Safety**: Use appropriate synchronization for concurrent access

### Performance Considerations
- Fact ID assignment should be fast (simple counter increment)
- Fact lookups should be O(1) via HashMap
- Truth status adds minimal overhead (one bool per row, only when enabled)
- Memory overhead should be minimal (one ID + optional bool per row)
- Consider unboxing optimization for small FactRefs

### Truth Status Design
- **Default Behavior**: Tables without truth tracking work as before (all facts implicitly asserted)
- **Truth-Enabled Tables**: Extra column tracks whether each fact is asserted or just referenced
- **Modal Logic**: Enables `(Color Sky Red)` to exist as referenceable fact without being true
- **Flexible**: Users can define their own modal operators (Believes, Knows, etc.) using fact references

### Testing Strategy
- Unit test each component in isolation
- Integration tests for cross-component interactions  
- Performance tests to ensure no regression
- End-to-end tests with realistic usage patterns
- Modal logic test suite covering intensional contexts
- Parser tests for automatic fact reference creation
- Round-trip tests for syntax sugar (parse → display → parse)

## Success Criteria
- [ ] Facts can be created for any tuple in any table
- [ ] Fact references work as regular Values in the system
- [ ] Facts can be referenced without being asserted (enabling modal logic)
- [ ] Truth status can be queried and modified independently
- [ ] Modal operators like `(Believes Alice fact-ref)` work naturally
- [ ] Fact resolution is fast and reliable
- [ ] Table operations preserve fact identity and truth status
- [ ] Implementation has minimal performance impact
- [ ] Code is well-documented and tested

## Example Usage Patterns

### Phase 1-5: Explicit Fact References
```egglog
; Create fact reference (asserted by default)
(let sky-blue-fact (fact-ref Color Sky Blue true))

; Create unasserted fact reference  
(let sky-red-fact (fact-ref Color Sky Red false))

; Use in modal contexts
(Believes Alice sky-red-fact)
(Knows Bob sky-blue-fact)
```

### Phase 6+: Direct Syntactic Sugar
```egglog
; Direct syntax - no variables needed!
(Believes Alice (Color Sky Red))    ; Automatically creates unasserted fact ref
(Knows Bob (Color Sky Blue))        ; Works even if Color Sky Blue is false
(Necessarily (= 2 (+ 1 1)))         ; Mathematical truths

; Mixed contexts work naturally
(rule ((Color ?obj ?color))          ; Extensional: requires asserted fact
      ((Believes Alice (Color ?obj ?color))))  ; Intensional: creates fact ref

; Nested modal contexts
(Believes Alice (Knows Bob (Color Sky Blue)))
; Creates fact refs for both (Knows Bob ...) and (Color Sky Blue)
```

### Truth Status Operations
```egglog
; Query what Alice believes (regardless of truth)
(rule ((Believes Alice ?fact)) ((Print "Alice believes" ?fact)))

; Only process asserted facts
(rule ((= ?fact (fact-ref ?pred ?args true))) 
      ((ProcessAssertion ?pred ?args)))

; Assert a previously unasserted fact
(assert (Color Sky Red))    ; Now (Color Sky Red) becomes true

; The same fact can be referenced and asserted independently
(Believes Alice (Color Sky Red))  ; Creates unasserted reference
(Color Sky Red)                   ; Asserts the fact as true
```

### Advanced Modal Logic Examples
```egglog
; Counterfactuals
(function CounterfactualIf (condition: intensional Fact) (consequence: intensional Fact) -> Fact)
(CounterfactualIf (Won Alice Lottery) (Rich Alice))

; Temporal logic
(function Eventually (fact: intensional Fact) -> Fact)
(Eventually (Color Sky Red))  ; It will eventually be true that sky is red

; Epistemic logic with multiple agents
(Believes Alice (Believes Bob (Color Sky Red)))
; Alice believes that Bob believes the sky is red
```