use std::rc::Rc;
use hashbrown::HashMap;
use crate::compiler::function_compiler::FunctionCompiler;
use crate::printable_error::PrintableError;
use crate::symbolizer::Symbol;
use crate::typing::{TypedProgram, TypedUserFunction};
use crate::vm::VmProgram;

mod function_compiler;
mod chunk;
pub use chunk::Chunk;


#[cfg(test)]
mod program_validator;
#[cfg(test)]
pub use crate::compiler::program_validator::validate_program;

type FunctionIdMap = HashMap<Symbol, (u16, Rc<TypedUserFunction>)>;

pub fn compile(program: TypedProgram) -> Result<VmProgram, PrintableError> {
    // Maps function symbols to their identifier u16
    let mut function_mapping = HashMap::new();
    for (idx, (name, function)) in program.functions.user_functions_iter().enumerate() {
        function_mapping.insert(name.clone(), (idx as u16, function.clone()));
    }

    let mut functions = vec![];
    for (_name, function) in program.functions.user_functions_iter() {
        let compiler = FunctionCompiler::new(&function_mapping, &program.global_analysis, function.clone());
        functions.push(compiler.compile()?);
    }
    let prog = VmProgram::new(functions, program.global_analysis);

    Ok(prog)
}