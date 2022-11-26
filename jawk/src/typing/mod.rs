mod analysis_results;
mod passes;
mod structs;
mod builtin_funcs;
#[cfg(test)]
mod tests;
mod ityped_function;
mod reconcile;

pub use analysis_results::AnalysisResults;
pub use structs::{FunctionMap, ITypedFunction, TypedProgram, TypedUserFunction};
pub use builtin_funcs::BuiltinFunc;

use passes::{function_pass, inference_pass};
use crate::parser::Program;
use crate::printable_error::PrintableError;

pub fn analyze(stmt: Program) -> Result<TypedProgram, PrintableError> {
    inference_pass(function_pass(stmt)?)
}
