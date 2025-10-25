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
}
