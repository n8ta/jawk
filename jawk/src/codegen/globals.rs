use std::collections::HashMap;
use std::os::raw::c_void;
use std::rc::Rc;
use gnu_libjit::{Context, Function, Value};
use crate::{AnalysisResults, PrintableError, Symbolizer};
use crate::codegen::{STRING_TAG, ValuePtrT, ValueT};
use crate::global_scalars::SymbolMapping;
use crate::parser::ScalarType;
use crate::runtime::Runtime;
use crate::symbolizer::Symbol;

pub struct Globals {
    mapping: SymbolMapping,
    global_scalar_allocation: Vec<i64>,
    arrays: SymbolMapping,
    const_str_allocation: Vec<*mut String>,
    const_str_mapping: HashMap<Symbol, usize>,
}

impl Globals {
    pub fn new<RuntimeT: Runtime>(
        analysis: AnalysisResults,
        runtime: &mut RuntimeT,
        function: &mut Function,
        _symbolizer: &mut Symbolizer) -> Self {
        let scalar_memory = 3 * analysis.global_scalars.len() + 3 * analysis.str_consts.len();
        let const_str_memory = analysis.str_consts.len();

        let global_scalar_allocation: Vec<i64> = Vec::with_capacity(scalar_memory);
        let mut const_str_allocation: Vec<*mut String> = Vec::with_capacity(const_str_memory);

        let mut const_str_mapping = HashMap::new();
        for (idx, str) in analysis.str_consts.iter().enumerate() {
            const_str_mapping.insert(str.clone(), idx);
            let str: Rc<String> = Rc::new(str.to_str().to_string());
            const_str_allocation.push(Rc::into_raw(str) as *mut String)
        }

        let mut init = Self {
            global_scalar_allocation,
            mapping: analysis.global_scalars,
            arrays: analysis.global_arrays,
            const_str_allocation,
            const_str_mapping,
        };

        for (name, _) in init.mapping.mapping().clone() {
            let ptr = runtime.empty_string(function);
            let val = ValueT::string(function.create_sbyte_constant(STRING_TAG), function.create_float64_constant(0.0), ptr);
            init.set(function, &name, &val)
        }

        init
    }

    fn ptrs(&self,
            name: &Symbol,
            function: &mut Function) -> ValuePtrT {
        let idx = self.mapping.get(name).expect(&format!("symbol not mapped to a global `{}`", name));
        self.ptrs_idx(*idx, function)
    }

    fn ptrs_idx(&self,
                idx: i32,
                function: &mut Function) -> ValuePtrT {
        unsafe {
            let alloc_ptr = (*self.global_scalar_allocation).as_ptr();
            let tag = alloc_ptr.offset((3 * idx) as isize);
            let float = alloc_ptr.offset((3 * idx + 1) as isize);
            let ptr = alloc_ptr.offset((3 * idx + 2) as isize);
            let tag_ptr_const = function.create_void_ptr_constant(tag as *mut c_void);
            let float_ptr_const = function.create_void_ptr_constant(float as *mut c_void);
            let ptr_ptr_const = function.create_void_ptr_constant(ptr as *mut c_void);
            ValuePtrT::new(tag_ptr_const, float_ptr_const, ptr_ptr_const, ScalarType::Variable)
        }
    }

    pub fn set(&self,
               function: &mut Function,
               name: &Symbol,
               value: &ValueT) {
        let ptrs = self.ptrs(&name, function);
        function.insn_store_relative(&ptrs.tag, 0, &value.tag);
        function.insn_store_relative(&ptrs.float, 0, &value.float);
        function.insn_store_relative(&ptrs.pointer, 0, &value.pointer);
    }

    pub fn get(&self, name: &Symbol, function: &mut Function) -> Result<ValueT, PrintableError> {
        let ptrs = self.ptrs(&name, function);
        Ok(self.load_value(ptrs, function))
    }

    fn load_value(&self, ptrs: ValuePtrT, function: &mut Function) -> ValueT {
        let tag = function.insn_load_relative(&ptrs.tag, 0, &Context::sbyte_type());
        let float = function.insn_load_relative(&ptrs.float, 0, &Context::float64_type());
        let ptr = function.insn_load_relative(&ptrs.pointer, 0, &Context::void_ptr_type());
        ValueT::var(tag, float, ptr)
    }

    pub fn get_const_str(&self, name: &Symbol) -> Result<*mut c_void, PrintableError> {
        let idx = self.const_str_mapping.get(name).unwrap();
        let alloc_ptr = self.const_str_allocation[*idx];
        Ok(alloc_ptr as *mut c_void)
    }

    pub fn scalars(&self, function: &mut Function) -> Vec<ValueT> {
        // TODO: This should return an iterator skipping the vec allocation
        let mut values: Vec<ValueT> = Vec::with_capacity(self.mapping.len());
        for (_, idx) in self.mapping.mapping() {
            let ptr = self.ptrs_idx(*idx, function);
            values.push(self.load_value(ptr, function));
        }
        values
    }

    pub fn get_array(&self, name: &Symbol, function: &mut Function) -> Result<Value, PrintableError> {
        let idx = self.arrays.get(name).expect(&format!("expected array to exist `{}`", name));
        Ok(function.create_int_constant(*idx))
    }
}