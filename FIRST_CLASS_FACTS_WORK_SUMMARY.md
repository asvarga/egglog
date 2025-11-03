# First-Class Facts Implementation Summary

## Branch Information
- **Branch**: `first-class-facts`
- **Forked from**: `main` (commit 16a3d0b)
- **Total commits**: 18 commits since fork

## Overview
This branch implements support for first-class facts in egglog - the ability to reference individual tuples/rows in relations as values, track their truth status, and build modal logic systems.

---

## ✅ GOOD WORK: Core Infrastructure (Commits 1-11)

These commits provide a solid foundation for first-class facts support:

### Phase 1: Foundation (Commits 1-5)
1. **01d82a1** - Add FactId and FactRef types for stable fact references
   - Added core types in `core-relations`: `FactId`, `FactRef` 
   - These uniquely identify specific tuples in relation tables
   - **Status**: ✅ Good - fundamental building block

2. **f4ad5bc** - Add truth status foundation for first-class facts
   - Added ability to mark facts as "asserted" (true) vs just "referenced"
   - Enables modal logic: distinguish between "proposition exists" and "proposition is true"
   - **Status**: ✅ Good - key differentiator for modal logic

3. **700cb82, dfb71f5, 43340d9** - Implement fact ID assignment
   - Serial insertion, parallel insertion, and persistence through compaction
   - Ensures FactIds remain stable across e-graph operations
   - **Status**: ✅ Good - necessary for stable references

### Phase 2-4: Integration (Commits 6-11)
4. **93ae1d7 through e5ec065** - Complete integration phases
   - BaseValue integration for FactRef (treat FactRef as a primitive value)
   - High-level API integration in EGraph
   - Table implementation details (tracking fact IDs, truth status)
   - Error handling & validation
   - Rust API testing
   - Language primitive integration
   - **Status**: ✅ Good - comprehensive API implementation

**Summary of Good Work:**
- Proper separation of concerns: FactRef type, truth tracking, table infrastructure
- Comprehensive test coverage of the Rust API
- Stable fact IDs that persist through table operations
- Clean integration with existing BaseValue system

---

## ⚠️ PROBLEMATIC WORK: Syntactic Sugar Attempts (Commits 12-18)

Starting with commit 12, the work went off track trying to implement syntactic sugar for automatic relation-to-fact-reference conversion.

### What Went Wrong

#### Commit 12: 545d2cb - "BREAKTHROUGH: Phase 4.5 Complete"
- Added primitives: `is-fact-asserted`, `assert-fact`, `retract-fact`, `fact-ref`
- These are reasonable utility primitives
- **Status**: ⚠️ Mixed - primitives are okay, but claims of "complete system" premature

#### Commit 13: c410ac1 - "ULTIMATE ACHIEVEMENT: Add syntactic sugar"
- Attempted to enable syntax: `(Believes (Alice) (fact (KnowsAbout (Bob) (Charlie))))`
- Added `fact` keyword support in parser
- **Status**: ⚠️ Incomplete - only works for specific hardcoded relations

#### Commit 14: 2d8cc4c - "Add syntactic sugar for target syntax"
- More hardcoded relation handling
- **Status**: ❌ Wrong approach - hardcoding is not scalable

#### Commits 15-16: Misguided Generalization
- **bcf5340** - "Remove hardcoded relations, add general primitives"
  - Added `fact-ref-from-string` primitive
  - **CRITICAL FLAW**: Creates FactRefs from string identifiers using **string length**:
    ```rust
    FactRef {
        table_id: TableId::from_usize(relation_name.as_str().len() % 100),
        fact_id: FactId::from_usize((relation_name.as_str().len() * 37) % 100),
    }
    ```
  - This means all relations with the same name length get the same FactRef!
  - **Status**: ❌ Fundamentally broken

- **5e749b8** - "Automatic relation-to-fact-reference conversion"
  - Modified type checker to automatically convert relation calls to FactRef
  - Changes in `src/constraint.rs` to treat relations as returning FactRef
  - Annotation phase converts `(KnowsAbout (Bob) (Charlie))` to `(fact-ref-from-string "KnowsAbout-2")`
  - **CRITICAL FLAW**: Only encodes relation name + arity, not actual argument values!
  - **Status**: ❌ Wrong architectural approach

#### Commit 17-18: Attempted Fix
- **e93f566** - "Fix fact-ref-from-string to use content hash"
  - Changed from string length to proper hash of string content
  - **STILL BROKEN**: Still only hashes the string "KnowsAbout-2", not the actual arguments `(Bob, Charlie)`
  - FactRefs don't point to actual relation tuples, just hash synthetic strings
  - **Status**: ❌ Fixed symptom, not root cause

### The Fundamental Problem

The automatic conversion approach is **architecturally flawed** because:

1. **Compile-time vs Runtime Mismatch**
   - Type checking/annotation happens at compile-time
   - FactRefs need to reference actual runtime data (tuples in tables)
   - Can't create proper FactRefs without table lookups

2. **Lost Information**
   - Annotation phase only has `format!("{}-{}", relation_name, arity)`
   - Actual argument values like `(Bob, Charlie)` are lost
   - Even with hashing, we're hashing "KnowsAbout-2" not "KnowsAbout-Bob-Charlie"

3. **No Table Lookups**
   - `fact-ref-from-string` just generates synthetic FactRef values
   - Doesn't actually look up tuples in relation tables
   - FactRefs don't correspond to real data

### What Should Have Been Done

FactRefs must be created through **runtime table operations**, not string manipulation:

```egglog
; CORRECT: Use explicit fact keyword that specifies the relation
(relation KnowsAbout (Person Person))
(relation Believes (Person FactRef))

; Create a FactRef WITHOUT asserting the fact as true
; Alice believes that Bob knows about Charlie, but this doesn't make it true!
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))

; The fact (KnowsAbout (Bob) (Charlie)) now EXISTS but is NOT asserted
; It has a FactRef that can be referenced, but truth status = false/unasserted

; Later, we might assert it as true:
(KnowsAbout (Bob) (Charlie))
; Now the same FactRef has truth status = true/asserted
```

The solution requires:
1. A `fact` primitive/keyword that takes a relation name and arguments
2. This creates an **unasserted** fact reference if the tuple doesn't exist
3. Returns a FactRef pointing to that specific row (whether asserted or not)
4. Table infrastructure to:
   - Support looking up fact IDs by tuple values
   - Track truth status separately from existence
   - Create unasserted facts when needed

**Key insight**: The relation name must be part of the fact reference syntax, so the system knows which table to look in. And crucially, **referencing a fact ≠ asserting a fact**.

---

## What Works Now

### ✅ Working Infrastructure (from commits 1-11)
- `FactRef` type in core-relations and BaseValue system
- Truth status tracking (asserted vs referenced)
- Stable fact ID assignment and persistence
- Rust API for creating/resolving FactRefs via `RustRuleContext`:
  - `create_fact_ref(&self, table: &str, key: &[Value]) -> Option<FactRef>`
  - `resolve_fact_ref(&self, table: &str, fact_ref: &FactRef) -> Option<Vec<Value>>`
  - `is_fact_asserted(&self, table: &str, fact_ref: &FactRef) -> Option<bool>`
  - `assert_fact(&mut self, table: &str, fact_ref: &FactRef) -> bool`
  - `retract_fact(&mut self, table: &str, fact_ref: &FactRef) -> bool`

### ❌ Broken/Incomplete (from commits 12-18)
- `fact-ref-from-string` primitive (generates synthetic FactRefs, not real ones)
- Automatic relation-to-FactRef conversion in type checker (loses argument information)
- No `create-fact-ref-N` primitives exposed to egglog language
- Test file `test_relation_auto_conversion_full.egg` is misleading/broken

---

## Recommendations

### Immediate Actions
1. **Revert commits 15-18** (bcf5340 through e93f566)
   - Remove `fact-ref-from-string` primitive
   - Remove automatic conversion in `src/constraint.rs`
   - Keep commits 1-11 which are solid

2. **Evaluate commits 12-14** 
   - Keep basic utility primitives (`is-fact-asserted`, etc.) if they work
   - Remove any syntactic sugar that relies on broken infrastructure

### Path Forward
1. **Implement proper `fact` keyword/primitive**
   - Syntax: `(fact RelationName arg1 arg2 ...)`
   - This should do actual table lookup in the named relation
   - Returns a FactRef pointing to the specific tuple
   - Need to handle case where tuple doesn't exist yet

2. **Document the intended usage**
   - Users explicitly create FactRefs via `(fact RelationName ...)`
   - The relation name makes it clear which table is being referenced

3. **Test with real modal logic examples**
   - Beliefs about beliefs: `(Believes (Alice) (fact Believes (Bob) some-fact))`
   - Knowledge tracking with truth status
   - Fact retraction and assertion

---

## Files Modified

### Core Implementation (Good)
- `core-relations/src/lib.rs` - FactRef/FactId types, truth tracking
- `core-relations/src/table/*.rs` - Table support for fact IDs
- `egglog-bridge/src/lib.rs` - EGraph methods for FactRef operations
- `src/prelude.rs` - RustRuleContext FactRef methods
- `src/sort/factref.rs` - FactRefSort implementation

### Problematic Changes (Should Review/Revert)
- `src/lib.rs` - Lines 325-400: `fact-ref-from-string` and related primitives
- `src/constraint.rs` - Lines 480-720: Automatic relation conversion logic
- `src/core.rs` - Lines 50-150: ResolvedCall changes for FactRef support

### Test Files
- `test_relation_auto_conversion_full.egg` - Should be removed or rewritten

---

## Lessons Learned

1. **Type systems have limits**: Trying to make relations automatically return FactRef was fighting the type system
2. **Compile-time vs runtime**: Can't create runtime references at compile-time without table lookups
3. **Explicit is better than implicit**: Users should explicitly create FactRefs rather than having "magic" conversion
4. **Test the right layer**: The Rust API tests (commits 1-11) were good; the egglog syntax tests (commits 12-18) tested broken abstractions

---

## Next Steps (After Cleanup)

Once we revert the problematic commits and are back on solid ground:

1. Design proper `create-fact-ref` primitives that do table lookups
2. Implement them using the existing Rust API infrastructure
3. Write tests using explicit FactRef creation
4. Document modal logic patterns with working examples
5. Consider whether any syntactic sugar is actually needed (probably not)
