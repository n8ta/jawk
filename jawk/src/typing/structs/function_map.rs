use std::fmt::Debug;
use std::hash::Hash;
use hashbrown::hash_map::Iter;
use hashbrown::HashMap;
use crate::symbolizer::Symbol;
use crate::typing::structs::typed_function::ITypedFunction;
use crate::typing::TypedUserFunction;

pub struct FunctionMap {
    functions: HashMap<Symbol, Box<TypedUserFunction>>
}

impl FunctionMap {
    pub fn new(functions: HashMap<Symbol, Box<TypedUserFunction>>) -> Self {
        Self {
            functions
        }
    }
    // pub fn get(&self, name: &Symbol) -> Option<&Box<dyn ITypedFunction>> {
    pub fn get<'a>(&self, name: &Symbol) -> Option<Box<dyn ITypedFunction>> {
        match  self.functions.get(name) {
            None => None,
            Some(boxed) => Some(TypedUserFunction::clone(boxed)),
        }
    }
    pub fn get_user_func(&self, name: &Symbol) -> Option<&Box<TypedUserFunction>> {
        self.functions.get(name)
    }
    pub fn user_functions(&self) -> &HashMap<Symbol, Box<TypedUserFunction>> {
        &self.functions
    }
    pub fn len(&self) -> usize {
        self.functions.len()
    }
    pub fn iter(&self) -> Iter<'_, Symbol, Box<TypedUserFunction>> {
        self.user_functions().iter()
    }
}