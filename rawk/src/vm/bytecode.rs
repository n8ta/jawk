use crate::typing::{FunctionMap, GlobalArrayId, GlobalScalarId, ITypedFunction};

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
    NumToVar,
    NumToStr,
    StrToVar,
    StrToNum,
    VarToNum,
    VarToStr,

    FloatZero,
    FloatOne,

    Pop,
    PopStr,
    PopNum,

    Column,

    NextLine,

    AssignGsclVar(GlobalScalarId),
    AssignGsclNum(GlobalScalarId),
    AssignGsclStr(GlobalScalarId),

    AssignRetGsclVar(GlobalScalarId),
    AssignRetGsclNum(GlobalScalarId),
    AssignRetGsclStr(GlobalScalarId),

    GlobalArr(GlobalArrayId),
    GsclVar(GlobalScalarId),
    GsclNum(GlobalScalarId),
    GsclStr(GlobalScalarId),

    AssignArgVar { arg_idx: u16 },
    AssignArgStr { arg_idx: u16 },
    AssignArgNum { arg_idx: u16 },

    AssignRetArgVar { arg_idx: u16 },
    // Returns prior value of variable
    AssignRetArgStr { arg_idx: u16 },
    // Returns prior value of variable
    AssignRetArgNum { arg_idx: u16 }, // Returns prior value of variable

    ArgVar { arg_idx: u16 },
    ArgNum { arg_idx: u16 },
    ArgStr { arg_idx: u16 },
    ArgArray { arg_idx: u16 },

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

    ArrayMember { indices: u16 },

    AssignArray { indices: u16 },
    AssignArrayNum { indices: u16 },
    AssignArrayStr { indices: u16 },

    AssignRetArray { indices: u16 },
    AssignRetArrayNum { indices: u16 },
    AssignRetArrayStr { indices: u16 }, // str stack

    ArrayIndex { indices: u16 },

    Call { target: u16 },

    Print,

    Printf { num_args: u16 }, // excluding fstring

    NoOp,

    Ret,

    // ConstI16(i16), // TODO: float which is exactly representable as an i16?
    // Index in constant table
    ConstLkpStr { idx: u16 },
    ConstLkpNum { idx: u16 },

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
    Sub3 { global: bool },

    // These will be transformed before reaching VM
    JumpIfFalseVarLbl(Label),
    JumpIfFalseNumLbl(Label),
    JumpIfFalseStrLbl(Label),
    JumpLbl(Label),
    JumpIfTrueVarLbl(Label),
    JumpIfTrueNumLbl(Label),
    JumpIfTrueStrLbl(Label),
    Label(Label), // n/a

    // Transformed into these
    RelJumpIfFalseNum { offset: i16 },
    RelJumpIfFalseVar { offset: i16 },
    RelJumpIfTrueNum { offset: i16 },
    RelJumpIfFalseStr { offset: i16 },
    RelJumpIfTrueStr { offset: i16 },
    RelJumpIfTrueVar { offset: i16 },

    RelJump { offset: i16 }, // n/a
}

impl Code {
    pub fn resolve_label_to_offset(&mut self, offset: i16) {
        let mut replacement_jump = match self {
            Code::JumpIfFalseStrLbl(_) => { Self::RelJumpIfFalseStr { offset } }
            Code::JumpIfFalseNumLbl(_) => { Self::RelJumpIfFalseNum { offset } }
            Code::JumpIfFalseVarLbl(_) => { Self::RelJumpIfFalseVar { offset } }
            Code::JumpLbl(_) => { Self::RelJump { offset } }
            Code::JumpIfTrueStrLbl(_) => { Self::RelJumpIfTrueStr { offset } }
            Code::JumpIfTrueNumLbl(_) => { Self::RelJumpIfTrueNum { offset } }
            Code::JumpIfTrueVarLbl(_) => { Self::RelJumpIfTrueVar { offset } }
            _ => return,
        };
        // Replace a jump to a label with a rel jump with an offset
        std::mem::swap(self, &mut replacement_jump);
    }

    pub fn move_stack_to_stack(src: ScalarType, dest: ScalarType) -> Self {
        match src {
            ScalarType::Str => {
                match dest {
                    ScalarType::Str => Code::NoOp,
                    ScalarType::Num => Code::StrToNum,
                    ScalarType::Var => Code::StrToVar,
                }
            }
            ScalarType::Num => {
                match dest {
                    ScalarType::Str => Code::NumToStr,
                    ScalarType::Num => Code::NoOp,
                    ScalarType::Var => Code::NumToVar,
                }
            }
            ScalarType::Var => {
                match dest {
                    ScalarType::Str => Code::VarToStr,
                    ScalarType::Num => Code::VarToNum,
                    ScalarType::Var => Code::NoOp,
                }
            }
        }
    }

    pub fn arg_scl(typ: ScalarType, arg_idx: u16) -> Self {
        return Code::ArgVar { arg_idx };
        // match typ {
        //     ScalarType::Variable => Code::ArgScl { arg_idx },
        //     ScalarType::String => Code::ArgStrScl { arg_idx },
        //     ScalarType::Float => Code::ArgNumScl { arg_idx },
        // }
    }

    pub fn jump_if_false(typ: ScalarType, label: &Label) -> Code {
        match typ {
            ScalarType::Str => Code::JumpIfFalseStrLbl(*label),
            ScalarType::Num => Code::JumpIfFalseNumLbl(*label),
            ScalarType::Var => Code::JumpIfFalseVarLbl(*label),
        }
    }
    pub fn jump_if_true(typ: ScalarType, label: &Label) -> Code {
        match typ {
            ScalarType::Str => Code::JumpIfTrueStrLbl(*label),
            ScalarType::Num => Code::JumpIfTrueNumLbl(*label),
            ScalarType::Var => Code::JumpIfTrueVarLbl(*label),
        }
    }
    pub fn pop(typ: ScalarType) -> Code {
        match typ {
            ScalarType::Num => Code::PopNum,
            ScalarType::Str => Code::PopStr,
            ScalarType::Var => Code::Pop,
        }
    }

    pub fn gscl(id: GlobalScalarId, typ: ScalarType) -> Self {
        match typ {
            ScalarType::Var => Code::GsclVar(id),
            ScalarType::Str => Code::GsclStr(id),
            ScalarType::Num => Code::GsclNum(id),
        }
    }
    pub fn arg_scl_assign(side_effect_only: bool, typ: ScalarType, arg_idx: u16) -> Self {
        if !side_effect_only {
            match typ {
                ScalarType::Str => Code::AssignRetArgStr { arg_idx },
                ScalarType::Num => Code::AssignRetArgNum { arg_idx },
                ScalarType::Var => Code::AssignRetArgVar { arg_idx },
            }
        } else {
            match typ {
                ScalarType::Str => Code::AssignArgStr { arg_idx },
                ScalarType::Num => Code::AssignArgNum { arg_idx },
                ScalarType::Var => Code::AssignArgVar { arg_idx },
            }
        }
    }
    pub fn gscl_assign(side_effect_only: bool, typ: ScalarType, idx: GlobalScalarId) -> Self {
        if !side_effect_only {
            match typ {
                ScalarType::Str => Code::AssignRetGsclVar(idx),
                ScalarType::Num => Code::AssignRetGsclNum(idx),
                ScalarType::Var => Code::AssignRetGsclStr(idx),
            }
        } else {
            match typ {
                ScalarType::Str => Code::AssignGsclStr(idx),
                ScalarType::Num => Code::AssignGsclNum(idx),
                ScalarType::Var => Code::AssignGsclVar(idx),
            }
        }
    }
    pub fn array_assign(indices: u16, typ: ScalarType, side_effect_only: bool) -> Self {
        if side_effect_only {
            match typ {
                ScalarType::Str => Code::AssignArrayStr { indices },
                ScalarType::Num => Code::AssignArrayNum { indices },
                ScalarType::Var => Code::AssignArray { indices },
            }
        } else {
            match typ {
                ScalarType::Str => Code::AssignRetArrayStr { indices },
                ScalarType::Num => Code::AssignRetArrayNum { indices },
                ScalarType::Var => Code::AssignRetArray { indices },
            }
        }
    }
}

use std::fmt::{Debug, Formatter};
use crate::parser::{ArgT, ScalarType};
use crate::stack_counter::{StackCounter as SC};
use crate::stackt::StackT;
use crate::util::pad;
use crate::vm::{VmFunc};
use crate::vm::VmProgram;


impl Debug for Meta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let args = pad(format!("[{:?}]", self.args), 20);
        let rets = self.returns.make_array();
        let ret = pad(format!("[{:?}]", rets), 40);
        write!(f, "args: {} push: {}", args, ret)
    }
}

pub struct Meta {
    // Stacks that arguments come from
    args: Vec<StackT>,
    // Stacks that are pushed to after the instruction
    returns: SC,
    is_ret: bool,
    descendant_offsets: Vec<isize>,
}

impl Meta {
    pub fn args(&self) -> &[StackT] {
        &self.args
    }
    pub fn new(args: Vec<StackT>, returns: SC) -> Self {
        Self { args, returns: returns, is_ret: false, descendant_offsets: vec![1] }
    }
    pub fn set_is_ret(mut self) -> Self {
        self.is_ret = true;
        self
    }
    pub fn jump(mut self, offsets: Vec<isize>) -> Self {
        self.descendant_offsets = offsets;
        self
    }
    pub fn returns(&self) -> &SC {
        &self.returns
    }
    pub fn is_ret(&self) -> bool {
        self.is_ret
    }
    pub fn descendants(&self) -> &[isize] {
        &self.descendant_offsets
    }
}

impl Code {
    pub fn pretty_print(&self, output: &mut String) {
        let mut byte_padded = pad(format!("{:?}", self), 40);
        output.push_str(&byte_padded);
    }
    pub fn pretty_print_owned(&self) -> String {
        let mut s = String::new();
        self.pretty_print(&mut s);
        s
    }

    pub fn meta(&self, functions: &FunctionMap) -> Meta {
        use StackT::{Num, Str, Var, Array};
        match self {
            Code::BuiltinAtan2 => Meta::new(vec![Num, Num], SC::num(1)),
            Code::BuiltinCos => Meta::new(vec![Num], SC::num(1)),
            Code::BuiltinExp => Meta::new(vec![Num], SC::num(1)),
            Code::BuiltinSubstr2 => Meta::new(vec![Str, Num], SC::str(1)),
            Code::BuiltinSubstr3 => Meta::new(vec![Str, Num, Num], SC::str(1)),
            Code::BuiltinIndex => Meta::new(vec![Str, Str], SC::num(1)),
            Code::BuiltinInt => Meta::new(vec![Num], SC::num(1)),
            Code::BuiltinLength0 => Meta::new(vec![], SC::num(1)),
            Code::BuiltinLength1 => Meta::new(vec![Str], SC::num(1)),
            Code::BuiltinLog => Meta::new(vec![Num], SC::num(1)),
            Code::BuiltinRand => Meta::new(vec![], SC::num(1)),
            Code::BuiltinSin => Meta::new(vec![Num], SC::num(1)),
            Code::BuiltinSplit2 => Meta::new(vec![Str, Array], SC::num(1)),
            Code::BuiltinSplit3 => Meta::new(vec![Str, Array, Str], SC::num(1)),
            Code::BuiltinSqrt => Meta::new(vec![Num], SC::num(1)),
            Code::BuiltinSrand0 => Meta::new(vec![], SC::num(1)),
            Code::BuiltinSrand1 => Meta::new(vec![Num], SC::num(1)),
            Code::BuiltinTolower => Meta::new(vec![Str], SC::str(1)),
            Code::BuiltinToupper => Meta::new(vec![Str], SC::str(1)),
            Code::FloatZero => Meta::new(vec![], SC::num(1)),
            Code::FloatOne => Meta::new(vec![], SC::num(1)),
            Code::Pop => Meta::new(vec![Var], SC::new()),
            Code::PopStr => Meta::new(vec![Str], SC::new()),
            Code::PopNum => Meta::new(vec![Num], SC::new()),
            Code::Column => Meta::new(vec![Num], SC::str(1)),
            Code::NextLine => Meta::new(vec![], SC::num(1)),

            // Global assignments
            Code::AssignGsclVar(_) => Meta::new(vec![Var], SC::new()),
            Code::AssignRetGsclVar(_) => Meta::new(vec![Var], SC::var(1)),
            Code::AssignGsclNum(_) => Meta::new(vec![Num], SC::new()),
            Code::AssignRetGsclNum(_) => Meta::new(vec![Num], SC::num(1)),
            Code::AssignGsclStr(_) => Meta::new(vec![Str], SC::new()),
            Code::AssignRetGsclStr(_) => Meta::new(vec![Str], SC::str(1)),

            // Load globals scalars
            Code::GsclVar(_) => Meta::new(vec![], SC::var(1)),
            Code::GsclNum(_) => Meta::new(vec![], SC::num(1)),
            Code::GsclStr(_) => Meta::new(vec![], SC::str(1)),

            // Arg assignments
            Code::AssignArgVar { .. } => Meta::new(vec![Var], SC::new()),
            Code::AssignRetArgVar { .. } => Meta::new(vec![Var], SC::var(1)),
            Code::AssignArgNum { .. } => Meta::new(vec![Num], SC::new()),
            Code::AssignRetArgNum { .. } => Meta::new(vec![Num], SC::num(1)),
            Code::AssignArgStr { .. } => Meta::new(vec![Str], SC::new()),
            Code::AssignRetArgStr { .. } => Meta::new(vec![Str], SC::str(1)),

            Code::ArgVar { .. } => Meta::new(vec![], SC::var(1)),
            Code::ArgNum { .. } => Meta::new(vec![], SC::num(1)),
            Code::ArgStr { .. } => Meta::new(vec![], SC::str(1)),
            Code::Exp => Meta::new(vec![Num, Num], SC::num(1)),
            Code::Mult => Meta::new(vec![Num, Num], SC::num(1)),
            Code::Div => Meta::new(vec![Num, Num], SC::num(1)),
            Code::Mod => Meta::new(vec![Num, Num], SC::num(1)),
            Code::Add => Meta::new(vec![Num, Num], SC::num(1)),
            Code::Minus => Meta::new(vec![Num, Num], SC::num(1)),

            Code::Lt => Meta::new(vec![Var, Var], SC::num(1)),
            Code::Gt => Meta::new(vec![Var, Var], SC::num(1)),
            Code::LtEq => Meta::new(vec![Var, Var], SC::num(1)),
            Code::GtEq => Meta::new(vec![Var, Var], SC::num(1)),
            Code::EqEq => Meta::new(vec![Var, Var], SC::num(1)),
            Code::Neq => Meta::new(vec![Var, Var], SC::num(1)),
            Code::Matches => Meta::new(vec![Str, Str], SC::num(1)),
            Code::NMatches => Meta::new(vec![Str, Str], SC::num(1)),

            Code::Concat { count } => {
                let mut args: Vec<StackT> = (0..*count).map(|_| Str).collect();
                Meta::new(args, SC::str(1))
            }
            Code::GlobalArr(_) => Meta::new(vec![], SC::arr(1)),
            Code::ArgArray { .. } => Meta::new(vec![], SC::arr(1)),

            Code::ArrayMember { indices } => Meta::new(add_indices(vec![Array], indices), SC::num(1)),
            Code::AssignArray { indices } => Meta::new(add_indices(vec![Var, Array], indices), SC::new()),
            Code::AssignArrayNum { indices } => Meta::new(add_indices(vec![Num, Array], indices), SC::new()),
            Code::AssignArrayStr { indices } => Meta::new(add_indices(vec![Str, Array], indices), SC::new()),
            Code::AssignRetArray { indices } => Meta::new(add_indices(vec![Var, Array], indices), SC::var(1)),
            Code::AssignRetArrayNum { indices } => Meta::new(add_indices(vec![Num, Array], indices), SC::num(1)),
            Code::AssignRetArrayStr { indices } => Meta::new(add_indices(vec![Str, Array], indices), SC::str(1)),
            Code::ArrayIndex { indices } => Meta::new(add_indices(vec![StackT::Array], indices), SC::var(1)),

            Code::Call { target } => {
                let func = functions.get_by_id(*target as usize).unwrap();
                let args = func.args();
                let mut arg_stacks: Vec<StackT> = args.iter().map(|a| match a.typ {
                    ArgT::Array => Some(Array),
                    ArgT::Scalar => Some(Var),
                    ArgT::Unknown => None,
                }).flatten().collect();
                Meta::new(arg_stacks, SC::var(1))
            }
            Code::Print => Meta::new(vec![Str], SC::new()),
            Code::Printf { num_args } => Meta::new((0..*num_args).map(|_| Str).collect(), SC::new()),
            Code::NoOp => Meta::new(vec![], SC::new()),
            Code::Ret => Meta::new(vec![Var], SC::var(1)).set_is_ret(),
            Code::ConstLkpStr { .. } => Meta::new(vec![], SC::str(1)),
            Code::ConstLkpNum { .. } => Meta::new(vec![], SC::num(1)),
            // Sub op doesn't do the assignment that'd be too complex
            Code::Sub3 { global } => Meta::new(vec![Str, Str, Str], SC::str(1).set(Num, 1)),
            Code::RelJumpIfFalseNum { offset } => Meta::new(vec![Num], SC::new()).jump(vec![*offset as isize, 1]),
            Code::RelJumpIfTrueNum { offset } => Meta::new(vec![Num], SC::new()).jump(vec![*offset as isize, 1]),
            Code::RelJumpIfTrueStr { offset } => Meta::new(vec![Str], SC::new()).jump(vec![*offset as isize, 1]),
            Code::RelJumpIfFalseStr { offset } => Meta::new(vec![Str], SC::new()).jump(vec![*offset as isize, 1]),
            Code::RelJumpIfTrueVar { offset } => Meta::new(vec![Var], SC::new()).jump(vec![*offset as isize, 1]),
            Code::RelJumpIfFalseVar { offset } => Meta::new(vec![Var], SC::new()).jump(vec![*offset as isize, 1]),
            Code::RelJump { offset } => Meta::new(vec![], SC::new()).jump(vec![*offset as isize]),
            Code::JumpIfTrueVarLbl(_) | Code::JumpIfFalseVarLbl(_)  | Code::JumpIfFalseNumLbl(_) | Code::JumpIfFalseStrLbl(_) | Code::JumpLbl(_) | Code::JumpIfTrueNumLbl(_) | Code::JumpIfTrueStrLbl(_) | Code::Label(_) => panic!("labels should be removed before bytecode analysis"),
            Code::NumToVar => Meta::new(vec![Num], SC::var(1)),
            Code::NumToStr => Meta::new(vec![Num], SC::str(1)),
            Code::StrToVar => Meta::new(vec![Str], SC::var(1)),
            Code::StrToNum => Meta::new(vec![Str], SC::num(1)),
            Code::VarToNum => Meta::new(vec![Var], SC::num(1)),
            Code::VarToStr => Meta::new(vec![Var], SC::str(1)),
        }
    }
}


fn add_indices(mut v: Vec<StackT>, num_arr_indices: &u16) -> Vec<StackT> {
    for _ in 0..*num_arr_indices {
        v.push(StackT::Str)
    }
    v
}

#[cfg(test)]
mod tests {
    use crate::vm::Code;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<Code>(), 4);
    }
}