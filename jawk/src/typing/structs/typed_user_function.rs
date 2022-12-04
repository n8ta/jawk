use crate::parser::{Arg, ArgT, Function, ScalarType};
use crate::symbolizer::Symbol;
use crate::typing::ityped_function::{ITypedFunction};
use crate::typing::reconcile::reconcile;
use crate::typing::structs::{Call, CallArg};
use crate::{AnalysisResults, PrintableError};
use hashbrown::HashSet;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Debug)]
pub struct TypedUserFunction {
    func: RefCell<Function>,
    callers: RefCell<HashSet<Rc<TypedUserFunction>>>,
    calls: RefCell<Vec<Call>>,
    return_type: RefCell<ScalarType>,
    globals_used: RefCell<HashSet<Symbol>>,
    args: RefCell<Vec<Arg>>,
    name: Symbol,
}

impl Hash for TypedUserFunction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl Eq for TypedUserFunction {}

impl PartialEq for TypedUserFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name()
    }
}

impl Display for TypedUserFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.func.borrow())
    }
}

impl ITypedFunction for TypedUserFunction {
    fn args(&self) -> Ref<'_, Vec<Arg>> {
        self.args.borrow()
    }
    fn arity(&self) -> usize {
        self.func.borrow().args.len()
    }
    fn add_caller(&self, caller: Rc<TypedUserFunction>) {
        let mut callers = self.callers.borrow_mut();
        callers.insert(caller);
    }
    fn calls(&self) -> Ref<'_, Vec<Call>> {
        self.calls.borrow()
    }
    fn callers(&self) -> Ref<'_, HashSet<Rc<TypedUserFunction>>> {
        self.callers.borrow()
    }
    fn name(&self) -> Symbol {
        self.name.clone()
    }

    fn get_call_types(&self, global_analysis: &AnalysisResults, link: &Call) -> Vec<ArgT> {
        link.args
            .iter()
            .map(|arg| match arg {
                CallArg::Variable(name) => {
                    TypedUserFunction::get_type(global_analysis, &self, &name)
                }
                CallArg::Scalar => ArgT::Scalar,
            })
            .collect()
    }

    fn reverse_call(
        &self,
        link: &Call,
        args: &Vec<Arg>,
        analysis: &mut AnalysisResults,
    ) -> Result<HashSet<Symbol>, PrintableError> {
        // Used in this case:
        //      function knows_type(arr) { arr[0[] = 1 }
        //      BEGIN { knows_type(a) }
        // reverse_call is called on main_function with call_arg: vec![CallArg::Variable(a)], args: &[ArgT::Array]
        // main_function can then mark the global a as an array (or scalar depending)
        let mut updated = HashSet::new();
        for (call_arg, function_arg) in link.args.iter().zip(args) {
            if let CallArg::Variable(name) = call_arg {
                let updated_sym = match function_arg.typ {
                    ArgT::Scalar => self.use_as_scalar(&name, analysis)?,
                    ArgT::Array => self.use_as_array(&name, analysis)?,
                    ArgT::Unknown => None,
                };
                if let Some(updated_sym) = updated_sym {
                    updated.insert(updated_sym);
                }
            }
        }
        Ok(updated)
    }
    fn receive_call(&self, call: &Vec<ArgT>) -> Result<HashSet<Symbol>, PrintableError> {
        // Used in this case:
        //      function arg_unknown(a) { ...  a not used here weirdly ... }
        //      BEGIN { c = 1; arg_unknown(c); }
        // receive_call is called on arg_unknown with call: vec![ArgT::Scalar] and
        // then arg_unknown can update its arg a to be a scalar.
        let mut function_args = self.args.borrow_mut();
        let mut updated_in_dest = HashSet::new();
        reconcile(
            call.as_slice(),
            &mut *function_args,
            self.name.clone(),
            &mut |sym| {
                updated_in_dest.insert(sym);
            },
        )?;
        Ok(updated_in_dest)
    }
}

impl TypedUserFunction {
    pub fn new(func: Function) -> Self {
        let name = func.name.clone();
        let args = func
            .args
            .iter()
            .map(|sym| Arg::new(sym.clone(), ArgT::Unknown))
            .collect();
        Self {
            func: RefCell::new(func),
            callers: RefCell::new(HashSet::new()),
            calls: RefCell::new(vec![]),
            return_type: RefCell::new(ScalarType::Variable),
            globals_used: RefCell::new(HashSet::new()),
            args: RefCell::new(args),
            name,
        }
    }

    pub fn user_func_args(&self) -> Ref<'_, Vec<Arg>> {
        self.args.borrow()
    }

    fn get_type(
        global_analysis: &AnalysisResults,
        func: &TypedUserFunction,
        name: &Symbol,
    ) -> ArgT {
        if let Some((_idx, typ)) = func.get_arg_idx_and_type(name) {
            return typ;
        }
        if global_analysis.global_scalars.contains_key(name) {
            return ArgT::Scalar;
        }
        if global_analysis.global_arrays.contains_key(name) {
            return ArgT::Array;
        }
        ArgT::Unknown
    }

    pub fn get_arg_idx_and_type(&self, name: &Symbol) -> Option<(usize, ArgT)> {
        let args = self.args.borrow();
        if let Some((idx, arg)) = args.iter().enumerate().find(|(_idx, a)| a.name == *name) {
            Some((idx, arg.typ.clone()))
        } else {
            None
        }
    }

    pub fn globals_used(&self) -> Ref<'_, HashSet<Symbol>> {
        self.globals_used.borrow()
    }

    fn use_as_array(
        &self,
        var: &Symbol,
        global_analysis: &mut AnalysisResults,
    ) -> Result<Option<Symbol>, PrintableError> {
        if let Some((_idx, typ)) = self.get_arg_idx_and_type(var) {
            match typ {
                ArgT::Scalar => {
                    return Err(PrintableError::new(format!(
                        "fatal: attempt to use scalar `{}` in a array context",
                        var
                    )));
                }
                ArgT::Array => {} // No-op type matches
                ArgT::Unknown => {
                    self.set_arg_type(var, ArgT::Array)?;
                }
            }
        }
        if let Some(_type) = global_analysis.global_scalars.get(var) {
            return Err(PrintableError::new(format!(
                "fatal: attempt to scalar `{}` in an array context",
                var
            )));
        }
        global_analysis.global_arrays.insert(&var);
        return Ok(Some(var.clone()));
    }
    fn use_as_scalar(
        &self,
        var: &Symbol,
        global_analysis: &mut AnalysisResults,
    ) -> Result<Option<Symbol>, PrintableError> {
        if let Some((_idx, typ)) = self.get_arg_idx_and_type(var) {
            match typ {
                ArgT::Scalar => {} // No-op type matches
                ArgT::Array => {
                    return Err(PrintableError::new(format!(
                        "fatal: attempt to use array `{}` in a scalar context",
                        var
                    )));
                }
                ArgT::Unknown => {
                    self.set_arg_type(var, ArgT::Scalar)?;
                }
            }
        }
        if let Some(_type) = global_analysis.global_arrays.get(var) {
            return Err(PrintableError::new(format!(
                "fatal: attempt to use array `{}` in an scalar context",
                var
            )));
        }
        global_analysis.global_scalars.insert(&var);
        return Ok(Some(var.clone()));
    }

    pub fn function(&self) -> RefMut<'_, Function> {
        self.func.borrow_mut()
    }
    pub fn add_call(&self, call: Call) {
        let mut calls = self.calls.borrow_mut();
        calls.push(call);
    }
    pub fn use_global(&self, var: &Symbol) {
        let mut globals_used = self.globals_used.borrow_mut();
        globals_used.insert(var.clone());
    }
    pub fn set_arg_type(&self, var: &Symbol, typ: ArgT) -> Result<(), PrintableError> {
        let mut args = self.args.borrow_mut();
        if let Some(arg) = args.iter_mut().find(|a| a.name == *var) {
            if arg.typ != ArgT::Unknown && arg.typ != typ {
                return Err(PrintableError::new(format!(
                    "fatal: attempt to mix array and scalar types for function {} arg {}",
                    self.func.borrow().name,
                    var
                )));
            }
            arg.typ = typ;
        }
        Ok(())
    }
}
