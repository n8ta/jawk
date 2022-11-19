mod inference_pass;
mod function_pass;
mod types;
mod typed_function;
#[cfg(test)]
mod inference_tests;
#[cfg(test)]
mod test;
mod ityped_function;
mod native_func;

pub use crate::typing::types::{TypedProgram, AnalysisResults};
pub use crate::typing::typed_function::TypedUserFunction;
pub use crate::typing::ityped_function::ITypedFunction;

use crate::parser::{Program};
use crate::printable_error::PrintableError;
use crate::typing::function_pass::FunctionAnalysis;
use crate::typing::inference_pass::variable_inference;

pub fn analyze(stmt: Program) -> Result<TypedProgram, PrintableError> {
    let func_analysis = FunctionAnalysis::new();
    let typed_program = variable_inference(func_analysis.analyze_program(stmt)?)?;
    Ok(typed_program)
}