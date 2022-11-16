use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use immutable_chunkmap::map::Map;
use crate::global_scalars::SymbolMapping;
use crate::parser::{Arg, ArgT, Function, Program, ScalarType, Stmt};
use crate::PrintableError;
use crate::symbolizer::Symbol;
use crate::typing::inference_pass::CallLink;
use crate::typing::TypedFunc;

#[derive(Clone, Debug)]
enum VarType {
    Float,
    String,
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

#[derive(Debug, PartialEq)]
pub struct AnalysisResults {
    pub global_scalars: SymbolMapping,
    pub global_arrays: SymbolMapping,
    pub str_consts: HashSet<Symbol>,
}

impl AnalysisResults {
    pub fn new() -> Self {
        Self {
            global_scalars: SymbolMapping::new(),
            global_arrays: SymbolMapping::new(),
            str_consts: Default::default(),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct Call {
    pub target: TypedFunc,
    pub args: Vec<CallArg>,
}

impl Debug for Call {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"target: {:?}, args: {:?}", self.target.name(), self.args)
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
    pub fn new(target: TypedFunc, args: Vec<CallArg>) -> Self {
        Self { target, args }
    }
}


pub struct TypedProgram {
    pub functions: HashMap<Symbol, TypedFunc>,
    pub global_analysis: AnalysisResults,
}

impl Display for TypedProgram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Tests will print the program and compare it with another string
        // keep function order consistent by sorting.
        let mut sorted: Vec<Symbol> = self.functions.iter().map(|(sym, _)| sym.clone()).collect();
        sorted.sort();
        for func_name in &sorted {
            let func = self.functions.get(func_name).unwrap();
            write!(f, "{}\n", func)?;
        }
        Ok(())
    }
}

impl TypedProgram {
    pub fn new(functions: HashMap<Symbol, TypedFunc>, results: AnalysisResults) -> Self {
        Self { functions, global_analysis: results }
    }
}