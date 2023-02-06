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

pub fn compile(mut program: TypedProgram) -> Result<VmProgram, PrintableError> {
    let mut functions = vec![];

    // TODO avoid this clone
    let funcs: Vec<(Symbol, Rc<TypedUserFunction>)> = program.functions
        .user_functions_iter()
        .map(|(name,func)| (name.clone(), func.clone())).
        collect();
    for (_name, function) in funcs {
        let compiler = FunctionCompiler::new(&mut program, function.clone());
        functions.push(compiler.compile()?);
    }
    let prog = VmProgram::new(functions, program.global_analysis, program.functions);
    Ok(prog)
}