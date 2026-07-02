//! FlatZinc subset parser and compiler for Propaga.
//!
//! # Example
//!
//! ```
//! use propaga_flatzinc::{compile, parse};
//!
//! let source = r#"
//!     var 1..3: x;
//!     constraint int_eq(x, 2);
//!     solve satisfy;
//! "#;
//! let program = parse(source).expect("valid FlatZinc");
//! let mut instance = compile(program).expect("supported constraints");
//! let (solution, _stats) = instance.model.solve_subset_with_stats(instance.solve_vars);
//! assert!(solution.is_some());
//! ```

mod compile;
mod error;
mod parse;

pub use compile::{AnnotationSearchConfig, CompiledInstance, ObjectiveSpec, compile};
pub use error::FlatZincError;
pub use parse::{
    Constraint, DurationSpec, Expr, FlatZincProgram, IntSearchAnnotation, OutputDirective,
    OutputSegment, ParamDecl, PredicateDecl, RestartAnnotation, RestartKind, SearchAnnotations,
    SolveDirective, SolveGoal, VarDecl, parse,
};
