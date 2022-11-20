mod inference_pass;
mod function_pass;
mod types;
mod typed_function;
#[cfg(test)]
mod inference_tests;
#[cfg(test)]
mod test;
mod builtin_func;
mod function;

pub use crate::typing::types::{TypedProgram, AnalysisResults};
pub use crate::typing::typed_function::TypedUserFunction;

use crate::parser::{Program};
use crate::printable_error::PrintableError;
use crate::typing::function_pass::FunctionAnalysis;
use crate::typing::inference_pass::variable_inference;

pub fn analyze(stmt: Program) -> Result<TypedProgram, PrintableError> {
    variable_inference(FunctionAnalysis::analyze(stmt)?)
}