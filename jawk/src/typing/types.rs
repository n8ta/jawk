use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
use hashbrown::{HashMap, HashSet};
use immutable_chunkmap::map::Map;
use crate::global_scalars::SymbolMapping;
use crate::parser::{Arg, ArgT, Function, Program, ScalarType, Stmt};
use crate::PrintableError;
use crate::symbolizer::Symbol;

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
    pub fn new(target: TypedFunc, args: Vec<CallArg>) -> Self {
        Self { target, args }
    }
}


#[derive(Debug)]
struct TypedFuncInner {
    func: RefCell<Function>,
    callers: RefCell<HashSet<Symbol>>,
    calls: RefCell<Vec<Call>>,
    return_type: RefCell<ScalarType>,
    globals_used: RefCell<HashSet<Symbol>>,
    args: RefCell<Vec<Arg>>
}

#[derive(Clone, Debug)]
pub struct TypedFunc {
    inner: Rc<TypedFuncInner>,
}
impl PartialEq for TypedFunc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Display for TypedFunc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner.func.borrow())
    }
}

impl TypedFunc {
    pub fn new(func: Function) -> Self {
        let args = func.args.iter().map(|sym| Arg::new(sym.clone(), ArgT::Unknown)).collect();
        Self {
            inner: Rc::new(TypedFuncInner {
                func: RefCell::new(func),
                callers: RefCell::new(HashSet::new()),
                calls: RefCell::new((vec![])),
                return_type: RefCell::new(ScalarType::Variable),
                globals_used: RefCell::new(HashSet::new()),
                args: RefCell::new(args),
            })
        }
    }

    pub fn args(&self) -> Ref<'_, Vec<Arg>> {
        self.inner.args.borrow()
    }

    pub fn function(&self) -> RefMut<'_, Function> {
        self.inner.func.borrow_mut()
    }

    pub fn add_call(&self, call: Call) {
        let mut calls = self.inner.calls.borrow_mut();
        calls.push(call);
    }

    pub fn globals_used(&self) -> Ref<'_, HashSet<Symbol>> {
        self.inner.globals_used.borrow()
    }

    pub fn calls(&self) -> Ref<'_, Vec<Call>> {
        self.inner.calls.borrow()
    }

    pub fn name(&self) -> Symbol {
        self.inner.func.borrow().name.clone()
    }

    pub fn arity(&self) -> usize {
        self.inner.func.borrow().args.len()
    }

    pub fn get_arg_idx_and_type(&self, name: &Symbol) -> Option<(usize, ArgT)> {
        let inner = self.inner.args.borrow();
        if let Some((idx, arg)) = inner.iter().enumerate().find(|(idx, a)| a.name == *name) {
            Some((idx, arg.typ.clone()))
        } else {
            None
        }
    }

    pub fn use_global(&self, var: &Symbol) {
        let mut globals_used = self.inner.globals_used.borrow_mut();
        globals_used.insert(var.clone());

    }

    pub fn set_arg_type(&self, var: &Symbol, typ: ArgT) -> Result<(),PrintableError> {
        let mut inner = self.inner.args.borrow_mut();
        if let Some(arg) = inner.iter_mut().find(|a| a.name == *var) {
            if arg.typ != ArgT::Unknown && arg.typ != typ {
                return Err(PrintableError::new(format!("fatal: attempt to mix array and scalar types for function {} arg {}", self.inner.func.borrow().name, var)))
            }
            arg.typ = typ;
        }
        Ok(())
    }

    pub fn receive_call(&self, call: Vec<ArgT>, global_analysis: &mut AnalysisResults) -> Result<Vec<Symbol>, PrintableError> {
        let mut args = self.inner.args.borrow_mut();
        let mut updated_in_dest = vec![];
        for (func_arg, call_arg) in args.iter_mut().zip(call.iter()) {
            match (func_arg.typ, call_arg) {
                // Mismatch
                (ArgT::Scalar, ArgT::Array) => return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context", func_arg.name))),
                (ArgT::Array, ArgT::Scalar) => return Err(PrintableError::new(format!("fatal: attempt to use scalar `{}` in a array context", func_arg.name))),
                // Function doesn't known arg type so just accept caller type
                (ArgT::Unknown, ArgT::Scalar)
                | (ArgT::Unknown, ArgT::Array)
                | (ArgT::Unknown, ArgT::Unknown) => {
                    func_arg.typ = call_arg.clone();
                    updated_in_dest.push(func_arg.name.clone());
                },
                (ArgT::Scalar, ArgT::Scalar) | (ArgT::Array, ArgT::Array) => {}
                (ArgT::Scalar, ArgT::Unknown) => {} // TODO back prop
                (ArgT::Array, ArgT::Unknown) => {} // TODO back prop
            }
        }
        Ok(updated_in_dest)
    }

    pub fn use_as_array(&self, var: &Symbol, global_analysis: &mut AnalysisResults) -> Result<Option<Symbol>, PrintableError> {
        if let Some((_idx, typ)) = self.get_arg_idx_and_type(var) {
            match typ {
                ArgT::Scalar => return Err(PrintableError::new(format!("fatal: attempt to use scalar `{}` in a array context", var))),
                ArgT::Array => {} // No-op type matches
                ArgT::Unknown => {self.set_arg_type(var, ArgT::Array)?;},
            }
        }
        if let Some(_type) = global_analysis.global_scalars.get(var) {
            return Err(PrintableError::new(format!("fatal: attempt to scalar `{}` in an array context", var)));
        }
        global_analysis.global_arrays.insert(&var);
        return Ok(Some(var.clone()));
    }

    pub fn use_as_scalar(&self, var: &Symbol, global_analysis: &mut AnalysisResults) -> Result<Option<Symbol>, PrintableError> {
        if let Some((_idx, typ)) = self.get_arg_idx_and_type(var) {
            match typ {
                ArgT::Scalar => {} // No-op type matches
                ArgT::Array => return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in a scalar context", var))),
                ArgT::Unknown => {self.set_arg_type(var, ArgT::Scalar)?;},
            }
        }
        if let Some(_type) = global_analysis.global_scalars.get(var) {
            return Err(PrintableError::new(format!("fatal: attempt to use array `{}` in an scalar context", var)));
        }
        global_analysis.global_arrays.insert(&var);
        return Ok(Some(var.clone()));
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