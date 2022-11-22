use crate::parser::ArgT;
use crate::typing::ITypedFunction;
use crate::typing::structs::Call;

pub struct CallLink {
    pub source: Box<dyn ITypedFunction>,
    pub call: Call,
}

pub type CallInfo = Vec<ArgT>;