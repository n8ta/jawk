use std::rc::Rc;
use crate::parser::ArgT;
use crate::typing::ITypedFunction;
use crate::typing::structs::Call;

pub struct CallLink {
    pub source: Rc<dyn ITypedFunction>,
    pub call: Call,
}