use egglog::*;

#[test]
fn test_fact_ref_sort_integration() {
    let egraph = EGraph::default();
    
    // Test that FactRefSort is properly integrated into the EGraph
    // by creating a FactRef value and manipulating it
    use egglog_core_relations::{FactId, FactRef, TableId};
    use egglog_numeric_id::NumericId;
    
    // Create a test FactRef
    let fact_ref = FactRef {
        table_id: TableId::from_usize(42),
        fact_id: FactId::from_usize(123),
    };
    
    // Convert to base value and back - this tests the FactRefSort integration
    let base_value = egraph.base_to_value(fact_ref.clone());
    let retrieved = egraph.value_to_base::<FactRef>(base_value);
    
    assert_eq!(fact_ref, retrieved);
    println!("✓ FactRef successfully integrated with base value system");
}

#[test]
fn test_modal_logic_api_availability() {
    let egraph = EGraph::default();
    
    // Test that the modal logic API methods are available on EGraph
    // We can't test them fully without a proper table setup, but we can
    // verify they compile and handle invalid inputs gracefully
    
    use egglog_core_relations::{FactId, FactRef, TableId};
    use egglog_numeric_id::NumericId;
    
    let fact_ref = FactRef {
        table_id: TableId::from_usize(999),  // Non-existent table
        fact_id: FactId::from_usize(888),    // Non-existent fact
    };
    let fact_ref_value = egraph.base_to_value(fact_ref);
    
    // Test validate_fact_ref - should return error for non-existent table/fact
    let validation_result = egraph.validate_fact_ref(fact_ref_value);
    assert!(validation_result.is_err());
    println!("✓ validate_fact_ref handles invalid input correctly");
    
    // Test is_fact_ref_stale - should return true for non-existent fact
    let is_stale = egraph.is_fact_ref_stale(fact_ref_value);
    assert!(is_stale);
    println!("✓ is_fact_ref_stale handles invalid input correctly");
    
    // Test get_fact_truth_status - should return error for non-existent fact
    let truth_status = egraph.get_fact_truth_status(fact_ref_value);
    assert!(truth_status.is_err());
    println!("✓ get_fact_truth_status handles invalid input correctly");
}

#[test]
fn test_first_class_facts_egraph_integration() {
    // Test that EGraph properly integrates first-class facts functionality
    // by testing the methods that were added to the EGraph
    
    let mut egraph = EGraph::default();
    
    // Create a simple program with facts
    let _outputs = egraph
        .parse_and_run_program(
            None,
            r#"
                (datatype Entity (Person String))
                (relation active (Entity))
                (active (Person "Alice"))
                (run 1)
            "#,
        )
        .unwrap();
    
    // Test that first-class facts methods are available on EGraph
    // These tests verify the API exists and compiles, even if we can't
    // fully test functionality without access to internal table IDs
    
    use egglog_core_relations::{FactId, FactRef, TableId};
    use egglog_numeric_id::NumericId;
    
    let fake_fact_ref = FactRef {
        table_id: TableId::from_usize(999),
        fact_id: FactId::from_usize(888),
    };
    let fact_ref_value = egraph.base_to_value(fake_fact_ref);
    
    // These should all handle non-existent references gracefully
    let _validation = egraph.validate_fact_ref(fact_ref_value);
    let _stale_status = egraph.is_fact_ref_stale(fact_ref_value);
    let _truth_status = egraph.get_fact_truth_status(fact_ref_value);
    let _assert_result = egraph.assert_fact_validated(fact_ref_value);
    let _retract_result = egraph.retract_fact_validated(fact_ref_value);
    
    println!("✓ All first-class facts methods are available on EGraph");
}

#[test]
fn test_fact_ref_basevalue_interning() {
    // Test that FactRef can be properly interned as a BaseValue
    use egglog_core_relations::{FactId, FactRef, TableId};
    use egglog_numeric_id::NumericId;
    
    // Use EGraph's built-in base values system instead of creating our own
    let egraph = EGraph::default();
    
    // Create test FactRefs
    let fact_ref1 = FactRef {
        table_id: TableId::from_usize(1),
        fact_id: FactId::from_usize(100),
    };
    
    let fact_ref2 = FactRef {
        table_id: TableId::from_usize(2),
        fact_id: FactId::from_usize(200),
    };
    
    // Test interning FactRefs as BaseValues using EGraph methods
    let value1 = egraph.base_to_value(fact_ref1.clone());
    let value2 = egraph.base_to_value(fact_ref2.clone());
    
    // Different FactRefs should produce different Values
    assert_ne!(value1, value2);
    
    // Test unwrapping BaseValues back to FactRefs
    let unwrapped1 = egraph.value_to_base::<FactRef>(value1);
    let unwrapped2 = egraph.value_to_base::<FactRef>(value2);
    
    assert_eq!(unwrapped1, fact_ref1);
    assert_eq!(unwrapped2, fact_ref2);
    
    // Test that interning the same FactRef produces the same Value
    let value1_again = egraph.base_to_value(fact_ref1);
    assert_eq!(value1, value1_again);
    
    println!("✓ FactRef BaseValue interning works correctly");
}

