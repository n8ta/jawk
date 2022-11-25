use std::cell::Ref;
use std::fmt::{Display, Formatter, write};
use std::process::Termination;
use std::rc::Rc;
use hashbrown::HashSet;
use crate::parser::{Arg, ArgT};
use crate::PrintableError;
use crate::symbolizer::Symbol;
use crate::typing::{AnalysisResults, CallLink, ITypedFunction, TypedProgram, TypedUserFunction};
use crate::typing::structs::Call;

#[derive(Debug)]
pub enum BuiltinFunc {
    Atan2,
    Close,
    Cos,
    Exp,
    Gsub,
    Index,
    Int,
    Length,
    Log,
    Matches,
    Rand,
    Sin,
    Split,
    Sprintf,
    Sqrt,
    Srand,
    Sub,
    Substr,
    System,
    Tolower,
    Toupper,
}

impl BuiltinFunc {
    pub fn get(value: &str) -> Option<BuiltinFunc> {
        let res = match value {
            "atan2" => BuiltinFunc::Atan2,
            "close" => BuiltinFunc::Close,
            "cos" => BuiltinFunc::Cos,
            "exp" => BuiltinFunc::Exp,
            "gsub" => BuiltinFunc::Gsub,
            "index" => BuiltinFunc::Index,
            "int" => BuiltinFunc::Int,
            "length" => BuiltinFunc::Length,
            "log" => BuiltinFunc::Log,
            "match" => BuiltinFunc::Matches,
            "rand" => BuiltinFunc::Rand,
            "sin" => BuiltinFunc::Sin,
            "split" => BuiltinFunc::Split,
            "sprintf" => BuiltinFunc::Sprintf,
            "sqrt" => BuiltinFunc::Sqrt,
            "srand" => BuiltinFunc::Srand,
            "sub" => BuiltinFunc::Sub,
            "substr" => BuiltinFunc::Substr,
            "system" => BuiltinFunc::System,
            "tolower" => BuiltinFunc::Tolower,
            "toupper" => BuiltinFunc::Toupper,
            _ => return None,
        };
        Some(res)
    }
    pub fn is_builtin(str: &str) -> bool {
        BuiltinFunc::get(str).is_some()
    }
}

impl Display for BuiltinFunc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&str> for BuiltinFunc {
    type Error = PrintableError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match BuiltinFunc::get(value) {
            Some(r) => Ok(r),
            _ => Err(PrintableError::new(format!("{} is not a builtin function", value))),
        }
    }
}

impl ITypedFunction for BuiltinFunc {
    fn args(&self) -> Ref<'_, Vec<Arg>> {
        todo!()
    }

    fn arity(&self) -> usize {
        todo!()
    }

    fn add_caller(&self, caller: Rc<TypedUserFunction>) {
        todo!()
    }

    fn calls(&self) -> Ref<'_, Vec<Call>> {
        todo!()
    }

    fn callers(&self) -> Ref<'_, HashSet<Rc<TypedUserFunction>>> {
        todo!()
    }

    fn name(&self) -> Symbol {
        todo!()
    }

    fn get_call_types(&self, global_analysis: &AnalysisResults, link: &CallLink) -> Vec<ArgT> {
        todo!()
    }

    fn reverse_call(&self, link: &CallLink, args: &[Arg], analysis: &mut AnalysisResults) -> Result<HashSet<Symbol>, PrintableError> {
        todo!()
    }

    fn receive_call(&self, call: &Vec<ArgT>) -> Result<HashSet<Symbol>, PrintableError> {
        todo!()
    }
}