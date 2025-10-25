#[cfg(test)]
mod fact_ref_high_level_api_tests {
    use crate::*;

    #[test]
    fn test_fact_ref_export() {
        // Simple test to verify FactRef is properly exported
        use crate::FactRef;
        use core_relations::FactId;
        use core_relations::TableId;
        use egglog_numeric_id::NumericId;

        let table_id = TableId::from_usize(1);
        let fact_id = FactId::from_usize(2);
        let fact_ref = FactRef { table_id, fact_id };

        // Verify basic properties
        assert_eq!(fact_ref.table_id, table_id);
        assert_eq!(fact_ref.fact_id, fact_id);

        // Verify Debug implementation works
        let debug_str = format!("{:?}", fact_ref);
        assert!(debug_str.contains("FactRef"));
    }

    #[test]
    fn test_fact_ref_in_egraph() {
        let egraph = EGraph::default();

        // FactRef is now automatically registered as a base value type
        // No manual registration needed!

        // Create a sample FactRef and convert to Value
        use core_relations::{FactId, TableId};
        use egglog_numeric_id::NumericId;

        let table_id = TableId::from_usize(42);
        let fact_id = FactId::from_usize(123);
        let fact_ref = FactRef { table_id, fact_id };

        let value = egraph.base_to_value(fact_ref.clone());
        let retrieved = egraph.value_to_base::<FactRef>(value);

        assert_eq!(fact_ref, retrieved);
    }

    #[test]
    fn test_modal_logic_operations() {
        let mut egraph = EGraph::default();

        // Create a simple table to work with
        let outputs = egraph
            .parse_and_run_program(
                None,
                r#"
                ; Define a simple datatype
                (datatype Color (Red) (Blue) (Green))
                
                ; Create some facts and extract them to produce outputs
                (extract (Red))
                (extract (Blue))
                "#,
            )
            .unwrap();

        // Test basic fact operations work
        assert_eq!(outputs.len(), 2);

        // Note: More comprehensive modal logic tests would require:
        // 1. Access to table IDs from the program
        // 2. Methods to create unasserted facts
        // 3. Truth status manipulation
        // For now, this tests that the API is available and compiles correctly
    }

    #[test]
    fn test_fact_validation_methods() {
        let egraph = EGraph::default();

        // Create a sample FactRef
        use core_relations::{FactId, TableId};
        use egglog_numeric_id::NumericId;

        let table_id = TableId::from_usize(42);
        let fact_id = FactId::from_usize(123);
        let fact_ref = FactRef { table_id, fact_id };
        let fact_ref_val = egraph.base_to_value(fact_ref);

        // Test validation methods - they should fail since the table doesn't exist
        // but we're testing that the API works correctly

        // Test validate_fact_ref - should return error for non-existent table/fact
        let validation_result = egraph.validate_fact_ref(fact_ref_val);
        assert!(validation_result.is_err());

        // Test is_fact_ref_stale - should return true for non-existent fact
        let is_stale = egraph.is_fact_ref_stale(fact_ref_val);
        assert!(is_stale);

        // Test get_fact_truth_status - should return error for non-existent fact
        let truth_status = egraph.get_fact_truth_status(fact_ref_val);
        assert!(truth_status.is_err());

        // Test assert_fact_validated - should return error for non-existent fact
        let mut egraph_mut = egraph;
        let assert_result = egraph_mut.assert_fact_validated(fact_ref_val);
        assert!(assert_result.is_err());

        // Test retract_fact_validated - should return error for non-existent fact
        let retract_result = egraph_mut.retract_fact_validated(fact_ref_val);
        assert!(retract_result.is_err());

        // All methods successfully returned appropriate errors for invalid input
    }
}
