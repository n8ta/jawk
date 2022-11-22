use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use hashbrown::hash_map::Iter;
use hashbrown::HashMap;
use crate::symbolizer::Symbol;
use crate::typing::structs::ityped_function::ITypedFunction;
use crate::typing::TypedUserFunction;

pub struct FunctionMap {
    functions: HashMap<Symbol, Rc<TypedUserFunction>>
}

impl FunctionMap {
    pub fn new(functions: HashMap<Symbol, Rc<TypedUserFunction>>) -> Self {
        Self {
            functions
        }
    }
    pub fn get<'a>(&self, name: &Symbol) -> Option<Rc<dyn ITypedFunction>> {
        match  self.functions.get(name) {
            None => None,
            Some(boxed) => Some(boxed.clone()),
        }
    }
    pub fn get_user_func(&self, name: &Symbol) -> Option<&Rc<TypedUserFunction>> {
        self.functions.get(name)
    }
    pub fn user_functions(&self) -> &HashMap<Symbol, Rc<TypedUserFunction>> {
        &self.functions
    }
    pub fn len(&self) -> usize {
        self.functions.len()
    }
    pub fn iter(&self) -> Iter<'_, Symbol, Rc<TypedUserFunction>> {
        self.user_functions().iter()
    }
}