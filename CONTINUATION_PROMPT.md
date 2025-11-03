# Continuation Prompt for First-Class Facts Implementation

Copy and paste this prompt to continue work in a new conversation:

---

I'm working on the **first-class facts** feature in the egglog project. This enables modal epistemic logic - beliefs and knowledge about propositions.

## Branch & Status
- **Branch**: `first-class-facts` (23 commits ahead of main)
- **Repository**: `/Users/avarga/Documents/egglog`
- **Overall Progress**: ~85% complete, core infrastructure done

## What's Implemented ✅

The syntax `(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))` is **fully working** with one limitation:

1. **FactRef Type System** - Complete
   - FactRef is a proper sort with BaseValue integration
   - Can be used as relation argument type
   
2. **Fact Tracking** - Complete
   - `:fact-tracking` flag enables truth tracking per relation
   - Separates "fact exists" from "fact is asserted"

3. **Syntax & Type Checking** - Complete
   - `(fact RelationName args)` macro transforms to `(fact-ref "RelationName" args)`
   - Full type checking against relation schema
   - Runtime table lookup by name

4. **Nested Facts** - Working
   - Beliefs about beliefs work perfectly
   - Tested up to 3 levels: `(Believes (Bob) (fact Believes (Alice) (fact KnowsAbout ...)))`

5. **Fact Lookup** - Working
   - Can look up existing facts and get their FactRef
   - FactRef stored as (TableId, FactId) pairs

## Current Limitation ⚠️

**Facts must exist before they can be referenced.**

### Current Workaround:
```egglog
;; Must assert the fact first
(KnowsAbout (Bob) (Charlie))

;; Then can reference it
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
```

### What Should Work:
```egglog
;; Should create (KnowsAbout (Bob) (Charlie)) as UNASSERTED fact
;; and return its FactRef - but currently returns None
(Believes (Alice) (fact KnowsAbout (Bob) (Charlie)))
```

## Next Task 🎯

**Implement creation of unasserted facts.**

**Problem**: Fact IDs currently allocated during table merge, but we need them during rule execution.

**Recommended Solution**: Eager fact ID allocation
- Add fact ID counter to EGraph (like timestamp counter)
- Allocate IDs immediately during rule execution  
- Mark new facts as "referenced but not asserted"

**Files to Modify**:
1. `egglog-bridge/src/lib.rs` - Add `next_fact_id` counter
2. `core-relations/src/free_join/mod.rs` - Implement `create_unasserted_fact_ref`
3. `src/lib.rs` - Update helper function (around line 373-440) to call create method

## After Fact Creation Works

Implement modal operations (straightforward):
- `(assert-fact ref)` - Mark fact as true
- `(retract-fact ref)` - Mark fact as not asserted  
- `(is-fact-asserted ref)` - Query truth status

These can use existing Rust API in `RustRuleContext`.

## Documentation

Read these files for context:
- **FIRST_CLASS_FACTS_WORK_SUMMARY.md** - Comprehensive status (just updated)
- **FACT_SYNTAX_IMPLEMENTATION_STATUS.md** - Detailed component status
- **FIRST_CLASS_FACTS_PLAN.md** - Original plan and architecture

## Key Architecture Notes

1. **FactRef = { table_id: u32, fact_id: u32 }** - globally unique reference
2. **Truth tracking is separate from existence** - core modal logic concept
3. **Relation names are part of syntax** - enables runtime table lookup
4. **Per-relation opt-in** - use `:fact-tracking` flag on relations that need it

## Local Test Files

These `.egg` files in the root directory work but are gitignored (intentional):
- `test_fact_as_argument.egg`
- `test_inline_fact_argument.egg`
- `test_nested_facts.egg`
- `COMPLETE_FIRST_CLASS_FACTS_DEMO.egg`

They verify the syntax works with the workaround of asserting facts first.

## Questions?

Ask me to:
- Show you the current helper function implementation
- Explain the fact ID allocation architecture
- Walk through the type checking flow
- Review test files or documentation

Let's implement fact creation so modal logic works properly!
