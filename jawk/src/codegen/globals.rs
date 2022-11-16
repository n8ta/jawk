use std::os::raw::c_void;
use std::rc::Rc;
use gnu_libjit::{Context, Function, Value};
use hashbrown::HashMap;
use crate::{AnalysisResults, PrintableError, Symbolizer};
use crate::codegen::{STRING_TAG, ValuePtrT, ValueT};
use crate::global_scalars::SymbolMapping;
use crate::runtime::Runtime;
use crate::symbolizer::Symbol;

pub struct Globals {
    mapping: SymbolMapping,
    global_scalar_allocation: Vec<i64>,
    arrays: SymbolMapping,
    const_str_allocation: Vec<*mut String>,
    const_str_mapping: HashMap<Symbol, usize>,
    global_return_value: Vec<i64>,
}

impl Globals {
    pub fn new<RuntimeT: Runtime>(
        analysis: AnalysisResults,
        runtime: &mut RuntimeT,
        function: &mut Function,
        _symbolizer: &mut Symbolizer) -> Self {
        // global scalars + str_consts + return_value
        let scalar_memory = 3 * analysis.global_scalars.len();
        let const_str_memory = analysis.str_consts.len();

        let global_scalar_allocation: Vec<i64> = Vec::with_capacity(scalar_memory);
        let global_return_value: Vec<i64> = Vec::with_capacity(3);
        let mut const_str_allocation: Vec<*mut String> = Vec::with_capacity(const_str_memory);

        let mut const_str_mapping = HashMap::new();
        for (idx, str) in analysis.str_consts.iter().enumerate() {
            const_str_mapping.insert(str.clone(), idx);
            let str: Rc<String> = Rc::new(str.to_str().to_string());
            const_str_allocation.push(Rc::into_raw(str) as *mut String)
        }

        let init = Self {
            global_scalar_allocation,
            mapping: analysis.global_scalars,
            arrays: analysis.global_arrays,
            global_return_value,
            const_str_allocation,
            const_str_mapping,
        };

        for (name, _) in init.mapping.mapping().clone() {
            let ptr = runtime.init_empty_string() as *mut c_void;
            let ptr_const = function.create_void_ptr_constant(ptr);
            let val = ValueT::string(function.create_sbyte_constant(STRING_TAG), function.create_float64_constant(0.0), ptr_const);
            init.set(function, &name, &val)
        }

        init
    }

    pub fn get_returned_value(&self, function: &mut Function) -> ValueT {
        let ptrs = Globals::ptrs_idx(&self.global_return_value, 0, function);
        self.load_value(ptrs, function)
    }

    pub fn return_value(&self, function: &mut Function, value: &ValueT) {
        let ptrs = Globals::ptrs_idx(&self.global_return_value, 0, function);
        Globals::store(function, &ptrs, value)
    }

    // Maps from pointer as a string to the name of the variable
    pub fn debug_mapping(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        unsafe {
            let alloc_ptr = (*self.global_scalar_allocation).as_ptr();
            for sym in self.mapping.all_symbols() {
                let idx = self.mapping.get(&sym).unwrap();
                let tag = alloc_ptr.offset((3 * idx) as isize) as i64;
                let float = alloc_ptr.offset((3 * idx + 1) as isize) as i64;
                let ptr = alloc_ptr.offset((3 * idx + 2) as isize) as i64;
                map.insert(tag.to_string(), format!("{}-tag", sym.to_str()));
                map.insert(float.to_string(), format!("{}-float", sym.to_str()));
                map.insert(ptr.to_string(), format!("{}-ptr", sym.to_str()));
            }
        }
        map
    }

    fn ptrs(&self,
            name: &Symbol,
            function: &mut Function) -> ValuePtrT {
        let idx = self.mapping.get(name).expect(&format!("symbol not mapped to a global `{}`", name));
        Globals::ptrs_idx(&self.global_scalar_allocation, *idx, function)
    }

    fn ptrs_idx(allocation : &Vec<i64>,
                idx: i32,
                function: &mut Function) -> ValuePtrT {
        unsafe {
            let alloc_ptr = (*allocation).as_ptr();
            let tag = alloc_ptr.offset((3 * idx) as isize);
            let float = alloc_ptr.offset((3 * idx + 1) as isize);
            let ptr = alloc_ptr.offset((3 * idx + 2) as isize);
            let tag_ptr_const = function.create_void_ptr_constant(tag as *mut c_void);
            let float_ptr_const = function.create_void_ptr_constant(float as *mut c_void);
            let ptr_ptr_const = function.create_void_ptr_constant(ptr as *mut c_void);
            ValuePtrT::new(tag_ptr_const, float_ptr_const, ptr_ptr_const)
        }
    }

    pub fn set(&self,
               function: &mut Function,
               name: &Symbol,
               value: &ValueT) {
        let ptrs = self.ptrs(&name, function);
        Globals::store(function, &ptrs, value)
    }

    fn store(function: &mut Function, ptrs: &ValuePtrT, value: &ValueT) {
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

    pub fn get_array(&self, function: &mut Function, name: &Symbol) -> Result<Value, PrintableError> {
        let idx = self.arrays.get(name).expect(&format!("expected array to exist `{}`", name));
        Ok(function.create_int_constant(*idx))
    }
}