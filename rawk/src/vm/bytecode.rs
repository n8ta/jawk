use std::fmt::{Debug, Formatter};
use crate::typing::{GlobalArrayId, GlobalScalarId};
use crate::vm::VmProgram;

pub type LabelId = u16;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Label {
    id: LabelId,
}

impl Label {
    pub fn new(id: LabelId) -> Self {
        Self { id }
    }
}

#[derive(Debug, PartialEq)]
pub enum Code {
    FloatZero,
    FloatOne,

    Pop,

    Column,

    NextLine,

    GSclAssign(GlobalScalarId),

    GScl(GlobalScalarId),

    ArgSclAsgn { arg_idx: u16 },

    ArgScl { arg_idx: u16 },

    Exp,

    Mult,
    Div,
    Mod,
    Add,
    Minus,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEq,
    Neq,
    Matches,
    NMatches,

    Concat { count: u16 },

    GlobalArr(GlobalArrayId),
    ArgArray { arg_idx: u16 },

    ArrayMember { indices: u16 },
    ArrayAssign { indices: u16 },

    ArrayIndex { indices: u16 },

    Call { target: u16 },

    Print,

    Printf { num_args: u16 }, // excluding fstring

    NoOp,

    Ret,

    // ConstI16(i16), // TODO: float which is exactly representable as an i16?
    // Index in constant table
    ConstLkp { idx: u16 },

    // BEGIN BUILTINS FUNCS
    BuiltinAtan2,
    BuiltinCos,
    BuiltinExp,
    BuiltinSubstr2,
    BuiltinSubstr3,
    BuiltinIndex,
    BuiltinInt,
    BuiltinLength0,
    BuiltinLength1,
    BuiltinLog,
    BuiltinRand,
    BuiltinSin,
    BuiltinSplit2,
    BuiltinSplit3,
    BuiltinSqrt,
    BuiltinSrand0,
    BuiltinSrand1,
    BuiltinTolower,
    BuiltinToupper,
    // END

    // Sub and gsub are paired with an assign code depending on what is being assigned to.
    // Pushes two scalars first the number of replacements, second the output string.
    Sub { global: bool},

    // These will be transformed before reaching VM
    JumpIfFalseLbl(Label),
    JumpLbl(Label),
    JumpIfTrueLbl(Label),
    Label(Label), // n/a

    // Transformed into these
    RelJumpIfFalse { offset: i16 },
    RelJumpIfTrue { offset: i16 },
    RelJump { offset: i16 }, // n/a
}

impl Code {
    pub fn resolve_label_to_offset(&mut self, offset: i16) {
        let mut replacement_jump = match self {
            Code::JumpIfFalseLbl(_) => { Self::RelJumpIfFalse { offset } }
            Code::JumpLbl(_) => { Self::RelJump { offset } }
            Code::JumpIfTrueLbl(_) => { Self::RelJumpIfTrue { offset } }
            _ => return,
        };
        // Replace a jump to a label with a rel jump with an offset
        std::mem::swap(self, &mut replacement_jump);
    }
}


// Side effects are useful for ensuring program correctness
// with respect to stack additions and removals


#[cfg(test)]
pub struct SideEffect {
    // scalar stack additions/removals
    pub ss_add: usize,
    pub ss_rem: usize,

    // array stack additions/removals
    pub as_add: usize,
    pub as_rem: usize,

    // is return code
    pub is_ret: bool,

    // Relative offsets to descendants (1 for all except ret and jump)
    pub descendant_offsets: Vec<isize>,
}

#[cfg(test)]
impl Debug for SideEffect {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ss: +{} -{}\tas: +{} -{}", self.ss_add, self.ss_rem, self.as_add, self.as_rem)
    }
}

#[cfg(test)]
impl SideEffect {
    pub fn new(ss_add: usize, ss_rem: usize, as_add: usize, as_rem: usize) -> Self {
        Self { descendant_offsets: vec![1], is_ret: false, ss_add, ss_rem, as_add, as_rem }
    }
    pub fn new_jump(descendant_offsets: Vec<isize>) -> Self {
        Self {
            descendant_offsets, is_ret: false, ss_add: 0, ss_rem: 0, as_add: 0, as_rem: 0,
        }
    }
    pub fn new_ret() -> Self {
        Self {
            descendant_offsets: vec![], is_ret: true, ss_add: 0, ss_rem: 0, as_add: 0, as_rem: 0,
        }
    }
}

#[cfg(test)]
impl Code {
    pub fn pretty_print(&self, output: &mut String) {
        let mut byte_padded = format!("{:?}", self);
        while byte_padded.len() < 50 {
            byte_padded.push(' ');
        }
        output.push_str(&byte_padded);
    }
    pub fn pretty_print_owned(&self) -> String {
        let mut s = String::new();
        self.pretty_print(&mut s);
        s
    }
    pub fn side_effect(&self, program: &VmProgram) -> SideEffect {
        match self {
            Code::FloatZero => SideEffect::new(1, 0, 0, 0),
            Code::FloatOne => SideEffect::new(1, 0, 0, 0),
            Code::Pop => SideEffect::new(0, 1, 0, 0),
            Code::Column => SideEffect::new(1, 1, 0, 0),
            Code::NextLine => SideEffect::new(1, 0, 0, 0),
            Code::GSclAssign(_) => SideEffect::new(1, 1, 0, 0),
            Code::GScl(_) => SideEffect::new(1, 0, 0, 0),
            Code::ArgSclAsgn { .. } => SideEffect::new(1, 1, 0, 0),
            Code::ArgScl { .. } => SideEffect::new(1, 0, 0, 0),
            Code::Exp => SideEffect::new(1, 2, 0, 0),
            Code::Mult => SideEffect::new(1, 2, 0, 0),
            Code::Div => SideEffect::new(1, 2, 0, 0),
            Code::Mod => SideEffect::new(1, 2, 0, 0),
            Code::Add => SideEffect::new(1, 2, 0, 0),
            Code::Minus => SideEffect::new(1, 2, 0, 0),
            Code::Lt => SideEffect::new(1, 2, 0, 0),
            Code::Gt => SideEffect::new(1, 2, 0, 0),
            Code::LtEq => SideEffect::new(1, 2, 0, 0),
            Code::GtEq => SideEffect::new(1, 2, 0, 0),
            Code::EqEq => SideEffect::new(1, 2, 0, 0),
            Code::Neq => SideEffect::new(1, 2, 0, 0),
            Code::Matches => SideEffect::new(1, 2, 0, 0),
            Code::NMatches => SideEffect::new(1, 2, 0, 0),
            Code::Concat { count } => SideEffect::new(1, *count as usize, 0, 0),
            Code::GlobalArr(_) => SideEffect::new(0, 0, 1, 0),
            Code::ArgArray { .. } => SideEffect::new(0, 0, 1, 0),
            Code::ArrayMember { indices } => SideEffect::new(1, *indices as usize, 0, 1),
            Code::ArrayAssign { indices } => SideEffect::new(1, *indices as usize, 0, 1),
            Code::ArrayIndex { indices } => SideEffect::new(1, *indices as usize, 0, 1),
            Code::Call { target } => {
                let target = &program.functions[*target as usize];
                SideEffect::new(1, target.num_scalar_args(), 0, target.num_array_args())
            }
            Code::Print => SideEffect::new(0, 1, 0, 0),
            Code::Printf { .. } => todo!("printf bytecode side effect"),
            Code::NoOp => SideEffect::new(0, 0, 0, 0),
            Code::Ret => SideEffect::new_ret(),
            Code::ConstLkp { .. } => SideEffect::new(1, 0, 0, 0),
            Code::BuiltinAtan2 => SideEffect::new(1, 2, 0, 0),
            Code::BuiltinCos => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinExp => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinSubstr2 => SideEffect::new(1, 2, 0, 0),
            Code::BuiltinSubstr3 => SideEffect::new(1, 3, 0, 0),
            Code::BuiltinIndex => SideEffect::new(1, 2, 0, 0),
            Code::BuiltinInt => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinLength0 => SideEffect::new(1, 0, 0, 0),
            Code::BuiltinLength1 => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinRand => SideEffect::new(1, 0, 0, 0),
            Code::BuiltinLog => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinSin => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinSplit2 => SideEffect::new(1, 2, 0, 1),
            Code::BuiltinSplit3 => SideEffect::new(1, 3, 0, 1),
            Code::BuiltinSqrt => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinSrand0 => SideEffect::new(1, 0, 0, 0),
            Code::BuiltinSrand1 => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinTolower => SideEffect::new(1, 1, 0, 0),
            Code::BuiltinToupper => SideEffect::new(1, 1, 0, 0),
            Code::Sub { .. } => SideEffect::new(2, 3, 0, 0),
            Code::JumpIfFalseLbl(_) => SideEffect::new(0, 0, 0, 0),
            Code::JumpLbl(_) => SideEffect::new(0, 0, 0, 0),
            Code::JumpIfTrueLbl(_) => SideEffect::new(0, 0, 0, 0),
            Code::Label(_) => SideEffect::new(0, 0, 0, 0),
            Code::RelJumpIfFalse { offset } => SideEffect::new_jump(vec![*offset as isize, 1]),
            Code::RelJumpIfTrue { offset } => SideEffect::new_jump(vec![*offset as isize, 1]),
            Code::RelJump { offset } => SideEffect::new_jump(vec![*offset as isize]),
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::vm::Code;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<Code>(), 4);
    }
}