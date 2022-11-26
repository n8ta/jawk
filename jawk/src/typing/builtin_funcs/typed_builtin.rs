use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::fmt::{Display, Formatter, write};
use std::rc::Rc;
use hashbrown::HashSet;
use crate::parser::{Arg, ArgT};
use crate::PrintableError;
use crate::symbolizer::Symbol;
use crate::typing::{AnalysisResults, BuiltinFunc, ITypedFunction, TypedUserFunction};
use crate::typing::structs::Call;


#[derive(Debug)]
struct BuiltinShared {
    callers: RefCell<HashSet<Rc<TypedUserFunction>>>,
    calls: RefCell<Vec<Call>>,
}

#[derive(Debug)]
struct TypedBuiltin {
    args: RefCell<Vec<Arg>>,
    builtin: BuiltinFunc,
    arity: usize,
    name: Symbol,
    shared: Rc<BuiltinShared>, // Shared empty callers and calls sets between all builtins
}

impl TypedBuiltin {
    pub fn new(
        name: Symbol,
        args: Vec<Arg>,
        builtin: BuiltinFunc,
        shared: Rc<BuiltinShared>) -> Self {
        let arity = args.len();
        Self {
            name,
            args: RefCell::new(args),
            builtin,
            arity,
            shared,
        }
    }
}

impl Display for TypedBuiltin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "builtin-func-{}", self.builtin)
    }
}

impl ITypedFunction for TypedBuiltin {
    fn args(&self) -> Ref<'_, Vec<Arg>> {
        self.args.borrow()
    }

    fn arity(&self) -> usize {
        self.arity
    }

    fn add_caller(&self, caller: Rc<TypedUserFunction>) {}

    fn calls(&self) -> Ref<'_, Vec<Call>> {
        self.shared.calls.borrow()
    }

    fn callers(&self) -> Ref<'_, HashSet<Rc<TypedUserFunction>>> {
        self.shared.callers.borrow()
    }

    fn name(&self) -> Symbol {
        self.name.clone()
    }

    fn get_call_types(&self, _analysis: &AnalysisResults, _call: &Call) -> Vec<ArgT> {
        self.args.borrow().iter().map(|arg| arg.typ).collect()
    }

    fn reverse_call(&self, link: &Call, args: &[Arg], _analysis: &mut AnalysisResults) -> Result<HashSet<Symbol>, PrintableError> {
        Ok(HashSet::new())
    }

    fn receive_call(&self, call: &Vec<ArgT>) -> Result<HashSet<Symbol>, PrintableError> {
        for arg in call {

        }
        todo!()
    }
}