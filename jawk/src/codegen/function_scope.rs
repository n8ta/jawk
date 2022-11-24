use std::os::raw::c_void;
use gnu_libjit::{Function, Value};
use hashbrown::{HashMap};
use crate::codegen::globals::Globals;
use crate::codegen::ValueT;
use crate::global_scalars::SymbolMapping;
use crate::parser::{Arg, ArgT};
use crate::PrintableError;
use crate::symbolizer::Symbol;

// Global variables are stored on the heap. Loading and storing to the heap is expensive so
// the function scopes acts as a cache of the globals by storing a copy of needed globals as
// function locals. The flush() functions writes all local copies of globals back to the heap.
// This is used before a function call so that other function see the latest value of globals.
pub struct FunctionScope<'a> {
    globals: &'a Globals,
    local_globals: HashMap<Symbol, ValueT>,
    pure_local_array: HashMap<Symbol, Value>,
    pure_local_scalar: HashMap<Symbol, ValueT>,
}

impl<'a> FunctionScope<'a> {
    pub fn args(&self) -> &HashMap<Symbol, ValueT> {
        &self.pure_local_scalar
    }
    pub fn new(globals: &'a Globals, function: &mut Function, args: &[Arg]) -> Self {
        let mut function_scope = Self {
            globals,
            local_globals: HashMap::with_capacity(1),
            pure_local_scalar: HashMap::with_capacity(1),
            pure_local_array: HashMap::with_capacity(1),
        };
        let mut idx: i32 = 0;
        for arg in args {
            match arg.typ {
                ArgT::Scalar => {
                    let value = ValueT::new(function.create_value_int(), function.create_value_float64(), function.create_value_void_ptr());
                    let tag = function.arg(idx).unwrap();
                    let float = function.arg(idx + 1).unwrap();
                    let pointer = function.arg(idx + 2).unwrap();
                    // Load args into stack variable
                    function.insn_store(&value.tag, &tag);
                    function.insn_store(&value.float, &float);
                    function.insn_store(&value.pointer, &pointer);
                    function_scope.pure_local_scalar.insert(arg.name.clone(), value);
                    idx += 3;
                }
                ArgT::Array => {
                    let value = function.create_value_int();
                    let arr = function.arg(idx).unwrap();
                    function.insn_store(&value, &arr);
                    function_scope.pure_local_array.insert(arg.name.clone(), value);
                    idx += 1;
                }
                ArgT::Unknown => {}
            }
        };
        function_scope
    }
    pub fn get_scalar(&mut self, function: &mut Function, name: &Symbol) -> Result<ValueT, PrintableError> {
        if let Some(local) = self.pure_local_scalar.get(name) {
            Ok(local.clone())
        } else if let Some(local_global) = self.local_globals.get(name) {
            Ok(local_global.clone())
        } else {
            let global_value = self.globals.get(name, function)?;
            let mut local_global = ValueT::new(function.create_value_int(), function.create_value_float64(), function.create_value_void_ptr());
            self.store(function, &mut local_global, &global_value);
            self.local_globals.insert(name.clone(), local_global.clone());
            Ok(local_global)
        }
    }

    pub fn return_value(&mut self, function: &mut Function, value: &ValueT) {
        self.globals.return_value(function, value);
    }

    pub fn get_returned_value(&mut self, function: &mut Function) -> ValueT {
        self.globals.get_returned_value(function)
    }

    pub fn global_scalars(&self) -> &SymbolMapping {
        self.globals.global_scalars()
    }

    pub fn set_scalar(&mut self, function: &mut Function, name: &Symbol, value: &ValueT) {
        let place_to_store = if let Some(local) = self.pure_local_scalar.get_mut(name) {
            Some(local.clone())
        } else if let Some(local_global) = self.local_globals.get_mut(name) {
            // We already have this global pulled in as a stack var
            Some(local_global.clone())
        } else {
            // Create a new stack var for it
            let mut local_global = ValueT::new(function.create_value_int(), function.create_value_float64(), function.create_value_void_ptr());
            self.store(function, &mut local_global, value);
            self.local_globals.insert(name.clone(), local_global);
            None
        };
        if let Some(mut place_to_store) = place_to_store {
            self.store(function, &mut place_to_store, value);
        }
    }

    fn store(&mut self, function: &mut Function, local_global: &mut ValueT, new_value: &ValueT) {
        function.insn_store(&local_global.tag, &new_value.tag);
        function.insn_store(&local_global.float, &new_value.float);
        function.insn_store(&local_global.pointer, &new_value.pointer);
    }

    pub fn flush(&mut self, function: &mut Function) {
        for (name, local_global) in &self.local_globals {
            self.globals.set(function, name, local_global)
        }
        self.local_globals.clear();
    }

    pub fn get_array(&mut self, function: &mut Function, name: &Symbol) -> Result<Value, PrintableError> {
        if let Some(val) = self.pure_local_array.get(name) {
            return Ok(val.clone());
        }
        self.globals.get_array(function, name)
    }

    pub fn get_const_str(&self, name: &Symbol) -> Result<*mut c_void, PrintableError> {
        self.globals.get_const_str(name)
    }

    pub fn debug_mapping(&self) -> HashMap<String, String> {
        self.globals.debug_mapping()
    }
}