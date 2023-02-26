use std::io::Write;
use crate::awk_str::{AwkByteStr, RcAwkStr};
use crate::specials::SclSpecial;
use crate::typing::{GlobalArrayId, GlobalScalarId};
use crate::vm::VirtualMachine;

type ByteFn = fn(&mut VirtualMachine, usize, Immed) -> usize;

#[derive(Copy, Clone)]
pub union Immed {
    pub num: f64,
    pub global_scl_id: GlobalScalarId,
    pub global_arr_id: GlobalArrayId,
    pub offset: isize,
    pub arg_idx: usize,
    pub concat_count: usize,
    pub array_indices: usize,
    pub sub3_isglobal: bool,
    pub printf_args: usize,
    pub call_target: usize,
    pub string: *const AwkByteStr,
    pub special: SclSpecial,
}

#[derive(Copy, Clone)]
pub struct CodeAndImmed {
    pub code: ByteFn,
    pub imm: Immed,
}

impl CodeAndImmed {
    pub fn new(code: ByteFn) -> Self {
        Self {
            code,
            imm: Immed { num: 0.0 },
        }
    }
    pub fn imm(code: ByteFn, imm: Immed) -> Self {
        Self {
            code,
            imm,
        }
    }
}

#[test]
fn test_imm_size() {
    let size = std::mem::size_of::<Immed>();
    println!("{}", size);
    assert!(size <= 8);
}