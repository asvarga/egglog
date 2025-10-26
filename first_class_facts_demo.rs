// This is what we CAN do right now (Rust API level):

use egglog::*;
use egglog_core_relations::{FactId, FactRef, TableId};
use egglog_numeric_id::NumericId;

fn demo_current_capabilities() {
    let mut egraph = EGraph::default();

    // 1. We can create FactRef values and manipulate them
    let fact_ref = FactRef {
        table_id: TableId::from_usize(1),
        fact_id: FactId::from_usize(100),
    };

    // 2. Convert to egglog Value and back
    let fact_value = egraph.base_to_value(fact_ref.clone());
    let retrieved = egraph.value_to_base::<FactRef>(fact_value);
    assert_eq!(fact_ref, retrieved);

    // 3. Use modal logic operations
    let _validation = egraph.validate_fact_ref(fact_value);
    let _is_stale = egraph.is_fact_ref_stale(fact_value);
    let _truth_status = egraph.get_fact_truth_status(fact_value);
    let _assert_result = egraph.assert_fact_validated(fact_value);

    println!("✓ All first-class facts operations work at Rust API level");
}

// This is what we CANNOT do yet (.egg file level):
/*
(datatype Person (Alice) (Bob))
(datatype City (Chicago) (NYC))
(relation Lives (Person City))
(relation Believes (Person FactRef))  ; ← FactRef sort exists but no primitives

; We want to write this but can't yet:
(Believes Alice (create-fact-ref Lives Bob Chicago))  ; ← create-fact-ref primitive missing
(assert-fact (create-fact-ref Lives Bob Chicago))     ; ← assert-fact primitive missing

; We want to query this but can't yet:
(query (and (Believes Alice ?fact-ref)
            (is-fact-asserted ?fact-ref)))            ; ← is-fact-asserted primitive missing
*/
