use std::ops::Deref;
use std::rc::Rc;
use crate::typing::{ITypedFunction, TypedUserFunction};
use crate::compiler::Chunk;
#[cfg(test)]
use crate::symbolizer::Symbol;
#[cfg(test)]
use crate::vm::{VmProgram};
use crate::vm::bytecode::CodeAndImmed;

pub struct VmFunc {
    chunk: Chunk,
    bytecode: Vec<CodeAndImmed>,
    parser_func: Rc<TypedUserFunction>,
    id: usize,
}

impl VmFunc {
    pub fn new(chunk: Chunk, id: usize, func: Rc<TypedUserFunction>) -> Self {
        let bytecode: Vec<CodeAndImmed> = chunk.iter().map(|code| code.transform()).collect();
        Self { chunk, bytecode, parser_func: func, id }
    }
    pub fn is_main(&self) -> bool {
        self.parser_func.name().sym.as_str() == "main function"
    }

    pub fn id(&self) -> usize {
        self.id
    }


    pub fn num_scalar_args(&self) -> usize {
        self.parser_func.num_scalar_args()
    }
    pub fn num_array_args(&self) -> usize {
        self.parser_func.num_array_args()
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }

    #[cfg(test)]
    pub fn name(&self) -> Symbol {
        self.parser_func.name()
    }
    #[cfg(test)]
    pub fn pretty_print(&self, func: &VmFunc, prog: &VmProgram, output: &mut Vec<u8>) {
        self.chunk.pretty_print(func, prog, output)
    }
}

impl Deref for VmFunc {
    type Target = Vec<CodeAndImmed>;

    fn deref(&self) -> &Self::Target {
        &self.bytecode
    }
}