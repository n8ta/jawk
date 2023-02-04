use std::ops::Deref;
use std::rc::Rc;
use crate::typing::{ITypedFunction, TypedUserFunction};
use crate::compiler::Chunk;
#[cfg(test)]
use crate::symbolizer::Symbol;
#[cfg(test)]
use crate::vm::{VmProgram};

pub struct VmFunc {
    chunk: Chunk,
    parser_func: Rc<TypedUserFunction>,
    id: u16,
}

impl VmFunc {
    pub fn new(chunk: Chunk, id: u16, func: Rc<TypedUserFunction>) -> Self {
        Self { chunk, parser_func: func, id }
    }
    pub fn is_main(&self) -> bool {
        self.parser_func.name().sym.as_str() == "main function"
    }


    pub fn num_scalar_args(&self) -> usize {
        self.parser_func.num_scalar_args()
    }
    pub fn num_array_args(&self) -> usize {
        self.parser_func.num_array_args()
    }

    #[cfg(test)]
    pub fn chunk(&self) -> &Chunk {
        &self.chunk
    }
    #[cfg(test)]
    pub fn name(&self) -> Symbol {
        self.parser_func.name()
    }
    #[cfg(test)]
    pub fn pretty_print(&self, func: &VmFunc, prog: &VmProgram, output: &mut String) {
        let tmp = format!("{} {} \n", self.parser_func.name(), &self.id);
        output.push_str(&tmp);
        self.chunk.pretty_print(func, prog, output)
    }
}

impl Deref for VmFunc {
    type Target = Chunk;

    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}