use std::fmt::Debug;
use std::hash::Hash;
use hashbrown::hash_map::Iter;
use hashbrown::HashMap;
use crate::symbolizer::Symbol;
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
    pub fn get(&self, name: &Symbol) -> Option<&Box<TypedUserFunction>> {
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