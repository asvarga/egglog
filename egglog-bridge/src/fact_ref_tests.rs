#[cfg(test)]
mod fact_ref_integration_tests {
    use crate::core_relations::{FactId, FactRef, TableId, Value};
    use crate::{ColumnTy, EGraph, QueryEntry};
    use egglog_numeric_id::NumericId;

    #[test]
    fn test_fact_ref_as_base_value() {
        let egraph = EGraph::default();

        // FactRef is now automatically registered as a base value type

        // Create a sample FactRef
        let table_id = TableId::from_usize(42);
        let fact_id = FactId::from_usize(123);
        let fact_ref = FactRef { table_id, fact_id };

        // Convert to Value and back
        let value = egraph.base_values().get(fact_ref.clone());
        let retrieved = egraph.base_values().unwrap::<FactRef>(value);

        assert_eq!(fact_ref, retrieved);
        assert_eq!(fact_ref.table_id, table_id);
        assert_eq!(fact_ref.fact_id, fact_id);
    }

    #[test]
    fn test_fact_ref_constant_creation() {
        let egraph = EGraph::default();

        // FactRef is now automatically registered as a base value type

        let table_id = TableId::from_usize(1);
        let fact_id = FactId::from_usize(456);
        let fact_ref = FactRef { table_id, fact_id };

        // Test base_value_constant method
        let query_entry = egraph.base_value_constant(fact_ref.clone());

        match query_entry {
            QueryEntry::Const { val, ty } => {
                let retrieved = egraph.base_values().unwrap::<FactRef>(val);
                assert_eq!(retrieved, fact_ref);

                // Verify the type is correct
                let expected_ty = ColumnTy::Base(egraph.base_values().get_ty::<FactRef>());
                assert_eq!(ty, expected_ty);
            }
            _ => panic!("Expected constant query entry"),
        }
    }

    #[test]
    fn test_create_fact_ref_from_nonexistent_table() {
        let mut egraph = EGraph::default();

        // Try to create fact ref from non-existent table
        let nonexistent_table = TableId::from_usize(999);
        let key = vec![Value::from_usize(1), Value::from_usize(2)];

        // This should not panic but return None since the table doesn't exist
        // For now, we expect this to panic with a bounds check error
        // In the future, this could be handled more gracefully
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            egraph.create_fact_ref(nonexistent_table, &key)
        }));

        assert!(
            result.is_err(),
            "Expected panic when accessing non-existent table"
        );
    }

    #[test]
    fn test_fact_ref_debug_formatting() {
        let table_id = TableId::from_usize(7);
        let fact_id = FactId::from_usize(89);
        let fact_ref = FactRef { table_id, fact_id };

        let debug_str = format!("{:?}", fact_ref);
        assert!(debug_str.contains("FactRef"));
        assert!(debug_str.contains("table_id"));
        assert!(debug_str.contains("fact_id"));
    }

    #[test]
    fn test_fact_ref_equality_and_hashing() {
        let table_id = TableId::from_usize(10);
        let fact_id = FactId::from_usize(20);

        let fact_ref1 = FactRef { table_id, fact_id };
        let fact_ref2 = FactRef { table_id, fact_id };
        let fact_ref3 = FactRef {
            table_id,
            fact_id: FactId::from_usize(21),
        };

        // Test equality
        assert_eq!(fact_ref1, fact_ref2);
        assert_ne!(fact_ref1, fact_ref3);

        // Test that equal FactRefs have the same hash
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher1 = DefaultHasher::new();
        fact_ref1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        fact_ref2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }
}
