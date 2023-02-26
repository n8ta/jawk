use std::ops::{Deref, DerefMut};
use hashbrown::HashMap;
use crate::awk_str::RcAwkStr;
#[cfg(test)]
use crate::vm::{VmProgram, VmFunc};
use crate::vm::{Code, StringScalar};

pub struct Chunk {
    bytecode: Vec<Code>,
}

impl Deref for Chunk {
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
        Self { bytecode: vec![] }
    }
    pub fn push(&mut self, code: Code) {
        self.bytecode.push(code);
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
                _ => {}
            }
        }
        let chunk_len = self.bytecode.len();
        for (idx, byte) in self.bytecode.iter_mut().enumerate() {
            let lbl = match byte {
                Code::JumpLbl(lbl) => lbl,
                Code::JumpIfFalseNumLbl(lbl) => lbl,
                Code::JumpIfFalseStrLbl(lbl) => lbl,
                Code::JumpIfFalseVarLbl(lbl) => lbl,
                Code::JumpIfTrueNumLbl(lbl) => lbl,
                Code::JumpIfTrueStrLbl(lbl) => lbl,
                Code::JumpIfTrueVarLbl(lbl) => lbl,
                Code::JumpIfTrueNextLineLbl(lbl) => lbl,
                Code::JumpIfFalseNextLineLbl(lbl) => lbl,
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
            let offset = label_idx - (idx as isize);
            byte.resolve_label_to_offset(offset)
        }
    }

    pub fn optimize(&mut self) {

        // Optimize concat to clear the destination scalar before concat'ing.
        // This allows RcAwkStr's to be downgraded for efficient extension
        let mut new_code = vec![];
        for pair in self.bytecode.windows(2) {
            if let Code::Concat { .. } = pair[0] {
                if let Code::AssignGsclStr(id) = pair[1] {
                    new_code.push(Code::ClearGscl(id));
                }
                if let Code::AssignArgStr { arg_idx } = pair[1] {
                    new_code.push(Code::ClearArgScl(arg_idx));
                }
            }
            new_code.push(pair[0].clone());
        }
        new_code.push(self.bytecode.last().unwrap().clone());
        self.bytecode = new_code;
    }

    #[cfg(test)]
    pub fn pretty_print(&self, _func: &VmFunc, prog: &VmProgram, output: &mut Vec<u8>) {
        output.extend_from_slice("Bytecode:\n".as_bytes());
        for (idx, byte) in self.bytecode.iter().enumerate() {
            let ip = format!("\t{:2} ", idx);
            output.extend_from_slice(&ip.as_bytes());

            byte.pretty_print(output);

            let meta = byte.meta(&prog.func_map);
            let side_effect = format!("{:?}\n", meta);
            output.extend_from_slice(&side_effect.as_bytes());
        }
    }
}
