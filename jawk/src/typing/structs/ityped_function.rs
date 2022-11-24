use std::cell::Ref;
use std::fmt::{Debug, Display};
use std::rc::Rc;
use hashbrown::HashSet;
use crate::{AnalysisResults, PrintableError};
use crate::parser::{Arg, ArgT};
use crate::symbolizer::Symbol;
use crate::typing::CallLink;
use crate::typing::TypedUserFunction;
use crate::typing::structs::{Call};

pub trait ITypedFunction: Debug + Display  {
    fn args(&self) -> Ref<'_, Vec<Arg>>;
    fn arity(&self) -> usize;
        fn add_caller(&self, caller: Rc<TypedUserFunction>);
    fn calls(&self) -> Ref<'_, Vec<Call>>;
    fn callers(&self) -> Ref<'_, HashSet<Rc<TypedUserFunction>>>;
    fn name(&self) -> Symbol;
    fn get_arg_idx_and_type(&self, name: &Symbol) -> Option<(usize, ArgT)>;

    fn reverse_call(&self, link: &CallLink, args: &[Arg], analysis: &mut AnalysisResults) -> Result<HashSet<Symbol>, PrintableError>;
    fn receive_call(&self, call: &Vec<ArgT>) -> Result<HashSet<Symbol>, PrintableError>;
}
impl PartialEq for dyn ITypedFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
    }
}