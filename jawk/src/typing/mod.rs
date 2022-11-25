mod types;
mod passes;
mod structs;
#[cfg(test)]
mod tests;

pub use crate::typing::types::{AnalysisResults};
pub use structs::{ITypedFunction, FunctionMap, TypedUserFunction, TypedProgram, CallLink, BuiltinFunc};

use passes::{function_pass, inference_pass};
use crate::parser::Program;
use crate::printable_error::PrintableError;

pub fn analyze(stmt: Program) -> Result<TypedProgram, PrintableError> {
    inference_pass(function_pass(stmt)?)
}