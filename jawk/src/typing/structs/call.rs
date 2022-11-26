use crate::symbolizer::Symbol;
use crate::typing::ITypedFunction;
use hashbrown::HashSet;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;

pub struct Call {
    pub target: Rc<dyn ITypedFunction>,
    pub args: Vec<CallArg>,
    pub src: Rc<dyn ITypedFunction>,
}
impl PartialEq for Call {
    fn eq(&self, other: &Self) -> bool {
        self.target.name() == other.target.name()
            && self.args == other.args
            && self.src.name() == other.src.name()
    }
}
impl Clone for Call {
    fn clone(&self) -> Self {
        Self {
            target: self.target.clone(),
            args: self.args.clone(),
            src: self.src.clone(),
        }
    }
}

impl Debug for Call {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "target: {:?}, args: {:?}", self.target.name(), self.args)
    }
}

impl Call {
    pub fn uses_any(&self, symbols: &HashSet<Symbol>) -> bool {
        for arg in self.args.iter() {
            match arg {
                CallArg::Variable(arg_name) => {
                    if symbols.contains(arg_name) {
                        return true;
                    }
                }
                CallArg::Scalar => {}
            }
        }
        false
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum CallArg {
    Variable(Symbol),
    Scalar,
}

impl CallArg {
    pub fn new(name: Symbol) -> Self {
        CallArg::Variable(name)
    }
    pub fn new_scalar() -> Self {
        CallArg::Scalar
    }
}

impl Call {
    pub fn new(
        src: Rc<dyn ITypedFunction>,
        target: Rc<dyn ITypedFunction>,
        args: Vec<CallArg>,
    ) -> Self {
        Self { src, target, args }
    }
}
