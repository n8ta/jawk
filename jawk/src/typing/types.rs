use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::rc::Rc;
use immutable_chunkmap::map::Map;
use crate::parser::{ArgT, Function, Program, ScalarType};
use crate::symbolizer::Symbol;

#[derive(Clone, Debug)]
enum VarType {
    Float,
    String,
    Array,
    Variable,
}

impl Into<VarType> for ScalarType {
    fn into(self) -> VarType {
        match self {
            ScalarType::String => VarType::String,
            ScalarType::Float => VarType::Float,
            ScalarType::Variable => VarType::Variable,
        }
    }
}

pub type MapT = Map<Symbol, ScalarType, 1000>;

#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisResults {
    pub global_scalars: HashSet<Symbol>,
    pub global_arrays: HashMap<Symbol, i32>,
    pub str_consts: HashSet<String>,
}

impl AnalysisResults {
    pub fn new() -> Self {
        Self {
            global_scalars: Default::default(),
            global_arrays: Default::default(),
            str_consts: Default::default(),
        }
    }
}

pub struct Call {
    target: Symbol,
    args: Vec<CallArg>,
}

pub struct CallArg {
    typ: Option<ArgT>,
    is_arg: Option<Symbol>,
}

impl CallArg {
    pub fn new(typ: Option<ArgT>, arg: Symbol) -> Self {
        CallArg { typ, is_arg: Some(arg) }
    }
    pub fn new_expr(typ: Option<ArgT>) -> Self {
        CallArg { typ, is_arg: None }
    }
}

impl Call {
    pub fn new(target: Symbol, args: Vec<CallArg>) -> Self {
        Self { target, args }
    }
}

pub struct TypedFunc {
    pub func: Function,
    pub callers: HashSet<TypedFunc>,
    pub calls: Vec<Call>,
}

impl TypedFunc {
    pub fn new(func: Function, calls: Vec<Call>) -> Self {
        let len = func.args.len();
        Self {
            func,
            callers: HashSet::new(),
            calls,
        }
    }
    pub fn done(self) -> Function {
        self.func
    }
}

pub struct TypedProgram {
    pub functions: HashMap<Symbol, TypedFunc>,
    pub global_analysis: AnalysisResults,
}

impl TypedProgram {
    pub fn new(functions: HashMap<Symbol, TypedFunc>, results: AnalysisResults) -> Self {
        Self { functions, global_analysis: results }
    }
    pub fn done(self) -> Program {
        Program {
            global_analysis: self.global_analysis,
            functions: self.functions.into_iter()
                .map(|(name, func)| (name, func.func))
                .collect(),
        }
    }
}