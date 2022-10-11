use crate::codegen::ValuePtrT;
use crate::printable_error::PrintableError;
use std::collections::HashMap;
use std::thread::scope;
use gnu_libjit::Value;
use crate::symbolizer::Symbol;


pub struct Scope {
    pub scalars: HashMap<Symbol, ValuePtrT>,
    pub arrays: HashMap<Symbol, Value>,
}

impl Scope {
    pub fn new() -> Self {
        Self { scalars: HashMap::new(), arrays: HashMap::new() }
    }
}

pub struct Scopes {
    levels: Vec<Scope>,
}

impl Scopes {
    pub fn new() -> Self {
        Scopes {
            levels: vec![Scope::new()],
        }
    }
    pub fn insert_scalar(&mut self, name: Symbol, value: ValuePtrT) -> Result<(), PrintableError> {
        let mut inner_scope = self.levels.last_mut().unwrap();

        if inner_scope.scalars.get(&name).is_some() {
            return Err(PrintableError::new(format!(
                "Name {} is already in used, cannot shadow it", &name
            )));
        }
        inner_scope.scalars.insert(name, value);
        Ok(())
    }

    pub fn insert_array(&mut self, name: Symbol, value: Value) -> Result<(), PrintableError> {
        let mut inner_scope = self.levels.last_mut().unwrap();

        if inner_scope.arrays.get(&name).is_some() {
            return Err(PrintableError::new(format!(
                "Name {} is already in used, cannot shadow it", &name
            )));
        }
        inner_scope.arrays.insert(name, value);
        Ok(())
    }

    pub fn get_scalar(&self, name: &Symbol) -> Result<&ValuePtrT, PrintableError> {
        for scope in self.levels.iter().rev() {
            if let Some(scalar) = scope.scalars.get(name) {
                return Ok(scalar)
            }
        }
        panic!("Scalar {} does not exist", name);
    }

    pub fn get_array(&self, name: &Symbol) -> Result<&Value, PrintableError> {
        for scope in self.levels.iter().rev() {
            if let Some(arr) = scope.arrays.get(name) {
                return Ok(arr)
            }
        }
        panic!("Array {} does not exist", name);
    }

    pub fn open_scope(&mut self) {
        self.levels.push(Scope::new())
    }
    pub fn close_scope(&mut self) -> Scope {
        self.levels.pop().unwrap()
    }
}
