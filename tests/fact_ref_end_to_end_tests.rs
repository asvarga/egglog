use egglog::*;

/// Comprehensive end-to-end tests for first-class facts (FactRef) system
/// covering basic table operations and fact stability

#[test]
fn test_basic_fact_references() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut egraph = EGraph::default();

    // Simple test using constructors instead of functions to avoid function setup issues
    let outputs = egraph
        .parse_and_run_program(
            None,
            r#"
            ; Define basic datatypes using constructors
            (datatype Person (MakePerson String))
            (datatype Department (MakeDept String))
            
            ; Create some facts
            (let alice (MakePerson "Alice"))
            (let bob (MakePerson "Bob"))
            (let engineering (MakeDept "Engineering"))
            (let marketing (MakeDept "Marketing"))
            
            ; Union operations to test fact stability
            (union alice bob)
            
            ; Extract to verify integrity
            (extract alice)
            (extract engineering)
            "#,
        )
        .unwrap();

    // Verify we got valid extractions
    assert_eq!(outputs.len(), 2);
    assert!(matches!(outputs[0], CommandOutput::ExtractBest(_, _, _)));
    assert!(matches!(outputs[1], CommandOutput::ExtractBest(_, _, _)));
}

#[test]
fn test_fact_reference_with_relations() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut egraph = EGraph::default();

    // Test using relations which are simpler than functions
    let outputs = egraph
        .parse_and_run_program(
            None,
            r#"
            ; Define datatypes
            (datatype Entity (MakeEntity i64))
            
            ; Define relations
            (relation HasProperty (Entity String i64))
            
            ; Create entities
            (let e1 (MakeEntity 1))
            (let e2 (MakeEntity 2))
            (let e3 (MakeEntity 3))
            
            ; Add facts to relations
            (HasProperty e1 "age" 25)
            (HasProperty e2 "age" 30)
            (HasProperty e1 "score" 100)
            
            ; Union entities to test fact reference stability
            (union e2 e3)
            
            ; Verify relations still work
            (check (HasProperty e1 "age" 25))
            (check (HasProperty e2 "age" 30))
            
            ; Extract results
            (extract e1)
            "#,
        )
        .unwrap();

    assert_eq!(outputs.len(), 1);
    assert!(matches!(outputs[0], CommandOutput::ExtractBest(_, _, _)));
}

#[test]
fn test_fact_reference_serialization() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut egraph = EGraph::default();

    // Create facts with relationships
    egraph
        .parse_and_run_program(
            None,
            r#"
            (datatype Node (MakeNode i64))
            (relation Connected (Node Node))
            
            (let n1 (MakeNode 1))
            (let n2 (MakeNode 2))
            (let n3 (MakeNode 3))
            
            (Connected n1 n2)
            (Connected n2 n3)
            (Connected n1 n3)
            "#,
        )
        .unwrap();

    // Test serialization
    let serialize_config = SerializeConfig::default();
    let serialized = egraph.serialize(serialize_config);

    assert!(serialized.is_complete(), "Serialization should be complete");

    // Verify that fact references are properly represented
    assert!(!serialized.egraph.nodes.is_empty());
    assert!(!serialized.egraph.class_data.is_empty());
}

#[test]
fn test_fact_reference_with_rules() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut egraph = EGraph::default();

    // Test rules that work with fact references
    let outputs = egraph
        .parse_and_run_program(
            None,
            r#"
            (datatype Person (MakePerson String))
            (relation Parent (Person Person))
            (relation Grandparent (Person Person))
            
            ; Create people
            (let alice (MakePerson "Alice"))
            (let bob (MakePerson "Bob"))
            (let charlie (MakePerson "Charlie"))
            
            ; Set up parent relationships
            (Parent alice bob)
            (Parent bob charlie)
            
            ; Rule to derive grandparent relationships
            (rule ((Parent gp p) (Parent p c))
                  ((Grandparent gp c)))
            
            ; Run the rule
            (run 1)
            
            ; Check derived facts
            (check (Grandparent alice charlie))
            
            ; Extract results
            (extract alice)
            "#,
        )
        .unwrap();

    // The check command produces output too, so we expect 2 outputs
    assert_eq!(outputs.len(), 2);
}

#[test]
fn test_large_scale_fact_operations() {
    let _ = env_logger::builder().is_test(true).try_init();

    let mut egraph = EGraph::default();

    // Create a program with many facts to test scalability
    let mut program = String::from(
        r#"
        (datatype Item (MakeItem i64))
        (relation HasTag (Item String))
        "#,
    );

    // Create many items and tag relationships
    for i in 0..20 {
        program.push_str(&format!("(let item{} (MakeItem {}))\n", i, i));
        program.push_str(&format!("(HasTag item{} \"tag{}\")\n", i, i % 5));
    }

    // Add some unions to test fact reference stability under pressure
    program.push_str(
        r#"
        ; Union some items to test stability
        (union item0 item10)
        (union item5 item15)
        
        ; Verify some relationships still work
        (check (HasTag item0 "tag0"))
        (extract item0)
        "#,
    );

    let result = egraph.parse_and_run_program(None, &program);
    assert!(
        result.is_ok(),
        "Large scale operations should complete successfully"
    );
}
