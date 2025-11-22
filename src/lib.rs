//! # egglog
//! egglog is a language specialized for writing equality saturation
//! applications. It is the successor to the rust library [egg](https://github.com/egraphs-good/egg).
//! egglog is faster and more general than egg.
//!
//! # Documentation
//! Documentation for the egglog language can be found
//! here: [`Command`]
//!
//! # Tutorial
//! [Here](https://www.youtube.com/watch?v=N2RDQGRBrSY) is the video tutorial on what egglog is and how to use it.
//! We plan to have a text tutorial here soon, PRs welcome!
//!
pub mod ast;
#[cfg(feature = "bin")]
mod cli;
pub mod constraint;
mod core;
pub mod extract;
#[cfg(test)]
mod fact_ref_api_tests;
pub mod prelude;
pub mod scheduler;
mod serialize;
pub mod sort;
mod termdag;
mod typechecking;
pub mod util;

// This is used to allow the `add_primitive` macro to work in
// both this crate and other crates by referring to `::egglog`.
extern crate self as egglog;
use ast::*;
#[cfg(feature = "bin")]
pub use cli::*;
use constraint::{Constraint, Problem, SimpleTypeConstraint, TypeConstraint};
use core::{AtomTerm, ResolvedAtomTerm, ResolvedCall};
pub use core_relations::{BaseValue, ContainerValue, ExecutionState, FactRef, Value};
use core_relations::{ExternalFunctionId, make_external_func};
use csv::Writer;
pub use egglog_add_primitive::add_primitive;
use egglog_ast::generic_ast::{Change, GenericExpr, Literal};
use egglog_ast::span::Span;
use egglog_ast::util::ListDisplay;
pub use egglog_bridge::FunctionRow;
use egglog_bridge::{ColumnTy, QueryEntry};
use egglog_core_relations as core_relations;
use egglog_numeric_id as numeric_id;
use egglog_reports::{ReportLevel, RunReport};
use extract::{CostModel, DefaultCost, Extractor, TreeAdditiveCostModel};
use indexmap::map::Entry;
use log::{Level, log_enabled};
use numeric_id::DenseIdMap;
use prelude::*;
use scheduler::{SchedulerId, SchedulerRecord};
pub use serialize::{SerializeConfig, SerializeOutput, SerializedNode};
use sort::*;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::hash::Hash;
use std::io::{Read, Write as _};
use std::iter::once;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
pub use termdag::{Term, TermDag, TermId};
use thiserror::Error;
pub use typechecking::TypeError;
use typechecking::TypeInfo;
use util::*;

use crate::core::{GenericActionsExt, ResolvedRuleExt};

pub type ArcSort = Arc<dyn Sort>;

/// A trait for implementing custom primitive operations in egglog.
///
/// Primitives are built-in functions that can be called in both rule queries and actions.
pub trait Primitive {
    /// Returns the name of this primitive operation.
    fn name(&self) -> &str;

    /// Constructs a type constraint for the primitive that uses the span information
    /// for error localization.
    fn get_type_constraints(&self, span: &Span) -> Box<dyn TypeConstraint>;

    /// Applies the primitive operation to the given arguments.
    ///
    /// Returns `Some(value)` if the operation succeeds, or `None` if it fails.
    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value>;
}

/// A user-defined command output trait.
pub trait UserDefinedCommandOutput: Debug + std::fmt::Display + Send + Sync {}
impl<T> UserDefinedCommandOutput for T where T: Debug + std::fmt::Display + Send + Sync {}

/// Output from a command.
#[derive(Clone, Debug)]
pub enum CommandOutput {
    /// The size of a function
    PrintFunctionSize(usize),
    /// The name of all functions and their sizes
    PrintAllFunctionsSize(Vec<(String, usize)>),
    /// The best function found after extracting
    ExtractBest(TermDag, DefaultCost, Term),
    /// The variants of a function found after extracting
    ExtractVariants(TermDag, Vec<Term>),
    /// The report from all runs
    OverallStatistics(RunReport),
    /// A printed function and all its values
    PrintFunction(Function, TermDag, Vec<(Term, Term)>, PrintFunctionMode),
    /// The report from a single run
    RunSchedule(RunReport),
    /// A user defined output
    UserDefined(Arc<dyn UserDefinedCommandOutput>),
}

impl std::fmt::Display for CommandOutput {
    /// Format the command output for display, ending with a newline.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandOutput::PrintFunctionSize(size) => writeln!(f, "{}", size),
            CommandOutput::PrintAllFunctionsSize(names_and_sizes) => {
                for name in names_and_sizes {
                    writeln!(f, "{}: {}", name.0, name.1)?;
                }
                Ok(())
            }
            CommandOutput::ExtractBest(termdag, _cost, term) => {
                writeln!(f, "{}", termdag.to_string(term))
            }
            CommandOutput::ExtractVariants(termdag, terms) => {
                writeln!(f, "(")?;
                for expr in terms {
                    writeln!(f, "   {}", termdag.to_string(expr))?;
                }
                writeln!(f, ")")
            }
            CommandOutput::OverallStatistics(run_report) => {
                write!(f, "Overall statistics:\n{}", run_report)
            }
            CommandOutput::PrintFunction(function, termdag, terms_and_outputs, mode) => {
                let out_is_unit = function.schema.output.name() == UnitSort.name();
                if *mode == PrintFunctionMode::CSV {
                    let mut wtr = Writer::from_writer(vec![]);
                    for (term, output) in terms_and_outputs {
                        match term {
                            Term::App(name, children) => {
                                let mut values = vec![name.clone()];
                                for child_id in children {
                                    values.push(termdag.to_string(termdag.get(*child_id)));
                                }

                                if !out_is_unit {
                                    values.push(termdag.to_string(output));
                                }
                                wtr.write_record(&values).map_err(|_| std::fmt::Error)?;
                            }
                            _ => panic!("Expect function_to_dag to return a list of apps."),
                        }
                    }
                    let csv_bytes = wtr.into_inner().map_err(|_| std::fmt::Error)?;
                    f.write_str(&String::from_utf8(csv_bytes).map_err(|_| std::fmt::Error)?)
                } else {
                    writeln!(f, "(")?;
                    for (term, output) in terms_and_outputs.iter() {
                        write!(f, "   {}", termdag.to_string(term))?;
                        if !out_is_unit {
                            write!(f, " -> {}", termdag.to_string(output))?;
                        }
                        writeln!(f)?;
                    }
                    writeln!(f, ")")
                }
            }
            CommandOutput::RunSchedule(_report) => Ok(()),
            CommandOutput::UserDefined(output) => {
                write!(f, "{}", *output)
            }
        }
    }
}

/// The main interface for an e-graph in egglog.
///
/// An [`EGraph`] maintains a collection of equivalence classes of terms and provides
/// operations for adding facts, running rules, and extracting optimal terms.
///
/// # Examples
///
/// ```
/// use egglog::*;
///
/// let mut egraph = EGraph::default();
/// egraph.parse_and_run_program(None, "(datatype Math (Num i64) (Add Math Math))").unwrap();
/// ```
#[derive(Clone)]
pub struct EGraph {
    backend: egglog_bridge::EGraph,
    pub parser: Parser,
    names: check_shadowing::Names,
    /// pushed_egraph forms a linked list of pushed egraphs.
    /// Pop reverts the egraph to the last pushed egraph.
    pushed_egraph: Option<Box<Self>>,
    functions: IndexMap<String, Function>,
    rulesets: IndexMap<String, Ruleset>,
    pub fact_directory: Option<PathBuf>,
    pub seminaive: bool,
    type_info: TypeInfo,
    /// The run report unioned over all runs so far.
    overall_run_report: RunReport,
    schedulers: DenseIdMap<SchedulerId, SchedulerRecord>,
    commands: IndexMap<String, Arc<dyn UserDefinedCommand>>,
    /// Mapping from function names to TableIds for the fact-ref helper function
    table_id_map:
        Option<std::sync::Arc<std::sync::Mutex<IndexMap<String, core_relations::TableId>>>>,
}

/// A user-defined command allows users to inject custom command that can be called
/// in an egglog program.
///
/// Compared to an external function, a user-defined command is more powerful because
/// it has an exclusive access to the e-graph.
pub trait UserDefinedCommand: Send + Sync {
    /// Run the command with the given arguments.
    fn update(&self, egraph: &mut EGraph, args: &[Expr]) -> Result<Option<CommandOutput>, Error>;
}

/// A function in the e-graph.
///
/// This contains the schema information of the function and
/// the backend id of the function in the e-graph.
#[derive(Clone)]
pub struct Function {
    decl: ResolvedFunctionDecl,
    schema: ResolvedSchema,
    can_subsume: bool,
    backend_id: egglog_bridge::FunctionId,
}

impl Function {
    /// Get the name of the function.
    pub fn name(&self) -> &str {
        &self.decl.name
    }

    /// Get the schema of the function.
    pub fn schema(&self) -> &ResolvedSchema {
        &self.schema
    }

    /// Whether this function supports subsumption.
    pub fn can_subsume(&self) -> bool {
        self.can_subsume
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedSchema {
    pub input: Vec<ArcSort>,
    pub output: ArcSort,
}

impl ResolvedSchema {
    /// Get the type at position `index`, counting the `output` sort as at position `input.len()`.
    pub fn get_by_pos(&self, index: usize) -> Option<&ArcSort> {
        if self.input.len() == index {
            Some(&self.output)
        } else {
            self.input.get(index)
        }
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function")
            .field("decl", &self.decl)
            .field("schema", &self.schema)
            .finish()
    }
}

/// Special primitive for creating fact references
/// Syntax: (fact-ref "RelationName" arg1 arg2 ... argN)
/// Creates or retrieves a FactRef for the given tuple in the specified relation
///
/// This primitive uses a helper external function to access the EGraph's
/// function table and create fact references.
#[derive(Clone)]
pub struct FactRefPrimitive {
    /// External function ID for the helper that creates fact refs
    helper_func_id: std::sync::Arc<std::sync::Mutex<Option<ExternalFunctionId>>>,
}

impl FactRefPrimitive {
    fn new() -> Self {
        Self {
            helper_func_id: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    fn set_helper_func(&self, func_id: ExternalFunctionId) {
        *self.helper_func_id.lock().unwrap() = Some(func_id);
    }
}

impl Primitive for FactRefPrimitive {
    fn name(&self) -> &str {
        "fact-ref"
    }

    fn get_type_constraints(&self, _span: &Span) -> Box<dyn TypeConstraint> {
        // The type constraints are handled specially in constraint.rs
        // This should never actually be called since we handle fact-ref specially
        panic!(
            "FactRefPrimitive::get_type_constraints should not be called - fact-ref is handled specially in constraint solving"
        );
    }

    fn apply(&self, exec_state: &mut ExecutionState, args: &[Value]) -> Option<Value> {
        // Get the helper function ID
        let helper_id = self
            .helper_func_id
            .lock()
            .unwrap()
            .expect("FactRefPrimitive helper function not initialized");

        // Delegate to the helper function which has access to EGraph internals
        exec_state.call_external_func(helper_id, args)
    }
}

impl Default for EGraph {
    fn default() -> Self {
        let mut eg = Self {
            backend: Default::default(),
            parser: Default::default(),
            names: Default::default(),
            pushed_egraph: Default::default(),
            functions: Default::default(),
            rulesets: Default::default(),
            fact_directory: None,
            seminaive: true,
            overall_run_report: Default::default(),
            type_info: Default::default(),
            schedulers: Default::default(),
            commands: Default::default(),
            table_id_map: None,
        };

        add_base_sort(&mut eg, UnitSort, span!()).unwrap();
        add_base_sort(&mut eg, StringSort, span!()).unwrap();
        add_base_sort(&mut eg, BoolSort, span!()).unwrap();
        add_base_sort(&mut eg, I64Sort, span!()).unwrap();
        add_base_sort(&mut eg, F64Sort, span!()).unwrap();
        add_base_sort(&mut eg, BigIntSort, span!()).unwrap();
        add_base_sort(&mut eg, BigRatSort, span!()).unwrap();
        add_base_sort(&mut eg, FactRefSort, span!()).unwrap();

        // Reserve "fact-ref" as a special primitive for creating fact references
        eg.type_info.reserved_primitives.insert("fact-ref");

        // Create the FactRefPrimitive
        let fact_ref_primitive = FactRefPrimitive::new();
        let fact_ref_helper = fact_ref_primitive.clone();

        // Register the primitive
        eg.add_primitive(fact_ref_primitive);

        // Now register a helper external function that has access to a mapping from
        // function names to TableIds. This helper will be called by the primitive
        // to create fact references.
        //
        // We store a mapping of function names to TableIds (from core-relations)
        // rather than storing the full Function objects.
        let table_id_map: std::sync::Arc<
            std::sync::Mutex<IndexMap<String, core_relations::TableId>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(IndexMap::default()));
        let table_id_map_ref = table_id_map.clone();

        let helper_func_id =
            eg.backend
                .register_external_func(make_external_func(move |exec_state, args| {
                    // Args: [relation_name_string, arg1, arg2, ..., argN]
                    if args.is_empty() {
                        return None;
                    }

                    // Extract the relation name from the first argument (a String value)
                    let relation_name_value = args[0];

                    // Get the string from base values
                    // Strings are stored as Boxed<String> in egglog (see sort::S)
                    let base_values = exec_state.base_values();
                    let boxed_string: core_relations::Boxed<String> =
                        base_values.unwrap(relation_name_value);
                    let relation_name_str: &str = boxed_string.deref();

                    // Look up the table ID from our mapping
                    let table_ids = table_id_map_ref.lock().unwrap();
                    let table_id = *table_ids.get(relation_name_str)?;
                    drop(table_ids); // Release lock

                    // Remaining arguments are the tuple key
                    let key = &args[1..];

                    // Create or lookup the fact reference, creating an unasserted fact if needed
                    // This is the key change: instead of just looking up, we create if it doesn't exist
                    // For relations (fact-tracked functions), the return value is always Unit
                    let unit = Value::new_const(0);
                    if let Some(fact_ref) =
                        exec_state.create_unasserted_fact_ref(table_id, key, unit)
                    {
                        let base_values = exec_state.base_values();
                        return Some(base_values.get(fact_ref));
                    }

                    // Fact tracking not enabled on this table
                    None
                }));

        // Set the helper function ID in the primitive
        fact_ref_helper.set_helper_func(helper_func_id);

        // Store the table_id_map so the helper can access it
        // Note: We'll need to populate/update this mapping when functions are added
        // For now it's empty, we'll update it in declare_function
        eg.table_id_map = Some(table_id_map);

        eg.type_info.add_presort::<MapSort>(span!()).unwrap();
        eg.type_info.add_presort::<SetSort>(span!()).unwrap();
        eg.type_info.add_presort::<VecSort>(span!()).unwrap();
        eg.type_info.add_presort::<FunctionSort>(span!()).unwrap();
        eg.type_info.add_presort::<MultiSetSort>(span!()).unwrap();

        add_primitive!(&mut eg, "!=" = |a: #, b: #| -?> () {
            (a != b).then_some(())
        });
        add_primitive!(&mut eg, "value-eq" = |a: #, b: #| -?> () {
            (a == b).then_some(())
        });
        add_primitive!(&mut eg, "ordering-min" = |a: #, b: #| -> # {
            if a < b { a } else { b }
        });
        add_primitive!(&mut eg, "ordering-max" = |a: #, b: #| -> # {
            if a > b { a } else { b }
        });

        eg.rulesets
            .insert("".into(), Ruleset::Rules(Default::default()));

        // Add the `fact` expression macro for creating fact references
        eg.parser
            .add_expr_macro(Arc::new(SimpleMacro::new("fact", |args, span, parser| {
                if args.is_empty() {
                    return Err(ParseError(
                        span,
                        "fact requires a relation name and arguments".to_string(),
                    ));
                }

                // First argument must be the relation name (an atom)
                let relation_name = args[0].expect_atom("relation name in fact expression")?;

                // Parse the remaining arguments as expressions
                let mut fact_args = Vec::new();
                for arg in &args[1..] {
                    fact_args.push(parser.parse_expr(arg)?);
                }

                // Return a Call expression: (fact-ref relation_name arg1 arg2 ...)
                // We use "fact-ref" as an internal marker that will be handled during typechecking
                let mut all_args = vec![Expr::Lit(span.clone(), Literal::String(relation_name))];
                all_args.extend(fact_args);

                Ok(Expr::Call(span, "fact-ref".to_string(), all_args))
            })));

        eg
    }
}

#[derive(Debug, Error)]
#[error("Not found: {0}")]
pub struct NotFoundError(String);

impl EGraph {
    /// Add a user-defined command to the e-graph
    pub fn add_command(
        &mut self,
        name: String,
        command: Arc<dyn UserDefinedCommand>,
    ) -> Result<(), Error> {
        if self.commands.contains_key(&name)
            || self.functions.contains_key(&name)
            || self.type_info.get_prims(&name).is_some()
        {
            return Err(Error::CommandAlreadyExists(name, span!()));
        }
        self.commands.insert(name.clone(), command);
        self.parser.add_user_defined(name)?;
        Ok(())
    }

    /// Push a snapshot of the e-graph into the stack.
    ///
    /// See [`EGraph::pop`].
    pub fn push(&mut self) {
        let prev_prev: Option<Box<Self>> = self.pushed_egraph.take();
        let mut prev = self.clone();
        prev.pushed_egraph = prev_prev;
        self.pushed_egraph = Some(Box::new(prev));
    }

    /// Pop the current egraph off the stack, replacing
    /// it with the previously pushed egraph.
    /// It preserves the run report and messages from the popped
    /// egraph.
    pub fn pop(&mut self) -> Result<(), Error> {
        match self.pushed_egraph.take() {
            Some(e) => {
                // Copy the overall report from the popped egraph
                let overall_run_report = self.overall_run_report.clone();
                *self = *e;
                self.overall_run_report = overall_run_report;
                Ok(())
            }
            None => Err(Error::Pop(span!())),
        }
    }

    fn translate_expr_to_mergefn(
        &self,
        expr: &ResolvedExpr,
    ) -> Result<egglog_bridge::MergeFn, Error> {
        match expr {
            GenericExpr::Lit(_, literal) => {
                let val = literal_to_value(&self.backend, literal);
                Ok(egglog_bridge::MergeFn::Const(val))
            }
            GenericExpr::Var(span, resolved_var) => match resolved_var.name.as_str() {
                "old" => Ok(egglog_bridge::MergeFn::Old),
                "new" => Ok(egglog_bridge::MergeFn::New),
                // NB: type-checking should already catch unbound variables here.
                _ => Err(TypeError::Unbound(resolved_var.name.clone(), span.clone()).into()),
            },
            GenericExpr::Call(_, ResolvedCall::Func(f), args) => {
                let translated_args = args
                    .iter()
                    .map(|arg| self.translate_expr_to_mergefn(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(egglog_bridge::MergeFn::Function(
                    self.functions[&f.name].backend_id,
                    translated_args,
                ))
            }
            GenericExpr::Call(_, ResolvedCall::Primitive(p), args) => {
                let translated_args = args
                    .iter()
                    .map(|arg| self.translate_expr_to_mergefn(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(egglog_bridge::MergeFn::Primitive(
                    p.primitive.1,
                    translated_args,
                ))
            }
        }
    }

    fn declare_function(&mut self, decl: &ResolvedFunctionDecl) -> Result<(), Error> {
        let get_sort = |name: &String| match self.type_info.get_sort_by_name(name) {
            Some(sort) => Ok(sort.clone()),
            None => Err(Error::TypeError(TypeError::UndefinedSort(
                name.to_owned(),
                decl.span.clone(),
            ))),
        };

        let input = decl
            .schema
            .input
            .iter()
            .map(get_sort)
            .collect::<Result<Vec<_>, _>>()?;
        let output = get_sort(&decl.schema.output)?;

        let can_subsume = match decl.subtype {
            FunctionSubtype::Constructor => true,
            FunctionSubtype::Relation => true,
            FunctionSubtype::Custom => false,
        };

        use egglog_bridge::{DefaultVal, MergeFn};
        let backend_id = self.backend.add_table(egglog_bridge::FunctionConfig {
            schema: input
                .iter()
                .chain([&output])
                .map(|sort| sort.column_ty(&self.backend))
                .collect(),
            default: match decl.subtype {
                FunctionSubtype::Constructor => DefaultVal::FreshId,
                FunctionSubtype::Custom => DefaultVal::Fail,
                FunctionSubtype::Relation => DefaultVal::Const(self.backend.base_values().get(())),
            },
            merge: match decl.subtype {
                FunctionSubtype::Constructor => MergeFn::UnionId,
                FunctionSubtype::Relation => MergeFn::AssertEq,
                FunctionSubtype::Custom => match &decl.merge {
                    None => MergeFn::AssertEq,
                    Some(expr) => self.translate_expr_to_mergefn(expr)?,
                },
            },
            name: decl.name.to_string(),
            can_subsume,
            will_enable_truth_tracking: decl.fact_tracking, // Pass through fact tracking flag
        });

        let function = Function {
            decl: decl.clone(),
            schema: ResolvedSchema { input, output },
            can_subsume,
            backend_id,
        };

        let old = self.functions.insert(decl.name.clone(), function);
        if old.is_some() {
            panic!(
                "Typechecking should have caught function already bound: {}",
                decl.name
            );
        }

        // Update the table_id_map for the fact-ref helper function
        if let Some(table_id_map) = &self.table_id_map {
            let table_id = self.backend.get_table_id(backend_id);
            table_id_map
                .lock()
                .unwrap()
                .insert(decl.name.clone(), table_id);
        }

        // Enable truth tracking if requested
        if decl.fact_tracking {
            self.backend.enable_truth_tracking(backend_id);
        }

        Ok(())
    }

    /// Extract rows of a table using the default cost model with name sym
    /// The `include_output` parameter controls whether the output column is always extracted
    /// For functions, the output column is usually useful
    /// For constructors and relations, the output column can be ignored
    pub fn function_to_dag(
        &self,
        sym: &str,
        n: usize,
        include_output: bool,
    ) -> Result<(Vec<Term>, Option<Vec<Term>>, TermDag), Error> {
        let func = self
            .functions
            .get(sym)
            .ok_or(TypeError::UnboundFunction(sym.to_owned(), span!()))?;
        let mut rootsorts = func.schema.input.clone();
        if include_output {
            rootsorts.push(func.schema.output.clone());
        }
        let extractor = Extractor::compute_costs_from_rootsorts(
            Some(rootsorts),
            self,
            TreeAdditiveCostModel::default(),
        );

        let mut termdag = TermDag::default();
        let mut inputs: Vec<Term> = Vec::new();
        let mut output: Option<Vec<Term>> = if include_output {
            Some(Vec::new())
        } else {
            None
        };

        let extract_row = |row: egglog_bridge::FunctionRow| {
            if inputs.len() < n {
                // include subsumed rows
                let mut children: Vec<Term> = Vec::new();
                for (value, sort) in row.vals.iter().zip(&func.schema.input) {
                    let (_, term) = extractor
                        .extract_best_with_sort(self, &mut termdag, *value, sort.clone())
                        .unwrap_or_else(|| (0, termdag.var("Unextractable".into())));
                    children.push(term);
                }
                inputs.push(termdag.app(sym.to_owned(), children));
                if include_output {
                    let value = row.vals[func.schema.input.len()];
                    let sort = &func.schema.output;
                    let (_, term) = extractor
                        .extract_best_with_sort(self, &mut termdag, value, sort.clone())
                        .unwrap_or_else(|| (0, termdag.var("Unextractable".into())));
                    output.as_mut().unwrap().push(term);
                }
                true
            } else {
                false
            }
        };

        self.backend.for_each_while(func.backend_id, extract_row);

        Ok((inputs, output, termdag))
    }

    /// Print up to `n` the tuples in a given function.
    /// Print all tuples if `n` is not provided.
    pub fn print_function(
        &mut self,
        sym: &str,
        n: Option<usize>,
        file: Option<File>,
        mode: PrintFunctionMode,
    ) -> Result<Option<CommandOutput>, Error> {
        let n = match n {
            Some(n) => {
                log::info!("Printing up to {n} tuples of function {sym} as {mode}");
                n
            }
            None => {
                log::info!("Printing all tuples of function {sym} as {mode}");
                usize::MAX
            }
        };

        let (terms, outputs, termdag) = self.function_to_dag(sym, n, true)?;
        let f = self
            .functions
            .get(sym)
            // function_to_dag should have checked this
            .unwrap();
        let terms_and_outputs: Vec<_> = terms.into_iter().zip(outputs.unwrap()).collect();
        let output = CommandOutput::PrintFunction(f.clone(), termdag, terms_and_outputs, mode);
        match file {
            Some(mut file) => {
                log::info!("Writing output to file");
                file.write_all(output.to_string().as_bytes())
                    .expect("Error writing to file");
                Ok(None)
            }
            None => Ok(Some(output)),
        }
    }

    /// Print the size of a function. If no function name is provided,
    /// print the size of all functions in "name: len" pairs.
    pub fn print_size(&mut self, sym: Option<&str>) -> Result<CommandOutput, Error> {
        if let Some(sym) = sym {
            let f = self
                .functions
                .get(sym)
                .ok_or(TypeError::UnboundFunction(sym.to_owned(), span!()))?;
            let size = self.backend.table_size(f.backend_id);
            log::info!("Function {} has size {}", sym, size);
            Ok(CommandOutput::PrintFunctionSize(size))
        } else {
            // Print size of all functions
            let mut lens = self
                .functions
                .iter()
                .map(|(sym, f)| (sym.clone(), self.backend.table_size(f.backend_id)))
                .collect::<Vec<_>>();

            // Function name's alphabetical order
            lens.sort_by_key(|(name, _)| name.clone());
            if log_enabled!(Level::Info) {
                for (sym, len) in &lens {
                    log::info!("Function {} has size {}", sym, len);
                }
            }
            Ok(CommandOutput::PrintAllFunctionsSize(lens))
        }
    }

    // returns whether the egraph was updated
    fn run_schedule(&mut self, sched: &ResolvedSchedule) -> Result<RunReport, Error> {
        match sched {
            ResolvedSchedule::Run(span, config) => self.run_rules(span, config),
            ResolvedSchedule::Repeat(_span, limit, sched) => {
                let mut report = RunReport::default();
                for _i in 0..*limit {
                    let rec = self.run_schedule(sched)?;
                    let updated = rec.updated;
                    report.union(rec);
                    if !updated {
                        break;
                    }
                }
                Ok(report)
            }
            ResolvedSchedule::Saturate(_span, sched) => {
                let mut report = RunReport::default();
                loop {
                    let rec = self.run_schedule(sched)?;
                    let updated = rec.updated;
                    report.union(rec);
                    if !updated {
                        break;
                    }
                }
                Ok(report)
            }
            ResolvedSchedule::Sequence(_span, scheds) => {
                let mut report = RunReport::default();
                for sched in scheds {
                    report.union(self.run_schedule(sched)?);
                }
                Ok(report)
            }
        }
    }

    /// Extract a value to a [`TermDag`] and [`Term`] in the [`TermDag`] using the default cost model.
    /// See also [`EGraph::extract_value_with_cost_model`] for more control.
    pub fn extract_value(
        &self,
        sort: &ArcSort,
        value: Value,
    ) -> Result<(TermDag, Term, DefaultCost), Error> {
        self.extract_value_with_cost_model(sort, value, TreeAdditiveCostModel::default())
    }

    /// Extract a value to a [`TermDag`] and [`Term`] in the [`TermDag`].
    /// Note that the `TermDag` may contain a superset of the nodes in the `Term`.
    /// See also [`EGraph::extract_value_to_string`] for convenience.
    pub fn extract_value_with_cost_model<CM: CostModel<DefaultCost> + 'static>(
        &self,
        sort: &ArcSort,
        value: Value,
        cost_model: CM,
    ) -> Result<(TermDag, Term, DefaultCost), Error> {
        let extractor =
            Extractor::compute_costs_from_rootsorts(Some(vec![sort.clone()]), self, cost_model);
        let mut termdag = TermDag::default();
        let (cost, term) = extractor.extract_best(self, &mut termdag, value).unwrap();
        Ok((termdag, term, cost))
    }

    /// Extract a value to a string for printing.
    /// See also [`EGraph::extract_value`] for more control.
    pub fn extract_value_to_string(
        &self,
        sort: &ArcSort,
        value: Value,
    ) -> Result<(String, DefaultCost), Error> {
        let (termdag, term, cost) = self.extract_value(sort, value)?;
        Ok((termdag.to_string(&term), cost))
    }

    fn run_rules(&mut self, span: &Span, config: &ResolvedRunConfig) -> Result<RunReport, Error> {
        let mut report: RunReport = Default::default();

        let GenericRunConfig { ruleset, until } = config;

        if let Some(facts) = until {
            if self.check_facts(span, facts).is_ok() {
                log::info!(
                    "Breaking early because of facts:\n {}!",
                    ListDisplay(facts, "\n")
                );
                return Ok(report);
            }
        }

        let subreport = self.step_rules(ruleset)?;
        report.union(subreport);

        if log_enabled!(Level::Debug) {
            log::debug!("database size: {}", self.num_tuples());
        }

        Ok(report)
    }

    /// Runs a ruleset for an iteration.
    ///
    /// This applies every match it finds (under semi-naive).
    /// See [`EGraph::step_rules_with_scheduler`] for more fine-grained control.
    ///
    /// This will return an error if an egglog primitive returns None in an action.
    pub fn step_rules(&mut self, ruleset: &str) -> Result<RunReport, Error> {
        fn collect_rule_ids(
            ruleset: &str,
            rulesets: &IndexMap<String, Ruleset>,
            ids: &mut Vec<egglog_bridge::RuleId>,
        ) {
            match &rulesets[ruleset] {
                Ruleset::Rules(rules) => {
                    for (_, id) in rules.values() {
                        ids.push(*id);
                    }
                }
                Ruleset::Combined(sub_rulesets) => {
                    for sub_ruleset in sub_rulesets {
                        collect_rule_ids(sub_ruleset, rulesets, ids);
                    }
                }
            }
        }

        let mut rule_ids = Vec::new();
        collect_rule_ids(ruleset, &self.rulesets, &mut rule_ids);

        let iteration_report = self
            .backend
            .run_rules(&rule_ids)
            .map_err(|e| Error::BackendError(e.to_string()))?;

        Ok(RunReport::singleton(ruleset, iteration_report))
    }

    fn add_rule(&mut self, rule: ast::ResolvedRule) -> Result<String, Error> {
        let core_rule =
            rule.to_canonicalized_core_rule(&self.type_info, &mut self.parser.symbol_gen)?;
        let (query, actions) = (&core_rule.body, &core_rule.head);

        let rule_id = {
            let mut translator = BackendRule::new(
                self.backend.new_rule(&rule.name, self.seminaive),
                &self.functions,
                &self.type_info,
            );
            // Regular rules don't include subsumed facts, and should see all facts (asserted and unasserted)
            translator.query(query, false, false);
            translator.actions(actions)?;
            translator.build()
        };

        if let Some(rules) = self.rulesets.get_mut(&rule.ruleset) {
            match rules {
                Ruleset::Rules(rules) => {
                    match rules.entry(rule.name.clone()) {
                        indexmap::map::Entry::Occupied(_) => {
                            let name = rule.name;
                            panic!("Rule '{name}' was already present")
                        }
                        indexmap::map::Entry::Vacant(e) => e.insert((core_rule, rule_id)),
                    };
                    Ok(rule.name)
                }
                Ruleset::Combined(_) => Err(Error::CombinedRulesetError(rule.ruleset, rule.span)),
            }
        } else {
            Err(Error::NoSuchRuleset(rule.ruleset, rule.span))
        }
    }

    fn eval_actions(&mut self, actions: &ResolvedActions) -> Result<(), Error> {
        eprintln!(
            "[DEBUG eval_actions] Processing {} actions",
            actions.0.len()
        );
        let (actions, _) = actions.to_core_actions(
            &self.type_info,
            &mut Default::default(),
            &mut self.parser.symbol_gen,
        )?;
        eprintln!(
            "[DEBUG eval_actions] Converted to {} core actions",
            actions.0.len()
        );

        let mut translator = BackendRule::new(
            self.backend.new_rule("eval_actions", false),
            &self.functions,
            &self.type_info,
        );
        translator.actions(&actions)?;
        let id = translator.build();
        let result = self.backend.run_rules(&[id]);
        self.backend.free_rule(id);

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::BackendError(e.to_string())),
        }
    }

    /// Evaluates an expression, returns the sort of the expression and the evaluation result.
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<(ArcSort, Value), Error> {
        let span = expr.span();
        let command = Command::Action(Action::Expr(span.clone(), expr.clone()));
        let resolved_commands = self.process_command(command)?;
        assert_eq!(resolved_commands.len(), 1);
        let resolved_command = resolved_commands.into_iter().next().unwrap();
        let resolved_expr = match resolved_command {
            ResolvedNCommand::CoreAction(ResolvedAction::Expr(_, resolved_expr)) => resolved_expr,
            _ => unreachable!(),
        };
        let sort = resolved_expr.output_type();
        let value = self.eval_resolved_expr(span, &resolved_expr)?;
        Ok((sort, value))
    }

    fn eval_resolved_expr(&mut self, span: Span, expr: &ResolvedExpr) -> Result<Value, Error> {
        let unit_id = self.backend.base_values().get_ty::<()>();
        let unit_val = self.backend.base_values().get(());

        let result: egglog_bridge::SideChannel<Value> = Default::default();
        let result_ref = result.clone();
        let ext_id = self
            .backend
            .register_external_func(make_external_func(move |_es, vals| {
                debug_assert!(vals.len() == 1);
                *result_ref.lock().unwrap() = Some(vals[0]);
                Some(unit_val)
            }));

        let mut translator = BackendRule::new(
            self.backend.new_rule("eval_resolved_expr", false),
            &self.functions,
            &self.type_info,
        );

        let result_var = ResolvedVar {
            name: self.parser.symbol_gen.fresh("eval_resolved_expr"),
            sort: expr.output_type(),
            is_global_ref: false,
        };
        let actions = ResolvedActions::singleton(ResolvedAction::Let(
            span.clone(),
            result_var.clone(),
            expr.clone(),
        ));
        let actions = actions
            .to_core_actions(
                &self.type_info,
                &mut Default::default(),
                &mut self.parser.symbol_gen,
            )?
            .0;
        translator.actions(&actions)?;

        let arg = translator.entry(&ResolvedAtomTerm::Var(span.clone(), result_var));
        translator.rb.call_external_func(
            ext_id,
            &[arg],
            egglog_bridge::ColumnTy::Base(unit_id),
            || "this function will never panic".to_string(),
        );

        let id = translator.build();
        let rule_result = self.backend.run_rules(&[id]);
        self.backend.free_rule(id);
        self.backend.free_external_func(ext_id);
        let _ = rule_result.map_err(|e| {
            Error::BackendError(format!("Failed to evaluate expression '{}': {}", expr, e))
        })?;

        let result = result.lock().unwrap().unwrap();
        Ok(result)
    }

    fn add_combined_ruleset(&mut self, name: String, rulesets: Vec<String>) {
        match self.rulesets.entry(name.clone()) {
            Entry::Occupied(_) => panic!("Ruleset '{name}' was already present"),
            Entry::Vacant(e) => e.insert(Ruleset::Combined(rulesets)),
        };
    }

    fn add_ruleset(&mut self, name: String) {
        match self.rulesets.entry(name.clone()) {
            Entry::Occupied(_) => panic!("Ruleset '{name}' was already present"),
            Entry::Vacant(e) => e.insert(Ruleset::Rules(Default::default())),
        };
    }

    fn check_facts(&mut self, span: &Span, facts: &[ResolvedFact]) -> Result<(), Error> {
        // For each fact, verify it exists and is asserted (if fact-tracked)
        for fact in facts {
            // Extract the function call from the fact (if it's a simple Fact, not Eq)
            let func_name = match fact {
                GenericFact::Fact(expr) => match expr {
                    GenericExpr::Call(_, head, _) => match head {
                        ResolvedCall::Func(func_type) => Some(&func_type.name),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };

            // If we found a function name, check if it has fact tracking enabled
            if let Some(func_name) = func_name {
                eprintln!(
                    "[DEBUG CHECK] Checking fact-tracked function: {}",
                    func_name
                );
                // Get the function info
                if let Some(function) = self.functions.get(func_name) {
                    // Check if this function has fact tracking enabled
                    if function.decl.fact_tracking {
                        eprintln!("[DEBUG CHECK] Function has fact tracking enabled");
                        // Need to evaluate the fact arguments to get the actual values
                        // For now, we'll do a simpler check: run the query and verify matches are asserted

                        // Convert fact to a query
                        let fresh_name = self.parser.symbol_gen.fresh("check_fact");
                        let fresh_ruleset = self.parser.symbol_gen.fresh("check_fact_ruleset");
                        let rule = ast::ResolvedRule {
                            span: span.clone(),
                            head: ResolvedActions::default(),
                            body: vec![fact.clone()],
                            name: fresh_name.clone(),
                            ruleset: fresh_ruleset.clone(),
                        };
                        let core_rule = rule.to_canonicalized_core_rule(
                            &self.type_info,
                            &mut self.parser.symbol_gen,
                        )?;
                        let query = core_rule.body;

                        // Create a side channel to capture if query matched
                        let ext_sc = egglog_bridge::SideChannel::default();
                        let ext_sc_ref = ext_sc.clone();
                        let ext_id =
                            self.backend
                                .register_external_func(make_external_func(move |_, _| {
                                    *ext_sc_ref.lock().unwrap() = Some(());
                                    Some(Value::new_const(0))
                                }));

                        let mut translator = BackendRule::new(
                            self.backend.new_rule("check_fact", false),
                            &self.functions,
                            &self.type_info,
                        );
                        // For check on fact-tracked relations, exclude subsumed facts AND require assertion
                        translator.query(&query, false, true);
                        translator.rb.call_external_func(
                            ext_id,
                            &[],
                            egglog_bridge::ColumnTy::Id,
                            || "check will never panic".to_string(),
                        );
                        let id = translator.build();
                        let _ = self.backend.run_rules(&[id]).unwrap();
                        self.backend.free_rule(id);
                        self.backend.free_external_func(ext_id);

                        let ext_sc_val = ext_sc.lock().unwrap().take();
                        let matched = matches!(ext_sc_val, Some(()));

                        if !matched {
                            // Fact doesn't exist at all, check fails
                            return Err(Error::CheckError(
                                facts.iter().map(|f| f.clone().make_unresolved()).collect(),
                                span.clone(),
                            ));
                        }

                        // TODO: CRITICAL - Truth status verification not yet implemented
                        // The current implementation only checks IF a fact exists, but does NOT
                        // verify that it's asserted (truth_status=true). This means check will
                        // pass even for unasserted facts, which is incorrect.
                        //
                        // To implement proper truth status checking, we need to:
                        // 1. Add a `require_asserted` parameter to query_table() in egglog-bridge
                        //    (similar to how is_subsumed works)
                        // 2. Pass this parameter through add_atom_with_timestamp_and_func()
                        // 3. Have the query execution call should_include_row() with require_asserted=true
                        //
                        // The infrastructure exists (should_include_row, fact_truth_status HashMap)
                        // but it's not integrated into the query execution path yet.
                        //
                        // For now, this check will incorrectly pass for unasserted facts.
                    }
                }
            }
        }

        // If all fact-tracked facts passed, do the original check for non-fact-tracked facts
        let fresh_name = self.parser.symbol_gen.fresh("check_facts");
        let fresh_ruleset = self.parser.symbol_gen.fresh("check_facts_ruleset");
        let rule = ast::ResolvedRule {
            span: span.clone(),
            head: ResolvedActions::default(),
            body: facts.to_vec(),
            name: fresh_name.clone(),
            ruleset: fresh_ruleset.clone(),
        };
        let core_rule =
            rule.to_canonicalized_core_rule(&self.type_info, &mut self.parser.symbol_gen)?;
        let query = core_rule.body;

        let ext_sc = egglog_bridge::SideChannel::default();
        let ext_sc_ref = ext_sc.clone();
        let ext_id = self
            .backend
            .register_external_func(make_external_func(move |_, _| {
                *ext_sc_ref.lock().unwrap() = Some(());
                Some(Value::new_const(0))
            }));

        let mut translator = BackendRule::new(
            self.backend.new_rule("check_facts", false),
            &self.functions,
            &self.type_info,
        );
        // For check, we don't want to include subsumed facts
        // For fact-tracked relations, also require facts to be asserted
        translator.query(&query, false, true);
        translator
            .rb
            .call_external_func(ext_id, &[], egglog_bridge::ColumnTy::Id, || {
                "this function will never panic".to_string()
            });
        let id = translator.build();
        let _ = self.backend.run_rules(&[id]).unwrap();
        self.backend.free_rule(id);

        self.backend.free_external_func(ext_id);

        let ext_sc_val = ext_sc.lock().unwrap().take();
        let matched = matches!(ext_sc_val, Some(()));

        if !matched {
            Err(Error::CheckError(
                facts.iter().map(|f| f.clone().make_unresolved()).collect(),
                span.clone(),
            ))
        } else {
            Ok(())
        }
    }

    fn run_command(&mut self, command: ResolvedNCommand) -> Result<Option<CommandOutput>, Error> {
        match command {
            // Sorts are already declared during typechecking
            ResolvedNCommand::Sort(_span, name, _presort_and_args) => {
                log::info!("Declared sort {}.", name)
            }
            ResolvedNCommand::Function(fdecl) => {
                self.declare_function(&fdecl)?;
                log::info!("Declared {} {}.", fdecl.subtype, fdecl.name)
            }
            ResolvedNCommand::AddRuleset(_span, name) => {
                self.add_ruleset(name.clone());
                log::info!("Declared ruleset {name}.");
            }
            ResolvedNCommand::UnstableCombinedRuleset(_span, name, others) => {
                self.add_combined_ruleset(name.clone(), others);
                log::info!("Declared ruleset {name}.");
            }
            ResolvedNCommand::NormRule { rule } => {
                let name = rule.name.clone();
                self.add_rule(rule)?;
                log::info!("Declared rule {name}.")
            }
            ResolvedNCommand::RunSchedule(sched) => {
                let report = self.run_schedule(&sched)?;
                log::info!("Ran schedule {}.", sched);
                log::info!("Report: {}", report);
                self.overall_run_report.union(report.clone());
                return Ok(Some(CommandOutput::RunSchedule(report)));
            }
            ResolvedNCommand::PrintOverallStatistics(span, file) => match file {
                None => {
                    log::info!("Printed overall statistics");
                    return Ok(Some(CommandOutput::OverallStatistics(
                        self.overall_run_report.clone(),
                    )));
                }
                Some(path) => {
                    let mut file = std::fs::File::create(&path)
                        .map_err(|e| Error::IoError(path.clone().into(), e, span.clone()))?;
                    log::info!("Printed overall statistics to json file {}", path);

                    serde_json::to_writer(&mut file, &self.overall_run_report)
                        .expect("error serializing to json");
                }
            },
            ResolvedNCommand::Check(span, facts) => {
                self.check_facts(&span, &facts)?;
                log::info!("Checked fact {:?}.", facts);
            }
            ResolvedNCommand::CoreAction(action) => match &action {
                ResolvedAction::Let(_, name, contents) => {
                    panic!("Globals should have been desugared away: {name} = {contents}")
                }
                _ => {
                    self.eval_actions(&ResolvedActions::new(vec![action.clone()]))?;
                }
            },
            ResolvedNCommand::Extract(span, expr, variants) => {
                let sort = expr.output_type();

                let x = self.eval_resolved_expr(span.clone(), &expr)?;
                let n = self.eval_resolved_expr(span, &variants)?;
                let n: i64 = self.backend.base_values().unwrap(n);

                let mut termdag = TermDag::default();

                let extractor = Extractor::compute_costs_from_rootsorts(
                    Some(vec![sort]),
                    self,
                    TreeAdditiveCostModel::default(),
                );
                return if n == 0 {
                    if let Some((cost, term)) = extractor.extract_best(self, &mut termdag, x) {
                        // dont turn termdag into a string if we have messages disabled for performance reasons
                        if log_enabled!(Level::Info) {
                            log::info!("extracted with cost {cost}: {}", termdag.to_string(&term));
                        }
                        Ok(Some(CommandOutput::ExtractBest(termdag, cost, term)))
                    } else {
                        Err(Error::ExtractError(
                            "Unable to find any valid extraction (likely due to subsume or delete)"
                                .to_string(),
                        ))
                    }
                } else {
                    if n < 0 {
                        panic!("Cannot extract negative number of variants");
                    }
                    let terms: Vec<Term> = extractor
                        .extract_variants(self, &mut termdag, x, n as usize)
                        .iter()
                        .map(|e| e.1.clone())
                        .collect();
                    if log_enabled!(Level::Info) {
                        let expr_str = expr.to_string();
                        log::info!("extracted {} variants for {expr_str}", terms.len());
                    }
                    Ok(Some(CommandOutput::ExtractVariants(termdag, terms)))
                };
            }
            ResolvedNCommand::Push(n) => {
                (0..n).for_each(|_| self.push());
                log::info!("Pushed {n} levels.")
            }
            ResolvedNCommand::Pop(span, n) => {
                for _ in 0..n {
                    self.pop().map_err(|err| {
                        if let Error::Pop(_) = err {
                            Error::Pop(span.clone())
                        } else {
                            err
                        }
                    })?;
                }
                log::info!("Popped {n} levels.")
            }
            ResolvedNCommand::PrintFunction(span, f, n, file, mode) => {
                let file = file
                    .map(|file| {
                        std::fs::File::create(&file)
                            .map_err(|e| Error::IoError(file.into(), e, span.clone()))
                    })
                    .transpose()?;
                return self.print_function(&f, n, file, mode).map_err(|e| match e {
                    Error::TypeError(TypeError::UnboundFunction(f, _)) => {
                        Error::TypeError(TypeError::UnboundFunction(f, span.clone()))
                    }
                    // This case is currently impossible
                    _ => e,
                });
            }
            ResolvedNCommand::PrintSize(span, f) => {
                let res = self.print_size(f.as_deref()).map_err(|e| match e {
                    Error::TypeError(TypeError::UnboundFunction(f, _)) => {
                        Error::TypeError(TypeError::UnboundFunction(f, span.clone()))
                    }
                    // This case is currently impossible
                    _ => e,
                })?;
                return Ok(Some(res));
            }
            ResolvedNCommand::Fail(span, c) => {
                let result = self.run_command(*c);
                if let Err(e) = result {
                    log::info!("Command failed as expected: {e}");
                } else {
                    return Err(Error::ExpectFail(span));
                }
            }
            ResolvedNCommand::Input {
                span: _,
                name,
                file,
            } => {
                self.input_file(&name, file)?;
            }
            ResolvedNCommand::Output { span, file, exprs } => {
                let mut filename = self.fact_directory.clone().unwrap_or_default();
                filename.push(file.as_str());
                // append to file
                let mut f = File::options()
                    .append(true)
                    .create(true)
                    .open(&filename)
                    .map_err(|e| Error::IoError(filename.clone(), e, span.clone()))?;

                let extractor = Extractor::compute_costs_from_rootsorts(
                    None,
                    self,
                    TreeAdditiveCostModel::default(),
                );
                let mut termdag: TermDag = Default::default();

                use std::io::Write;
                for expr in exprs {
                    let value = self.eval_resolved_expr(span.clone(), &expr)?;
                    let expr_type = expr.output_type();

                    let term = extractor
                        .extract_best_with_sort(self, &mut termdag, value, expr_type)
                        .unwrap()
                        .1;
                    writeln!(f, "{}", termdag.to_string(&term))
                        .map_err(|e| Error::IoError(filename.clone(), e, span.clone()))?;
                }

                log::info!("Output to '{filename:?}'.")
            }
            ResolvedNCommand::UserDefined(_span, name, exprs) => {
                let command = self.commands.swap_remove(&name).unwrap_or_else(|| {
                    panic!("Unrecognized user-defined command: {}", name);
                });
                let res = command.update(self, &exprs);
                self.commands.insert(name, command);
                return res;
            }
        };

        Ok(None)
    }

    fn input_file(&mut self, func_name: &str, file: String) -> Result<(), Error> {
        let function_type = self
            .type_info
            .get_func_type(func_name)
            .unwrap_or_else(|| panic!("Unrecognized function name {}", func_name));
        let func = self.functions.get_mut(func_name).unwrap();

        let mut filename = self.fact_directory.clone().unwrap_or_default();
        filename.push(file.as_str());

        // check that the function uses supported types

        for t in &func.schema.input {
            match t.name() {
                "i64" | "f64" | "String" => {}
                s => panic!("Unsupported type {} for input", s),
            }
        }

        if function_type.subtype != FunctionSubtype::Constructor {
            match func.schema.output.name() {
                "i64" | "String" | "Unit" => {}
                s => panic!("Unsupported type {} for input", s),
            }
        }

        log::info!("Opening file '{:?}'...", filename);
        let mut f = File::open(filename).unwrap();
        let mut contents = String::new();
        f.read_to_string(&mut contents).unwrap();

        // Can also do a row-major Vec<Value>
        let mut parsed_contents: Vec<Vec<Value>> = Vec::with_capacity(contents.lines().count());

        let mut row_schema = func.schema.input.clone();
        if function_type.subtype == FunctionSubtype::Custom {
            row_schema.push(func.schema.output.clone());
        }

        log::debug!("{:?}", row_schema);

        let unit_val = self.backend.base_values().get(());

        for line in contents.lines() {
            let mut it = line.split('\t').map(|s| s.trim());

            let mut row: Vec<Value> = Vec::with_capacity(row_schema.len());

            for sort in row_schema.iter() {
                if let Some(raw) = it.next() {
                    let val = match sort.name() {
                        "i64" => {
                            if let Ok(i) = raw.parse::<i64>() {
                                self.backend.base_values().get(i)
                            } else {
                                return Err(Error::InputFileFormatError(file));
                            }
                        }
                        "f64" => {
                            if let Ok(f) = raw.parse::<f64>() {
                                self.backend
                                    .base_values()
                                    .get::<F>(core_relations::Boxed::new(f.into()))
                            } else {
                                return Err(Error::InputFileFormatError(file));
                            }
                        }
                        "String" => self.backend.base_values().get::<S>(raw.to_string().into()),
                        "Unit" => unit_val,
                        _ => panic!("Unreachable"),
                    };
                    row.push(val);
                } else {
                    break;
                }
            }

            if row.is_empty() {
                continue;
            }

            if row.len() != row_schema.len() || it.next().is_some() {
                return Err(Error::InputFileFormatError(file));
            }

            parsed_contents.push(row);
        }

        log::debug!("Successfully loaded file.");

        let num_facts = parsed_contents.len();

        let mut table_action = egglog_bridge::TableAction::new(&self.backend, func.backend_id);

        if function_type.subtype != FunctionSubtype::Constructor {
            self.backend.with_execution_state(|es| {
                for row in parsed_contents.iter() {
                    table_action.insert(es, row.iter().copied());
                }
                Some(unit_val)
            });
        } else {
            self.backend.with_execution_state(|es| {
                for row in parsed_contents.iter() {
                    table_action.lookup(es, row);
                }
                Some(unit_val)
            });
        }

        self.backend.flush_updates();

        log::info!("Read {num_facts} facts into {func_name} from '{file}'.");
        Ok(())
    }

    fn process_command(&mut self, command: Command) -> Result<Vec<ResolvedNCommand>, Error> {
        let mut program = self.resolve_command(command)?;

        program = remove_globals::remove_globals(program, &mut self.parser.symbol_gen);
        for command in &program {
            self.names.check_shadowing(command)?;
        }

        Ok(program)
    }

    fn resolve_command(&mut self, command: Command) -> Result<Vec<ResolvedNCommand>, Error> {
        let program = desugar::desugar_program(vec![command], &mut self.parser, self.seminaive)?;
        Ok(self.typecheck_program(&program)?)
    }

    /// Run a program, represented as an AST.
    /// Return a list of messages.
    pub fn run_program(&mut self, program: Vec<Command>) -> Result<Vec<CommandOutput>, Error> {
        let mut outputs = Vec::new();
        for command in program {
            // Important to process each command individually
            // because push and pop create new scopes
            for processed in self.process_command(command)? {
                let result = self.run_command(processed)?;
                if let Some(output) = result {
                    outputs.push(output);
                }
            }
        }

        Ok(outputs)
    }

    pub fn resugar_program(
        &mut self,
        filename: Option<String>,
        input: &str,
    ) -> Result<Vec<String>, Error> {
        let parsed = self.parser.get_program_from_string(filename, input)?;
        let mut outputs = Vec::new();
        for command in parsed {
            for processed in self.resolve_command(command)? {
                // When re-suggaring, we still need to run scope-related commands (Push/Pop) to make
                // the program well-scoped.
                if let GenericNCommand::Push(..) | GenericNCommand::Pop(..) = &processed {
                    self.run_command(processed.clone())?;
                }
                outputs.push(processed.to_command().to_string());
            }
        }
        Ok(outputs)
    }

    /// Takes a source program `input`, parses it, runs it, and returns a list of messages.
    ///
    /// `filename` is an optional argument to indicate the source of
    /// the program for error reporting. If `filename` is `None`,
    /// a default name will be used.
    pub fn parse_and_run_program(
        &mut self,
        filename: Option<String>,
        input: &str,
    ) -> Result<Vec<CommandOutput>, Error> {
        let parsed = self.parser.get_program_from_string(filename, input)?;
        self.run_program(parsed)
    }

    /// Get the number of tuples in the database.
    ///
    pub fn num_tuples(&self) -> usize {
        self.functions
            .values()
            .map(|f| self.backend.table_size(f.backend_id))
            .sum()
    }

    /// Returns a sort based on the type.
    pub fn get_sort<S: Sort>(&self) -> Arc<S> {
        self.type_info.get_sort()
    }

    /// Returns a sort that satisfies the type and predicate.
    pub fn get_sort_by<S: Sort>(&self, f: impl Fn(&Arc<S>) -> bool) -> Arc<S> {
        self.type_info.get_sort_by(f)
    }

    /// Returns all sorts based on the type.
    pub fn get_sorts<S: Sort>(&self) -> Vec<Arc<S>> {
        self.type_info.get_sorts()
    }

    /// Returns all sorts that satisfy the type and predicate.
    pub fn get_sorts_by<S: Sort>(&self, f: impl Fn(&Arc<S>) -> bool) -> Vec<Arc<S>> {
        self.type_info.get_sorts_by(f)
    }

    /// Returns a sort based on the predicate.
    pub fn get_arcsort_by(&self, f: impl Fn(&ArcSort) -> bool) -> ArcSort {
        self.type_info.get_arcsort_by(f)
    }

    /// Returns all sorts that satisfy the predicate.
    pub fn get_arcsorts_by(&self, f: impl Fn(&ArcSort) -> bool) -> Vec<ArcSort> {
        self.type_info.get_arcsorts_by(f)
    }

    /// Returns the sort with the given name if it exists.
    pub fn get_sort_by_name(&self, sym: &str) -> Option<&ArcSort> {
        self.type_info.get_sort_by_name(sym)
    }

    /// Gets the overall run report and returns it.
    pub fn get_overall_run_report(&self) -> &RunReport {
        &self.overall_run_report
    }

    /// Convert from an egglog value to a Rust type.
    pub fn value_to_base<T: BaseValue>(&self, x: Value) -> T {
        self.backend.base_values().unwrap::<T>(x)
    }

    /// Convert from a Rust type to an egglog value.
    pub fn base_to_value<T: BaseValue>(&self, x: T) -> Value {
        self.backend.base_values().get::<T>(x)
    }

    /// Convert from an egglog value to a reference of a Rust container type.
    ///
    /// Returns `None` if the value cannot be converted to the requested container type.
    ///
    /// Warning: The return type of this function may contain lock guards.
    /// Attempts to modify the contents of the containers database may deadlock if the given guard has not been dropped.
    pub fn value_to_container<T: ContainerValue>(
        &self,
        x: Value,
    ) -> Option<impl Deref<Target = T>> {
        self.backend.container_values().get_val::<T>(x)
    }

    /// Convert from a Rust container type to an egglog value.
    pub fn container_to_value<T: ContainerValue>(&mut self, x: T) -> Value {
        self.backend.with_execution_state(|state| {
            self.backend.container_values().register_val::<T>(x, state)
        })
    }

    /// Validate that a FactRef Value points to an existing, valid fact.
    ///
    /// Performs comprehensive validation including existence and consistency checks.
    pub fn validate_fact_ref(&self, fact_ref_val: Value) -> Result<(), Error> {
        self.backend
            .validate_fact_ref(fact_ref_val)
            .map_err(|e| Error::BackendError(e.to_string()))
    }

    /// Check if a fact reference is stale (points to deleted/non-existent fact).
    pub fn is_fact_ref_stale(&self, fact_ref_val: Value) -> bool {
        self.backend.is_fact_ref_stale(fact_ref_val)
    }

    /// Get the truth status of a fact, with validation.
    pub fn get_fact_truth_status(&self, fact_ref_val: Value) -> Result<Option<bool>, Error> {
        self.backend
            .get_fact_truth_status(fact_ref_val)
            .map_err(|e| Error::BackendError(e.to_string()))
    }

    /// Assert a fact with validation, returning detailed error information.
    pub fn assert_fact_validated(&mut self, fact_ref_val: Value) -> Result<bool, Error> {
        self.backend
            .assert_fact_validated(fact_ref_val)
            .map_err(|e| Error::BackendError(e.to_string()))
    }

    /// Retract a fact with validation, returning detailed error information.
    pub fn retract_fact_validated(&mut self, fact_ref_val: Value) -> Result<bool, Error> {
        self.backend
            .retract_fact_validated(fact_ref_val)
            .map_err(|e| Error::BackendError(e.to_string()))
    }

    /// Get the size of a function in the e-graph.
    ///
    /// `panics` if the function does not exist.
    pub fn get_size(&self, func: &str) -> usize {
        let function_id = self.functions.get(func).unwrap().backend_id;
        self.backend.table_size(function_id)
    }

    /// Lookup a tuple in afunction in the e-graph.
    ///
    /// Returns `None` if the tuple does not exist.
    /// `panics` if the function does not exist.
    pub fn lookup_function(&self, name: &str, key: &[Value]) -> Option<Value> {
        let func = self.functions.get(name).unwrap().backend_id;
        self.backend.lookup_id(func, key)
    }

    /// Get a function by name.
    ///
    /// Returns `None` if the function does not exist.
    pub fn get_function(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }

    pub fn set_report_level(&mut self, level: ReportLevel) {
        self.backend.set_report_level(level);
    }

    /// A basic method for dumping the state of the database to `log::info!`.
    ///
    /// For large tables, this is unlikely to give particularly useful output.
    pub fn dump_debug_info(&self) {
        self.backend.dump_debug_info();
    }

    /// Get the canonical representation for `val` based on type.
    pub fn get_canonical_value(&self, val: Value, sort: &ArcSort) -> Value {
        self.backend
            .get_canon_repr(val, sort.column_ty(&self.backend))
    }
}

struct BackendRule<'a> {
    rb: egglog_bridge::RuleBuilder<'a>,
    entries: HashMap<core::ResolvedAtomTerm, QueryEntry>,
    functions: &'a IndexMap<String, Function>,
    type_info: &'a TypeInfo,
}

impl<'a> BackendRule<'a> {
    fn new(
        rb: egglog_bridge::RuleBuilder<'a>,
        functions: &'a IndexMap<String, Function>,
        type_info: &'a TypeInfo,
    ) -> BackendRule<'a> {
        BackendRule {
            rb,
            functions,
            type_info,
            entries: Default::default(),
        }
    }

    fn entry(&mut self, x: &core::ResolvedAtomTerm) -> QueryEntry {
        self.entries
            .entry(x.clone())
            .or_insert_with(|| match x {
                core::GenericAtomTerm::Var(_, v) => self
                    .rb
                    .new_var_named(v.sort.column_ty(self.rb.egraph()), &v.name),
                core::GenericAtomTerm::Literal(_, l) => literal_to_entry(self.rb.egraph(), l),
                core::GenericAtomTerm::Global(..) => {
                    panic!("Globals should have been desugared")
                }
            })
            .clone()
    }

    fn func(&self, f: &typechecking::FuncType) -> egglog_bridge::FunctionId {
        self.functions[&f.name].backend_id
    }

    fn prim(
        &mut self,
        prim: &core::SpecializedPrimitive,
        args: &[core::ResolvedAtomTerm],
    ) -> (ExternalFunctionId, Vec<QueryEntry>, ColumnTy) {
        let mut qe_args = self.args(args);

        if prim.primitive.0.name() == "unstable-fn" {
            let core::ResolvedAtomTerm::Literal(_, Literal::String(ref name)) = args[0] else {
                panic!("expected string literal after `unstable-fn`")
            };
            let id = if let Some(f) = self.type_info.get_func_type(name) {
                ResolvedFunctionId::Lookup(egglog_bridge::TableAction::new(
                    self.rb.egraph(),
                    self.func(f),
                ))
            } else if let Some(possible) = self.type_info.get_prims(name) {
                let mut ps: Vec<_> = possible.iter().collect();
                ps.retain(|p| {
                    self.type_info
                        .get_sorts::<FunctionSort>()
                        .into_iter()
                        .any(|f| {
                            let types: Vec<_> = prim
                                .input
                                .iter()
                                .skip(1)
                                .chain(f.inputs())
                                .chain([&f.output()])
                                .cloned()
                                .collect();
                            p.accept(&types, self.type_info)
                        })
                });
                assert!(ps.len() == 1, "options for {name}: {ps:?}");
                ResolvedFunctionId::Prim(ps.into_iter().next().unwrap().1)
            } else {
                panic!("no callable for {name}");
            };
            let do_rebuild = prim
                .input
                .iter()
                .skip(1)
                .map(|s| s.is_eq_sort() || s.is_eq_container_sort())
                .collect();

            qe_args[0] = self.rb.egraph().base_value_constant(ResolvedFunction {
                id,
                do_rebuild,
                name: name.clone(),
            });
        }

        (
            prim.primitive.1,
            qe_args,
            prim.output.column_ty(self.rb.egraph()),
        )
    }

    fn args<'b>(
        &mut self,
        args: impl IntoIterator<Item = &'b core::ResolvedAtomTerm>,
    ) -> Vec<QueryEntry> {
        args.into_iter().map(|x| self.entry(x)).collect()
    }

    fn query(
        &mut self,
        query: &core::Query<ResolvedCall, ResolvedVar>,
        include_subsumed: bool,
        require_asserted: bool,
    ) {
        for atom in &query.atoms {
            match &atom.head {
                ResolvedCall::Func(f) => {
                    let func_id = self.func(f);
                    let args = self.args(&atom.args);
                    let is_subsumed = match include_subsumed {
                        true => None,
                        false => Some(false),
                    };

                    // Check if this function has fact tracking enabled
                    // If so, and require_asserted is true, filter by truth status
                    let function = &self.functions[&f.name];
                    let require_asserted_for_query =
                        if function.decl.fact_tracking && require_asserted {
                            Some(true)
                        } else {
                            None
                        };

                    self.rb
                        .query_table(func_id, &args, is_subsumed, require_asserted_for_query)
                        .unwrap();
                }
                ResolvedCall::Primitive(p) => {
                    let (p, args, ty) = self.prim(p, &atom.args);
                    self.rb.query_prim(p, &args, ty).unwrap()
                }
            }
        }
    }

    fn actions(&mut self, actions: &core::ResolvedCoreActions) -> Result<(), Error> {
        for action in &actions.0 {
            eprintln!(
                "[DEBUG actions] Action type: {}",
                match action {
                    core::GenericCoreAction::Let(..) => "Let",
                    core::GenericCoreAction::LetAtomTerm(..) => "LetAtomTerm",
                    core::GenericCoreAction::Set(..) => "Set",
                    core::GenericCoreAction::Change(..) => "Change",
                    core::GenericCoreAction::Union(..) => "Union",
                    core::GenericCoreAction::Panic(..) => "Panic",
                }
            );
            match action {
                core::GenericCoreAction::Let(span, v, f, args) => {
                    let v = core::GenericAtomTerm::Var(span.clone(), v.clone());
                    let y = match f {
                        ResolvedCall::Func(f) => {
                            let name = f.name.clone();
                            let f = self.func(f);
                            let args = self.args(args);
                            let span = span.clone();
                            self.rb.lookup(f, &args, move || {
                                format!("{span}: lookup of function {name} failed")
                            })
                        }
                        ResolvedCall::Primitive(p) => {
                            let name = p.primitive.0.name().to_owned();
                            let (p, args, ty) = self.prim(p, args);
                            let span = span.clone();
                            self.rb.call_external_func(p, &args, ty, move || {
                                format!("{span}: call of primitive {name} failed")
                            })
                        }
                    };
                    self.entries.insert(v, y.into());
                }
                core::GenericCoreAction::LetAtomTerm(span, v, x) => {
                    let v = core::GenericAtomTerm::Var(span.clone(), v.clone());
                    let x = self.entry(x);
                    self.entries.insert(v, x);
                }
                core::GenericCoreAction::Set(_, f, xs, y) => match f {
                    ResolvedCall::Primitive(..) => panic!("runtime primitive set!"),
                    ResolvedCall::Func(f) => {
                        eprintln!("[DEBUG ACTION] Set action for function: {}", f.name);
                        let f = self.func(f);
                        let args = self.args(xs.iter().chain([y]));
                        self.rb.set(f, &args)
                    }
                },
                core::GenericCoreAction::Change(span, change, f, args) => match f {
                    ResolvedCall::Primitive(..) => panic!("runtime primitive change!"),
                    ResolvedCall::Func(f) => {
                        let name = f.name.clone();
                        let can_subsume = self.functions[&f.name].can_subsume;
                        let f = self.func(f);
                        let args = self.args(args);
                        match change {
                            Change::Delete => self.rb.remove(f, &args),
                            Change::Subsume if can_subsume => self.rb.subsume(f, &args),
                            Change::Subsume => {
                                return Err(Error::SubsumeMergeError(name, span.clone()));
                            }
                        }
                    }
                },
                core::GenericCoreAction::Union(_, x, y) => {
                    let x = self.entry(x);
                    let y = self.entry(y);
                    self.rb.union(x, y)
                }
                core::GenericCoreAction::Panic(_, message) => self.rb.panic(message.clone()),
            }
        }
        Ok(())
    }

    fn build(self) -> egglog_bridge::RuleId {
        self.rb.build()
    }
}

fn literal_to_entry(egraph: &egglog_bridge::EGraph, l: &Literal) -> QueryEntry {
    match l {
        Literal::Int(x) => egraph.base_value_constant::<i64>(*x),
        Literal::Float(x) => egraph.base_value_constant::<sort::F>(x.into()),
        Literal::String(x) => egraph.base_value_constant::<sort::S>(sort::S::new(x.clone())),
        Literal::Bool(x) => egraph.base_value_constant::<bool>(*x),
        Literal::Unit => egraph.base_value_constant::<()>(()),
    }
}

fn literal_to_value(egraph: &egglog_bridge::EGraph, l: &Literal) -> Value {
    match l {
        Literal::Int(x) => egraph.base_values().get::<i64>(*x),
        Literal::Float(x) => egraph.base_values().get::<sort::F>(x.into()),
        Literal::String(x) => egraph.base_values().get::<sort::S>(sort::S::new(x.clone())),
        Literal::Bool(x) => egraph.base_values().get::<bool>(*x),
        Literal::Unit => egraph.base_values().get::<()>(()),
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ParseError(#[from] ParseError),
    #[error(transparent)]
    NotFoundError(#[from] NotFoundError),
    #[error(transparent)]
    TypeError(#[from] TypeError),
    #[error("Errors:\n{}", ListDisplay(.0, "\n"))]
    TypeErrors(Vec<TypeError>),
    #[error("{}\nCheck failed: \n{}", .1, ListDisplay(.0, "\n"))]
    CheckError(Vec<Fact>, Span),
    #[error("{1}\nNo such ruleset: {0}")]
    NoSuchRuleset(String, Span),
    #[error(
        "{1}\nAttempted to add a rule to combined ruleset {0}. Combined rulesets may only depend on other rulesets."
    )]
    CombinedRulesetError(String, Span),
    #[error("{0}")]
    BackendError(String),
    #[error("{0}\nTried to pop too much")]
    Pop(Span),
    #[error("{0}\nCommand should have failed.")]
    ExpectFail(Span),
    #[error("{2}\nIO error: {0}: {1}")]
    IoError(PathBuf, std::io::Error, Span),
    #[error("{1}\nCannot subsume function with merge: {0}")]
    SubsumeMergeError(String, Span),
    #[error("extraction failure: {:?}", .0)]
    ExtractError(String),
    #[error("{1}\n{2}\nShadowing is not allowed, but found {0}")]
    Shadowing(String, Span, Span),
    #[error("{1}\nCommand already exists: {0}")]
    CommandAlreadyExists(String, Span),
    #[error("Incorrect format in file '{0}'.")]
    InputFileFormatError(String),
}

#[cfg(test)]
mod tests {
    use crate::constraint::SimpleTypeConstraint;
    use crate::sort::*;
    use crate::*;

    #[derive(Clone)]
    struct InnerProduct {
        vec: ArcSort,
    }

    impl Primitive for InnerProduct {
        fn name(&self) -> &str {
            "inner-product"
        }

        fn get_type_constraints(&self, span: &Span) -> Box<dyn crate::constraint::TypeConstraint> {
            SimpleTypeConstraint::new(
                self.name(),
                vec![self.vec.clone(), self.vec.clone(), I64Sort.to_arcsort()],
                span.clone(),
            )
            .into_box()
        }

        fn apply(&self, exec_state: &mut ExecutionState<'_>, args: &[Value]) -> Option<Value> {
            let mut sum = 0;
            let vec1 = exec_state
                .container_values()
                .get_val::<VecContainer>(args[0])
                .unwrap();
            let vec2 = exec_state
                .container_values()
                .get_val::<VecContainer>(args[1])
                .unwrap();
            assert_eq!(vec1.data.len(), vec2.data.len());
            for (a, b) in vec1.data.iter().zip(vec2.data.iter()) {
                let a = exec_state.base_values().unwrap::<i64>(*a);
                let b = exec_state.base_values().unwrap::<i64>(*b);
                sum += a * b;
            }
            Some(exec_state.base_values().get::<i64>(sum))
        }
    }

    #[test]
    fn test_user_defined_primitive() {
        let mut egraph = EGraph::default();
        egraph
            .parse_and_run_program(None, "(sort IntVec (Vec i64))")
            .unwrap();

        let int_vec_sort = egraph.get_arcsort_by(|s| {
            s.value_type() == Some(std::any::TypeId::of::<VecContainer>())
                && s.inner_sorts()[0].name() == I64Sort.name()
        });

        egraph.add_primitive(InnerProduct { vec: int_vec_sort });

        egraph
            .parse_and_run_program(
                None,
                "
                (let a (vec-of 1 2 3 4 5 6))
                (let b (vec-of 6 5 4 3 2 1))
                (check (= (inner-product a b) 56))
            ",
            )
            .unwrap();
    }

    // Test that an `EGraph` is `Send` & `Sync`
    #[test]
    fn test_egraph_send_sync() {
        fn is_send<T: Send>(_t: &T) -> bool {
            true
        }
        fn is_sync<T: Sync>(_t: &T) -> bool {
            true
        }
        let egraph = EGraph::default();
        assert!(is_send(&egraph) && is_sync(&egraph));
    }

    fn get_function(egraph: &EGraph, name: &str) -> Function {
        egraph.functions.get(name).unwrap().clone()
    }

    fn get_value(egraph: &EGraph, name: &str) -> Value {
        let mut out = None;
        let id = get_function(egraph, name).backend_id;
        egraph.backend.for_each(id, |row| out = Some(row.vals[0]));
        out.unwrap()
    }

    #[test]
    fn test_subsumed_unextractable_rebuild_arg() {
        // Tests that a term stays unextractable even after a rebuild after a union would change the value of one of its args
        let mut egraph = EGraph::default();

        egraph
            .parse_and_run_program(
                None,
                r#"
                (datatype Math)
                (constructor container (Math) Math)
                (constructor exp () Math :cost 100)
                (constructor cheap () Math)
                (constructor cheap-1 () Math)
                ; we make the container cheap so that it will be extracted if possible, but then we mark it as subsumed
                ; so the (exp) expr should be extracted instead
                (let res (container (cheap)))
                (union res (exp))
                (cheap)
                (cheap-1)
                (subsume (container (cheap)))
                "#,
            ).unwrap();
        // At this point (cheap) and (cheap-1) should have different values, because they aren't unioned
        let orig_cheap_value = get_value(&egraph, "cheap");
        let orig_cheap_1_value = get_value(&egraph, "cheap-1");
        assert_ne!(orig_cheap_value, orig_cheap_1_value);
        // Then we can union them
        egraph
            .parse_and_run_program(
                None,
                r#"
                (union (cheap-1) (cheap))
                "#,
            )
            .unwrap();
        // And verify that their values are now the same and different from the original (cheap) value.
        let new_cheap_value = get_value(&egraph, "cheap");
        let new_cheap_1_value = get_value(&egraph, "cheap-1");
        assert_eq!(new_cheap_value, new_cheap_1_value);
        assert!(new_cheap_value != orig_cheap_value || new_cheap_1_value != orig_cheap_1_value);
        // Now verify that if we extract, it still respects the unextractable, even though it's a different values now
        let outputs = egraph
            .parse_and_run_program(
                None,
                r#"
                (extract res)
                "#,
            )
            .unwrap();
        assert_eq!(outputs[0].to_string(), "(exp)\n");
    }

    #[test]
    fn test_subsumed_unextractable_rebuild_self() {
        // Tests that a term stays unextractable even after a rebuild after a union change its output value.
        let mut egraph = EGraph::default();

        egraph
            .parse_and_run_program(
                None,
                r#"
                (datatype Math)
                (constructor container (Math) Math)
                (constructor exp () Math :cost 100)
                (constructor cheap () Math)
                (exp)
                (let x (cheap))
                (subsume (cheap))
                "#,
            )
            .unwrap();

        let orig_cheap_value = get_value(&egraph, "cheap");
        // Then we can union them
        egraph
            .parse_and_run_program(
                None,
                r#"
                (union (exp) x)
                "#,
            )
            .unwrap();
        // And verify that the cheap value is now different
        let new_cheap_value = get_value(&egraph, "cheap");
        assert_ne!(new_cheap_value, orig_cheap_value);

        // Now verify that if we extract, it still respects the subsumption, even though it's a different values now
        let res = egraph
            .parse_and_run_program(
                None,
                r#"
                (extract x)
                "#,
            )
            .unwrap();
        assert_eq!(res[0].to_string(), "(exp)\n");
    }
}
