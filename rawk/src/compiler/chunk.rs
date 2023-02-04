use std::ops::{Deref, DerefMut};
use hashbrown::HashMap;
use crate::awk_str::RcAwkStr;
#[cfg(test)]
use crate::vm::{VmProgram, VmFunc};
use crate::vm::{Code, StringScalar};

pub struct Chunk {
    str_consts: Vec<StringScalar>,
    num_consts: Vec<f64>,
    bytecode: Vec<Code>,
}

impl Deref  for Chunk {
    type Target = Vec<Code>;
    fn deref(&self) -> &Self::Target {
        &self.bytecode
    }
}
impl DerefMut for Chunk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytecode
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self { bytecode: vec![], num_consts: vec![], str_consts: vec![] }
    }
    pub fn push(&mut self, code: Code) {
        self.bytecode.push(code);
    }
    pub fn add_const_float(&mut self, flt: f64) -> u16 {
        Self::add_const(flt, &mut self.num_consts)
    }
    pub fn add_const_str(&mut self, str: RcAwkStr) -> u16 {
        Self::add_const(StringScalar::Str(str), &mut self.str_consts)
    }
    pub fn add_const_strnum(&mut self, str: RcAwkStr) -> u16 {
        Self::add_const(StringScalar::StrNum(str), &mut self.str_consts)
    }
    pub fn get_const_float(&self, idx: u16) -> f64 {
        self.num_consts[idx as usize].clone()
    }
    pub fn get_const_str(&self, idx: u16) -> StringScalar {
        self.str_consts[idx as usize].clone()
    }
    fn add_const<T: PartialEq>(val: T, constants: &mut Vec<T>) -> u16 {
        let idx = if let Some((idx, _constant)) = constants.iter().enumerate().find(|(_idx, constant)| **constant == val) {
            idx
        } else {
            constants.push(val);
            constants.len() - 1
        };
        if idx > u16::MAX as usize {
            // TODO: u16 max
            panic!("More than u16::MAX constants")
        }
        idx as u16
    }

    pub fn resolve_labels(&mut self) {
        let mut label_indices = HashMap::new();
        for (idx, byte) in self.bytecode.iter_mut().enumerate() {
            match byte {
                Code::Label(lbl) => {
                    label_indices.insert(*lbl, idx);
                    let mut nop = Code::NoOp;
                    std::mem::swap(byte, &mut nop);
                }
                _ => {},
            }
        }
        let chunk_len = self.bytecode.len();
        for (idx, byte) in self.bytecode.iter_mut().enumerate() {
            let lbl = match byte {
                Code::JumpIfFalseLbl(lbl) => lbl,
                Code::JumpLbl(lbl) => lbl,
                Code::JumpIfTrueLbl(lbl) => lbl,
                _ => continue,
            };
            let mut label_idx = *label_indices.get(lbl).unwrap();
            // TODO: Restore this
            if label_idx + 1 < chunk_len {
                // As long as the jump isn't to the end of the program there's no
                // need to jump to the no-op itself, instead jump to the next actual op.
                label_idx += 1;
            }
            let label_idx = label_idx as isize;
            let offset = label_idx - (idx as isize) ;
            if offset > i16::MAX as isize {
                panic!("todo handle long jumps");
            }
            byte.resolve_label_to_offset(offset as i16)
        }
    }

    #[cfg(test)]
    pub fn pretty_print(&self, func: &VmFunc, prog: &VmProgram, output: &mut String) {
        output.push_str("Bytecode:\n");
        for (idx, byte) in self.bytecode.iter().enumerate() {
            let ip = format!("\t{:2} ", idx);
            output.push_str(&ip);

            byte.pretty_print(output);

            let meta = byte.meta(&prog.func_map);
            let side_effect = format!("{:?}\n",  meta);
            output.push_str(&side_effect);
        }
        let consts = format!("\nConsts:\n{:?}\t{:?}",self.str_consts, self.num_consts);
        output.push_str(&consts);
    }
}
