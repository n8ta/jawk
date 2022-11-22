mod types;
mod passes;
mod structs;
#[cfg(test)]
mod tests;

pub use crate::typing::types::{AnalysisResults, TypedProgram};
pub use structs::TypedUserFunction;
pub use structs::ITypedFunction;
pub use crate::typing::structs::{CallInfo, CallLink};
pub(crate) use crate::typing::passes::{function_pass, inference_pass};

use crate::parser::Program;
use crate::printable_error::PrintableError;

pub fn analyze(stmt: Program) -> Result<TypedProgram, PrintableError> {
    inference_pass(function_pass(stmt)?)
}