use super::*;
use egglog_numeric_id::NumericId;

#[derive(Debug)]
pub struct FactRefSort;

impl BaseSort for FactRefSort {
    type Base = FactRef;

    fn name(&self) -> &str {
        "FactRef"
    }

    fn reconstruct_termdag(
        &self,
        base_values: &BaseValues,
        value: Value,
        termdag: &mut TermDag,
    ) -> Term {
        let fact_ref = base_values.unwrap::<FactRef>(value);
        // For now, represent FactRef as a function call with table_id and fact_id
        let table_id_term = termdag.lit(Literal::Int(fact_ref.table_id.rep() as i64));
        let fact_id_term = termdag.lit(Literal::Int(fact_ref.fact_id.rep() as i64));
        termdag.app("fact-ref".to_string(), vec![table_id_term, fact_id_term])
    }
}
