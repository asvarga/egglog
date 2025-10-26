use std::hint::black_box;

use egglog_core_relations::{
    BaseValues, Database, FactId, FactRef, SortedWritesTable, TableId, Value,
};
use egglog_numeric_id::NumericId;

fn main() {
    divan::main();
}

// Simple benchmark without complex parameters
#[divan::bench]
fn fact_creation_small() {
    let mut db = Database::default();
    let table_id = db.add_table(create_test_table(), std::iter::empty(), std::iter::empty());

    for i in 0..100 {
        let values = vec![Value::from_usize(i), Value::from_usize(i * 2)];
        black_box(db.create_fact_ref(table_id, &values));
    }
}

#[divan::bench]
fn fact_creation_medium() {
    let mut db = Database::default();
    let table_id = db.add_table(create_test_table(), std::iter::empty(), std::iter::empty());

    for i in 0..1000 {
        let values = vec![Value::from_usize(i), Value::from_usize(i * 2)];
        black_box(db.create_fact_ref(table_id, &values));
    }
}

#[divan::bench]
fn fact_creation_large() {
    let mut db = Database::default();
    let table_id = db.add_table(create_test_table(), std::iter::empty(), std::iter::empty());

    for i in 0..10000 {
        let values = vec![Value::from_usize(i), Value::from_usize(i * 2)];
        black_box(db.create_fact_ref(table_id, &values));
    }
}

#[divan::bench]
fn fact_resolution_small() {
    let mut db = Database::default();
    let table_id = db.add_table(create_test_table(), std::iter::empty(), std::iter::empty());

    // Create facts first
    let fact_refs: Vec<_> = (0..100)
        .map(|i| {
            let values = vec![Value::from_usize(i), Value::from_usize(i * 2)];
            db.create_fact_ref(table_id, &values).unwrap()
        })
        .collect();

    // Benchmark fact resolution
    for fact_ref in &fact_refs {
        black_box(db.resolve_fact_ref(fact_ref));
    }
}

#[divan::bench]
fn basevalue_intern_bench() {
    let base_values = BaseValues::default();

    // Create test FactRefs
    let fact_refs: Vec<_> = (0..1000)
        .map(|i| FactRef {
            table_id: TableId::from_usize(i % 10),
            fact_id: FactId::from_usize(i),
        })
        .collect();

    // Benchmark interning FactRefs into BaseValue
    for fact_ref in &fact_refs {
        black_box(base_values.get(*fact_ref));
    }
}

#[divan::bench]
fn basevalue_unwrap_bench() {
    let base_values = BaseValues::default();

    // Create test FactRefs and intern them
    let values: Vec<_> = (0..1000)
        .map(|i| {
            let fact_ref = FactRef {
                table_id: TableId::from_usize(i % 10),
                fact_id: FactId::from_usize(i),
            };
            base_values.get(fact_ref)
        })
        .collect();

    // Benchmark unwrapping BaseValue back to FactRef
    for value in &values {
        black_box(base_values.unwrap::<FactRef>(*value));
    }
}

fn create_test_table() -> SortedWritesTable {
    SortedWritesTable::new(
        1,      // 1 key column
        2,      // 2 total columns
        None,   // No sorting
        vec![], // No rebuild columns
        Box::new(|_, _, new, out| {
            out.clear();
            out.extend_from_slice(new);
            false
        }),
    )
}
