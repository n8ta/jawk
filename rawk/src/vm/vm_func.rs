use std::ops::Deref;
use std::rc::Rc;
use crate::awk_str::AwkStr;
use crate::typing::{ITypedFunction, TypedUserFunction};
use crate::vm::Code;

pub struct Chunk {
    floats: Vec<f64>,
    strings: Vec<Rc<AwkStr>>,
    bytecode: Vec<Code>,
}

impl Deref  for Chunk {
    type Target = Vec<Code>;

    fn deref(&self) -> &Self::Target {
        &self.bytecode
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self { floats: vec![], bytecode: vec![], strings: vec![] }
    }
    pub fn push(&mut self, code: Code) {
        self.bytecode.push(code);
    }
    pub fn get_const_float(&mut self, flt: f64) -> u16 {
        let idx = if let Some((idx, _float)) = self.floats.iter().enumerate().find(|(_idx, const_flt)| **const_flt == flt) {
            idx
        } else {
            self.floats.push(flt);
            self.floats.len() - 1
        };
        if idx > u16::MAX as usize {
            // TODO: u16 max
            panic!("More than u16::MAX float constants")
        }
        idx as u16
    }
    pub fn get_const_str(&mut self, str: Rc<AwkStr>) -> u16 {
        let idx = if let Some((idx, _float)) = self.strings.iter().enumerate().find(|(_idx, const_str)| **const_str == str) {
            idx
        } else {
            self.strings.push(str);
            self.strings.len() - 1
        };
        if idx > u16::MAX as usize {
            // TODO: u16 max
            panic!("More than u16::MAX string constants")
        }
        idx as u16
    }
}

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
}
impl Deref for VmFunc {
    type Target = Chunk;

    fn deref(&self) -> &Self::Target {
        &self.chunk
    }
}