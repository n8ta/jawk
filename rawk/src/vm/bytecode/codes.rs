use crate::typing::{FunctionMap, GlobalArrayId, GlobalScalarId, ITypedFunction};
use std::fmt::{Debug, Write};
use std::rc::Rc;
use crate::awk_str::RcAwkStr;
use crate::parser::{ArgT, ScalarType, SclSpecial};
use crate::stack_counter::{StackCounter as SC};
use crate::stackt::StackT;
use crate::util::pad;
use crate::vm::bytecode::code_and_immed::{CodeAndImmed as CI};
use crate::vm::bytecode::{Immed, Meta};
use crate::vm::{VmProgram, StringScalar};
use crate::vm::bytecode::subroutines::{num_to_var, builtin_atan2, builtin_cos, builtin_exp, builtin_substr2, builtin_substr3, builtin_index, builtin_int, builtin_length0, builtin_length1, builtin_log, builtin_rand, builtin_sin, builtin_split2, builtin_split3, builtin_sqrt, builtin_srand0, builtin_srand1, builtin_tolower, builtin_toupper, num_to_str, str_to_var, str_to_num, var_to_num, var_to_str, pop, pop_str, pop_num, column, assign_gscl_var, assign_gscl_num, assign_gscl_str, assign_gscl_ret_str, assign_gscl_ret_var, assign_gscl_ret_num, global_arr, gscl_var, gscl_num, gscl_str, assign_arg_var, assign_arg_str, assign_arg_num, assign_arg_ret_var, assign_arg_ret_str, assign_arg_ret_num, arg_var, arg_str, arg_num, arg_arr, exp, mult, div, modulo, add, minus, lt, gt, lteq, gteq, eqeq, neq, matches, nmatches, assign_array_var, assign_array_str, assign_array_num, assign_array_ret_var, assign_array_ret_str, assign_array_ret_num, array_index, array_member, concat, gsub3, sub3, rel_jump_if_false_var, rel_jump_if_false_str, rel_jump_if_false_num, rel_jump_if_true_var, rel_jump_if_true_str, rel_jump_if_true_num, rel_jump, print, printf, noop, ret, const_num, const_str, const_str_num, call, neq_num, gteq_num, eqeq_num, lteq_num, lt_num, gt_num, clear_gscl, clear_argscl, rel_jump_if_true_next_line, rel_jump_if_false_next_line, scl_special, assign_scl_special, assign_ret_scl_special};

pub type LabelId = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Label {
    id: LabelId,
}

impl Label {
    pub fn new(id: LabelId) -> Self {
        Self { id }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Code {
    NumToVar,
    NumToStr,
    StrToVar,
    StrToNum,
    VarToNum,
    VarToStr,

    Pop,
    PopStr,
    PopNum,

    Column,

    ClearGscl(GlobalScalarId),
    ClearArgScl(usize),

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

    AssignArgVar { arg_idx: usize },
    AssignArgStr { arg_idx: usize },
    AssignArgNum { arg_idx: usize },

    AssignRetArgVar { arg_idx: usize },
    AssignRetArgStr { arg_idx: usize },
    AssignRetArgNum { arg_idx: usize },

    ArgVar { arg_idx: usize },
    ArgNum { arg_idx: usize },
    ArgStr { arg_idx: usize },
    ArgArray { arg_idx: usize },

    AssignSclSpecialVar(SclSpecial),
    AssignRetSclSpecialVar(SclSpecial),
    SclSpecialVar(SclSpecial),

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

    LtNum,
    GtNum,
    LtEqNum,
    GtEqNum,
    EqEqNum,
    NeqNum,

    Matches,
    NMatches,

    Concat { count: usize },

    ArrayMember { indices: usize },

    AssignArray { indices: usize },
    AssignArrayNum { indices: usize },
    AssignArrayStr { indices: usize },

    AssignRetArray { indices: usize },
    AssignRetArrayNum { indices: usize },
    AssignRetArrayStr { indices: usize }, // str stack

    ArrayIndex { indices: usize },

    Call { target: usize },

    Print,

    Printf { num_args: usize }, // excluding fstring

    NoOp,

    Ret,

    // Index in constant table
    ConstStr { str: RcAwkStr },
    ConstStrNum { strnum: RcAwkStr },
    ConstNum { num: f64 },

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
    JumpIfTrueNextLineLbl(Label),
    JumpIfFalseNextLineLbl(Label),
    JumpIfFalseVarLbl(Label),
    JumpIfFalseNumLbl(Label),
    JumpIfFalseStrLbl(Label),
    JumpLbl(Label),
    JumpIfTrueVarLbl(Label),
    JumpIfTrueNumLbl(Label),
    JumpIfTrueStrLbl(Label),
    Label(Label), // n/a

    // Transformed into these
    RelJumpIfFalseNum { offset: isize },
    RelJumpIfFalseVar { offset: isize },
    RelJumpIfTrueNum { offset: isize },
    RelJumpIfFalseStr { offset: isize },
    RelJumpIfTrueStr { offset: isize },
    RelJumpIfTrueVar { offset: isize },
    RelJumpIfTrueNextLine { offset: isize },
    RelJumpIfFalseNextLine { offset: isize },

    RelJump { offset: isize },
}

impl Code {
    pub fn resolve_label_to_offset(&mut self, offset: isize) {
        let mut replacement_jump = match self {
            Code::JumpIfFalseStrLbl(_) => { Self::RelJumpIfFalseStr { offset } }
            Code::JumpIfFalseNumLbl(_) => { Self::RelJumpIfFalseNum { offset } }
            Code::JumpIfFalseVarLbl(_) => { Self::RelJumpIfFalseVar { offset } }
            Code::JumpLbl(_) => { Self::RelJump { offset } }
            Code::JumpIfTrueStrLbl(_) => { Self::RelJumpIfTrueStr { offset } }
            Code::JumpIfTrueNumLbl(_) => { Self::RelJumpIfTrueNum { offset } }
            Code::JumpIfTrueVarLbl(_) => { Self::RelJumpIfTrueVar { offset } }
            Code::JumpIfFalseNextLineLbl(_) => { Self::RelJumpIfFalseNextLine { offset } }
            Code::JumpIfTrueNextLineLbl(_) => { Self::RelJumpIfTrueNextLine { offset } }
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

    pub fn arg_scl(_typ: ScalarType, arg_idx: usize) -> Self {
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
            ScalarType::Str => Code::GsclStr(id),
            ScalarType::Num => Code::GsclNum(id),
            ScalarType::Var => Code::GsclVar(id),
        }
    }
    pub fn arg_scl_assign(side_effect_only: bool, typ: ScalarType, arg_idx: usize) -> (Self, Option<StackT>) {
        if !side_effect_only {
            let stack = Some(typ.into());
            (match typ {
                ScalarType::Str => Code::AssignRetArgStr { arg_idx },
                ScalarType::Num => Code::AssignRetArgNum { arg_idx },
                ScalarType::Var => Code::AssignRetArgVar { arg_idx },
            }, stack)
        } else {
            (match typ {
                ScalarType::Str => Code::AssignArgStr { arg_idx },
                ScalarType::Num => Code::AssignArgNum { arg_idx },
                ScalarType::Var => Code::AssignArgVar { arg_idx },
            }, None)
        }
    }
    pub fn gscl_assign(side_effect_only: bool, typ: ScalarType, idx: GlobalScalarId) -> (Self, Option<StackT>) {
        if !side_effect_only {
            let stack = Some(typ.into());
            (match typ {
                ScalarType::Str => Code::AssignRetGsclStr(idx),
                ScalarType::Num => Code::AssignRetGsclNum(idx),
                ScalarType::Var => Code::AssignRetGsclVar(idx),
            }, stack)
        } else {
            (match typ {
                ScalarType::Str => Code::AssignGsclStr(idx),
                ScalarType::Num => Code::AssignGsclNum(idx),
                ScalarType::Var => Code::AssignGsclVar(idx),
            }, None)
        }
    }

    pub fn special_assign(side_effect_only: bool, special: SclSpecial) -> (Self, Option<StackT>) {
        if !side_effect_only {
            (Code::AssignRetSclSpecialVar(special), Some(StackT::Var))
        } else {
            (Code::AssignSclSpecialVar(special), None)
        }
    }

    pub fn array_assign(indices: usize, typ: ScalarType, side_effect_only: bool) -> Self {
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

    #[cfg(test)]
    pub fn pretty_print(&self, output: &mut Vec<u8>) {
        let byte_padded = pad(format!("{:?}", self), 40);
        output.extend_from_slice(&byte_padded.as_bytes());
    }

    #[cfg(test)]
    pub fn pretty_print_owned(&self) -> String {
        let mut s = vec![];
        self.pretty_print(&mut s);
        unsafe { String::from_utf8_unchecked(s) }
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
            Code::Pop => Meta::new(vec![Var], SC::new()),
            Code::PopStr => Meta::new(vec![Str], SC::new()),
            Code::PopNum => Meta::new(vec![Num], SC::new()),
            Code::Column => Meta::new(vec![Num], SC::str(1)),

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

            Code::LtNum => Meta::new(vec![Num, Num], SC::num(1)),
            Code::GtNum => Meta::new(vec![Num, Num], SC::num(1)),
            Code::LtEqNum => Meta::new(vec![Num, Num], SC::num(1)),
            Code::GtEqNum => Meta::new(vec![Num, Num], SC::num(1)),
            Code::EqEqNum => Meta::new(vec![Num, Num], SC::num(1)),
            Code::NeqNum => Meta::new(vec![Num, Num], SC::num(1)),

            Code::Matches => Meta::new(vec![Str, Str], SC::num(1)),
            Code::NMatches => Meta::new(vec![Str, Str], SC::num(1)),

            Code::Concat { count } => {
                let args: Vec<StackT> = (0..*count).map(|_| Str).collect();
                Meta::new(args, SC::str(1))
            }
            Code::ClearGscl { .. } => Meta::new(vec![], SC::new()),
            Code::ClearArgScl { .. } => Meta::new(vec![], SC::new()),

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
                let arg_stacks: Vec<StackT> = args.iter().map(|a| match a.typ {
                    ArgT::Array => Some(Array),
                    ArgT::Scalar => Some(Var),
                    ArgT::Unknown => None,
                }).flatten().collect();
                Meta::new(arg_stacks, SC::var(1))
            }
            Code::Print => Meta::new(vec![Str], SC::new()),
            Code::Printf { num_args } => Meta::new((0..*num_args + 1).map(|_| Str).collect(), SC::new()),
            Code::NoOp => Meta::new(vec![], SC::new()),
            Code::Ret => Meta::new(vec![Var], SC::var(1)).set_is_ret(),
            Code::ConstStr { .. } => Meta::new(vec![], SC::str(1)),
            Code::ConstStrNum { .. } => Meta::new(vec![], SC::str(1)),
            Code::ConstNum { .. } => Meta::new(vec![], SC::num(1)),
            // Sub op doesn't do the assignment that'd be too complex
            Code::Sub3 { .. } => Meta::new(vec![Str, Str, Str], SC::str(1).set(Num, 1)),
            Code::RelJumpIfFalseNum { offset } => Meta::new(vec![Num], SC::new()).jump(vec![*offset, 1]),
            Code::RelJumpIfTrueNum { offset } => Meta::new(vec![Num], SC::new()).jump(vec![*offset, 1]),
            Code::RelJumpIfTrueStr { offset } => Meta::new(vec![Str], SC::new()).jump(vec![*offset, 1]),
            Code::RelJumpIfFalseStr { offset } => Meta::new(vec![Str], SC::new()).jump(vec![*offset, 1]),
            Code::RelJumpIfTrueVar { offset } => Meta::new(vec![Var], SC::new()).jump(vec![*offset, 1]),
            Code::RelJumpIfFalseVar { offset } => Meta::new(vec![Var], SC::new()).jump(vec![*offset, 1]),
            Code::RelJump { offset } => Meta::new(vec![], SC::new()).jump(vec![*offset]),
            Code::RelJumpIfTrueNextLine { offset } => Meta::new(vec![], SC::new()).jump(vec![*offset as isize, 1]),
            Code::RelJumpIfFalseNextLine { offset } => Meta::new(vec![], SC::new()).jump(vec![*offset as isize, 1]),
            Code::JumpIfFalseNextLineLbl(_) | Code::JumpIfTrueNextLineLbl(_) | Code::JumpIfTrueVarLbl(_)
            | Code::JumpIfFalseVarLbl(_) | Code::JumpIfFalseNumLbl(_) | Code::JumpIfFalseStrLbl(_)
            | Code::JumpLbl(_) | Code::JumpIfTrueNumLbl(_) | Code::JumpIfTrueStrLbl(_)
            | Code::Label(_) =>
                panic!("labels should be removed before bytecode analysis"),
            Code::NumToVar => Meta::new(vec![Num], SC::var(1)),
            Code::NumToStr => Meta::new(vec![Num], SC::str(1)),
            Code::StrToVar => Meta::new(vec![Str], SC::var(1)),
            Code::StrToNum => Meta::new(vec![Str], SC::num(1)),
            Code::VarToNum => Meta::new(vec![Var], SC::num(1)),
            Code::VarToStr => Meta::new(vec![Var], SC::str(1)),

            Code::AssignSclSpecialVar(_) => Meta::new(vec![Var], SC::new()),
            Code::AssignRetSclSpecialVar(_) => Meta::new(vec![Var], SC::var(1)),
            Code::SclSpecialVar(_) => Meta::new(vec![], SC::var(1)),
        }
    }

    pub fn transform(&self) -> CI {
        match self {
            Code::NumToVar => CI::new(num_to_var),
            Code::NumToStr => CI::new(num_to_str),
            Code::StrToVar => CI::new(str_to_var),
            Code::StrToNum => CI::new(str_to_num),
            Code::VarToNum => CI::new(var_to_num),
            Code::VarToStr => CI::new(var_to_str),
            Code::Pop => CI::new(pop),
            Code::PopStr => CI::new(pop_str),
            Code::PopNum => CI::new(pop_num),
            Code::Column => CI::new(column),
            Code::AssignGsclVar(id) => CI::imm(assign_gscl_var, Immed { global_scl_id: *id }),
            Code::AssignGsclNum(id) => CI::imm(assign_gscl_num, Immed { global_scl_id: *id }),
            Code::AssignGsclStr(id) => CI::imm(assign_gscl_str, Immed { global_scl_id: *id }),
            Code::AssignRetGsclVar(id) => CI::imm(assign_gscl_ret_var, Immed { global_scl_id: *id }),
            Code::AssignRetGsclNum(id) => CI::imm(assign_gscl_ret_num, Immed { global_scl_id: *id }),
            Code::AssignRetGsclStr(id) => CI::imm(assign_gscl_ret_str, Immed { global_scl_id: *id }),

            Code::GlobalArr(id) => CI::imm(global_arr, Immed { global_arr_id: *id }),
            Code::GsclVar(id) => CI::imm(gscl_var, Immed { global_scl_id: *id }),
            Code::GsclNum(id) => CI::imm(gscl_num, Immed { global_scl_id: *id }),
            Code::GsclStr(id) => CI::imm(gscl_str, Immed { global_scl_id: *id }),

            Code::AssignArgVar { arg_idx } => CI::imm(assign_arg_var, Immed { arg_idx: *arg_idx }),
            Code::AssignArgStr { arg_idx } => CI::imm(assign_arg_str, Immed { arg_idx: *arg_idx }),
            Code::AssignArgNum { arg_idx } => CI::imm(assign_arg_num, Immed { arg_idx: *arg_idx }),
            Code::AssignRetArgVar { arg_idx } => CI::imm(assign_arg_ret_var, Immed { arg_idx: *arg_idx }),
            Code::AssignRetArgStr { arg_idx } => CI::imm(assign_arg_ret_str, Immed { arg_idx: *arg_idx }),
            Code::AssignRetArgNum { arg_idx } => CI::imm(assign_arg_ret_num, Immed { arg_idx: *arg_idx }),
            Code::ArgVar { arg_idx } => CI::imm(arg_var, Immed { arg_idx: *arg_idx }),
            Code::ArgNum { arg_idx } => CI::imm(arg_num, Immed { arg_idx: *arg_idx }),
            Code::ArgStr { arg_idx } => CI::imm(arg_str, Immed { arg_idx: *arg_idx }),
            Code::ArgArray { arg_idx } => CI::imm(arg_arr, Immed { arg_idx: *arg_idx }),

            Code::Exp => CI::new(exp),
            Code::Mult => CI::new(mult),
            Code::Div => CI::new(div),
            Code::Mod => CI::new(modulo),
            Code::Add => CI::new(add),
            Code::Minus => CI::new(minus),
            Code::Lt => CI::new(lt),
            Code::Gt => CI::new(gt),
            Code::LtEq => CI::new(lteq),
            Code::GtEq => CI::new(gteq),
            Code::EqEq => CI::new(eqeq),
            Code::Neq => CI::new(neq),
            Code::Matches => CI::new(matches),
            Code::NMatches => CI::new(nmatches),

            Code::Concat { count } => CI::imm(concat, Immed { concat_count: *count }),

            Code::ArrayMember { indices } => CI::imm(array_member, Immed { array_indices: *indices }),
            Code::ArrayIndex { indices } => CI::imm(array_index, Immed { array_indices: *indices }),

            Code::AssignArray { indices } => CI::imm(assign_array_var, Immed { array_indices: *indices }),
            Code::AssignArrayStr { indices } => CI::imm(assign_array_str, Immed { array_indices: *indices }),
            Code::AssignArrayNum { indices } => CI::imm(assign_array_num, Immed { array_indices: *indices }),
            Code::AssignRetArray { indices } => CI::imm(assign_array_ret_var, Immed { array_indices: *indices }),
            Code::AssignRetArrayStr { indices } => CI::imm(assign_array_ret_str, Immed { array_indices: *indices }),
            Code::AssignRetArrayNum { indices } => CI::imm(assign_array_ret_num, Immed { array_indices: *indices }),

            Code::BuiltinAtan2 => CI::new(builtin_atan2),
            Code::BuiltinCos => CI::new(builtin_cos),
            Code::BuiltinExp => CI::new(builtin_exp),
            Code::BuiltinSubstr2 => CI::new(builtin_substr2),
            Code::BuiltinSubstr3 => CI::new(builtin_substr3),
            Code::BuiltinIndex => CI::new(builtin_index),
            Code::BuiltinInt => CI::new(builtin_int),
            Code::BuiltinLength0 => CI::new(builtin_length0),
            Code::BuiltinLength1 => CI::new(builtin_length1),
            Code::BuiltinLog => CI::new(builtin_log),
            Code::BuiltinRand => CI::new(builtin_rand),
            Code::BuiltinSin => CI::new(builtin_sin),
            Code::BuiltinSplit2 => CI::new(builtin_split2),
            Code::BuiltinSplit3 => CI::new(builtin_split3),
            Code::BuiltinSqrt => CI::new(builtin_sqrt),
            Code::BuiltinSrand0 => CI::new(builtin_srand0),
            Code::BuiltinSrand1 => CI::new(builtin_srand1),
            Code::BuiltinTolower => CI::new(builtin_tolower),
            Code::BuiltinToupper => CI::new(builtin_toupper),
            Code::Sub3 { global } => CI::new(if *global { gsub3 } else { sub3 }),
            Code::Print => CI::new(print),
            Code::Printf { num_args } => CI::imm(printf, Immed { printf_args: *num_args }),
            Code::NoOp => CI::new(noop),
            Code::Ret => CI::new(ret),

            Code::Call { target } => CI::imm(call, Immed { call_target: *target }),

            Code::RelJumpIfFalseVar { offset } => CI::imm(rel_jump_if_false_var, Immed { offset: *offset }),
            Code::RelJumpIfFalseStr { offset } => CI::imm(rel_jump_if_false_str, Immed { offset: *offset }),
            Code::RelJumpIfFalseNum { offset } => CI::imm(rel_jump_if_false_num, Immed { offset: *offset }),

            Code::RelJumpIfTrueVar { offset } => CI::imm(rel_jump_if_true_var, Immed { offset: *offset }),
            Code::RelJumpIfTrueStr { offset } => CI::imm(rel_jump_if_true_str, Immed { offset: *offset }),
            Code::RelJumpIfTrueNum { offset } => CI::imm(rel_jump_if_true_num, Immed { offset: *offset }),
            Code::RelJumpIfTrueNextLine { offset } => CI::imm(rel_jump_if_true_next_line, Immed { offset: *offset }),
            Code::RelJumpIfFalseNextLine { offset } => CI::imm(rel_jump_if_false_next_line, Immed { offset: *offset }),

            Code::RelJump { offset } => CI::imm(rel_jump, Immed { offset: *offset }),

            Code::ConstStr { str: string } => CI::imm(const_str, Immed { string: unsafe { string.clone().into_raw() } }),
            Code::ConstStrNum { strnum: string } => CI::imm(const_str_num, Immed { string: unsafe { string.clone().into_raw() } }),
            Code::ConstNum { num } => CI::imm(const_num, Immed { num: *num }),

            Code::LtNum => CI::new(lt_num),
            Code::GtNum => CI::new(gt_num),
            Code::LtEqNum => CI::new(lteq_num),
            Code::GtEqNum => CI::new(gteq_num),
            Code::EqEqNum => CI::new(eqeq_num),
            Code::NeqNum => CI::new(neq_num),
            Code::ClearGscl(id) => CI::imm(clear_gscl, Immed { global_scl_id: *id }),
            Code::ClearArgScl(arg_idx) => CI::imm(clear_argscl, Immed { arg_idx: *arg_idx }),

            Code::AssignSclSpecialVar(special) => CI::imm(assign_scl_special, Immed { special: *special }),
            Code::AssignRetSclSpecialVar(special) => CI::imm(assign_ret_scl_special, Immed { special: *special }),
            Code::SclSpecialVar(special) => CI::imm(scl_special, Immed { special: *special }),

            Code::Label(_) | Code::JumpIfTrueNextLineLbl(_) | Code::JumpIfFalseNextLineLbl(_) | Code::JumpIfFalseVarLbl(_) | Code::JumpIfFalseNumLbl(_) | Code::JumpIfFalseStrLbl(_) | Code::JumpLbl(_) | Code::JumpIfTrueVarLbl(_) | Code::JumpIfTrueNumLbl(_) | Code::JumpIfTrueStrLbl(_) => {
                panic!("labels should be removed before direct threading {:?}", self);
            }
        }
    }
}


fn add_indices(mut v: Vec<StackT>, num_arr_indices: &usize) -> Vec<StackT> {
    for _ in 0..*num_arr_indices {
        v.push(StackT::Str)
    }
    v
}