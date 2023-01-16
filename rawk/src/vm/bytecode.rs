use crate::typing::{GlobalArrayId, GlobalScalarId};

pub type LabelId = u16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Label {
    id: LabelId,
}

impl Label {
    pub fn new(id: LabelId) -> Self {
        Self { id }
    }
}

pub enum Code {
    // Key: we use two stacks.

    // ss contains scalar values like Value::Float(1.1) and Value::Str(..)
    // ss: scalar stack

    // as: contains global array ids
    // as: array stack

    // +1ss
    FloatZero,
    FloatOne,

    // +1ss
    ScalarBarrier,
    // +1as
    ArrayBarrier,

    // -1ss
    Pop,

    // -1ss +1ss
    Column,

    // +1ss (is there a next line?)
    NextLine,

    // -1ss, +1ss
    GlobalScalarAssign(GlobalScalarId),

    // +1ss
    GlobalScalar(GlobalScalarId),

    // -1ss, +1ss
    ArgScalarAssign { arg_idx: u16 },

    // +1ss
    ArgScalar { arg_idx: u16 },

    // -2ss, +1ss
    Exp,

    // -1ss, +1ss
    UnaryPlus,

    // -1ss, +1ss
    UnaryMinus,

    // -2ss, +1ss
    Mult,
    Div,
    Mod,
    Add,
    Sub,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEq,
    Neq,
    Matches,
    NotMatches,

    // -count ss, +1ss
    Concat { count: u16 },

    // +1as pushes an array identifier onto the as
    GlobalArray(GlobalArrayId),
    ArgArray { arg_idx: u16 },

    // -1as
    // -1ss new value
    // -num_indices ss
    // +1ss
    ArrayMember { num_indices: u16 },
    ArrayAssign { num_indices: u16 },

    // -1as,
    // -num_indices ss,
    // +1ss
    ArrayIndex { num_indices: u16 },

    // -X as (remove until barrier)
    // -X ss (remove until barrier)
    // +1ss
    Call { target: u16 },

    // -1ss
    Print,

    // -Xss
    Printf { num_args: u16 }, // excluding fstring

    // nothing
    NoOp,

    // -1ss
    Ret,

    // +1ss
    // ConstI16(i16), // TODO: float which is exactly representable as an i16
    ConstantLookup { idx: u16 }, // Index in constant table

    // These will be transformed before reaching VM
    JumpIfFalseLbl(Label),
    // -1ss
    JumpLbl(Label),
    JumpIfTrueLbl(Label),
    // -1ss
    Label(Label), // n/a

    // Transformed into these
    RelJumpIfFalse { offset: i16 },
    //-1ss
    RelJumpIfTrue { offset: i16 },
    // -1ss
    RelJump { offset: i16 }, // n/a
}

#[cfg(test)]
mod tests {
    use crate::vm::Code;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<Code>(), 4);
    }
}