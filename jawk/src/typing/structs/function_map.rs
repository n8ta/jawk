use std::rc::Rc;
use hashbrown::hash_map::Iter;
use hashbrown::HashMap;
use crate::Symbolizer;
use crate::symbolizer::Symbol;
use crate::typing::builtin_funcs::BuiltinFactory;
use crate::typing::ityped_function::ITypedFunction;
use crate::typing::{BuiltinFunc, TypedUserFunction};

pub struct FunctionMap {
    functions: HashMap<Symbol, Rc<TypedUserFunction>>,
    builtin_factory: BuiltinFactory,
}

impl FunctionMap {
    pub fn new(functions: HashMap<Symbol, Rc<TypedUserFunction>>, symbolizer: &Symbolizer) -> Self {
        Self {
            functions,
            builtin_factory: BuiltinFactory::new(symbolizer.clone()),
        }
    }
    pub fn get<'a>(&mut self, name: &Symbol) -> Option<Rc<dyn ITypedFunction>> {
        match self.functions.get(name) {
            None => {
              if let Some(builtin) = BuiltinFunc::get(name.to_str()) {
                  Some(self.builtin_factory.get(builtin))
              }  else {
                  None
              }
            },
            Some(boxed) => Some(boxed.clone()),
        }
    }
    pub fn get_user_function<'a>(&self, name: &Symbol) -> Option<Rc<TypedUserFunction>> {
        match self.functions.get(name) {
            None => None,
            Some(boxed) => Some(boxed.clone()),
        }
    }
    pub fn user_functions(&self) -> &HashMap<Symbol, Rc<TypedUserFunction>> {
        &self.functions
    }
    pub fn len(&self) -> usize {
        self.functions.len()
    }
    pub fn user_functions_iter(&self) -> Iter<'_, Symbol, Rc<TypedUserFunction>> {
        self.user_functions().iter()
    }
}