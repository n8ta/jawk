use std::cell::{Ref, RefMut};
use hashbrown::HashSet;
use crate::parser::{Arg, ArgT, Function};
use crate::{AnalysisResults, PrintableError};
use crate::symbolizer::Symbol;
use crate::typing::inference_pass::CallLink;
use crate::typing::TypedUserFunction;
use crate::typing::types::Call;

pub trait ITypedFunction {
    fn args(&self) -> Ref<'_, Vec<Arg>>;
    fn function(&self) -> RefMut<'_, Function>;
    fn add_call(&self, call: Call);
    fn add_caller(&self, caller: TypedUserFunction);
    fn calls(&self) -> Ref<'_, Vec<Call>>;
    fn callers(&self) -> Ref<'_, HashSet<TypedUserFunction>>;
    fn name(&self) -> Symbol;
    fn arity(&self) -> usize;
    fn get_arg_idx_and_type(&self, name: &Symbol) -> Option<(usize, ArgT)>;
    fn use_global(&self, var: &Symbol);
    fn set_arg_type(&self, var: &Symbol, typ: ArgT) -> Result<(), PrintableError>;
    fn reverse_call(&self, link: &CallLink, args: &[Arg], analysis: &mut AnalysisResults) -> Result<HashSet<Symbol>, PrintableError>;
    fn receive_call(&self, call: &Vec<ArgT>) -> Result<HashSet<Symbol>, PrintableError>;
}