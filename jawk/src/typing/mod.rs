mod builtin_funcs;
mod ityped_function;
mod passes;
mod reconcile;
mod structs;
#[cfg(test)]
mod tests;

pub use builtin_funcs::BuiltinFunc;
pub use structs::{
    AnalysisResults, FunctionMap, ITypedFunction, MapT, TypedProgram, TypedUserFunction,
};

use crate::parser::Program;
use crate::printable_error::PrintableError;
use passes::{function_pass, inference_pass};

pub fn analyze(stmt: Program) -> Result<TypedProgram, PrintableError> {
    inference_pass(function_pass(stmt)?)
}
