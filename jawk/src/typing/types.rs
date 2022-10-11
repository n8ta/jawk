use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::rc::Rc;
use immutable_chunkmap::map::Map;
use libc::glob;
use crate::parser::{Arg, ArgT, Function, Program, ScalarType};
use crate::PrintableError;
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
    pub global_scalars: HashMap<Symbol, i32>,
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

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Call {
    pub target: Symbol,
    pub args: Vec<CallArg>,
}

impl Call {
    pub fn uses(&self, name: &Symbol) -> bool {
        for arg in self.args.iter() {
            match arg {
                CallArg::Variable(arg_name) => {
                    if arg_name == name {
                        return true;
                    }
                }
                CallArg::Scalar => {}
            }
        }
        false
    }
    pub fn uses_any(&self, symbols: &[Symbol]) -> bool {
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
    pub fn new(target: Symbol, args: Vec<CallArg>) -> Self {
        Self { target, args }
    }
}

pub struct TypedFunc {
    pub func: Function,
    pub callers: HashSet<Symbol>,
    pub calls: Vec<Call>,
}

impl TypedFunc {
    pub fn new(func: Function, calls: Vec<Call>) -> Self {
        Self {
            func,
            callers: HashSet::new(),
            calls,
        }
    }
    pub fn done(self) -> Function {
        self.func
    }
    pub fn get_arg(&mut self, name: &Symbol) -> Option<&mut Arg> {
        if let Some(arg) = self.func.args.iter_mut().find(|a| a.name == *name) {
            Some(arg)
        } else {
            None
        }
    }
    pub fn use_as_array(&mut self, var: &Symbol, global_analysis: &mut AnalysisResults) -> Result<Option<Symbol>, PrintableError> {
        if let Some(arg) = self.get_arg(var) {
            if let Some(arg_typ) = arg.typ {
                match arg_typ {
                    ArgT::Scalar => return Err(PrintableError::new(format!("fatal: attempt to use scalar `{}` in a array context", var))),
                    ArgT::Array => {}
                }
            } else {
                arg.typ = Some(ArgT::Array);
                return Ok(Some(arg.name.clone()));
            }
            return Ok(None);
        }

        if let Some(_type) = global_analysis.global_scalars.get(var) {
            return Err(PrintableError::new(format!("fatal: attempt to scalar `{}` in an array context", var)));
        }
        global_analysis.global_arrays.insert(var.clone(), global_analysis.global_arrays.len() as i32);
        return Ok(Some(var.clone()));
    }
    pub fn use_as_scalar(&mut self, var: &Symbol, global_analysis: &mut AnalysisResults) -> Result<Option<Symbol>, PrintableError> {
        if let Some(arg) = self.get_arg(var) {
            if let Some(arg_typ) = arg.typ {
                match arg_typ {
                    ArgT::Array => return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context", var))),
                    ArgT::Scalar => {}
                }
            } else {
                arg.typ = Some(ArgT::Scalar);
                return Ok(Some(arg.name.clone()));
            }
            return Ok(None);
        }
        if let Some(_type) = global_analysis.global_arrays.get(var) {
            return Err(PrintableError::new(format!("fatal: attempt to array `{}` in an scalar context", var)));
        }
        global_analysis.global_scalars.insert(var.clone(), global_analysis.global_arrays.len() as i32);
        return Ok(Some(var.clone()));
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