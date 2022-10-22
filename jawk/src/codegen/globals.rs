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
    // mapping: SymbolMapping,
    // global_scalar_allocation: Vec<i64>,
    arrays: SymbolMapping,
    const_str_allocation: Vec<*mut String>,
    const_str_mapping: HashMap<Symbol, usize>,
    scalars: HashMap<Symbol, ValueT>
}

impl Globals {
    pub fn new<RuntimeT: Runtime>(
        analysis: AnalysisResults,
        runtime: &mut RuntimeT,
        function: &mut Function,
        _symbolizer: &mut Symbolizer) -> Self {
        // let scalar_memory = 3 * analysis.global_scalars.len() + 3 * analysis.str_consts.len();
        let const_str_memory = analysis.str_consts.len();

        // let global_scalar_allocation: Vec<i64> = Vec::with_capacity(scalar_memory);
        let mut const_str_allocation: Vec<*mut String> = Vec::with_capacity(const_str_memory);

        let mut const_str_mapping = HashMap::new();
        for (idx, str) in analysis.str_consts.iter().enumerate() {
            const_str_mapping.insert(str.clone(), idx);
            let str: Rc<String> = Rc::new(str.to_str().to_string());
            const_str_allocation.push(Rc::into_raw(str) as *mut String)
        }

        let mut scalars = HashMap::new();
        let string_tag = function.create_sbyte_constant(STRING_TAG);
        let zero_f = function.create_float64_constant(0.0);
        for (name, _) in analysis.global_scalars.mapping().clone() {
            let tag = function.create_value_int();
            let float = function.create_value_float64();
            let ptr = function.create_value_void_ptr();
            let ptr_const =function.create_void_ptr_constant(runtime.init_empty_string() as *mut c_void);
            function.insn_store(&tag, &string_tag);
            function.insn_store(&float, &zero_f);
            function.insn_store(&ptr, &ptr_const);

            let val = ValueT::string(tag, float, ptr);
            scalars.insert(name, val);
        }

        Self {
            // global_scalar_allocation,
            // mapping: analysis.global_scalars,
            arrays: analysis.global_arrays,
            const_str_allocation,
            const_str_mapping,
            scalars,
        }
    }

    // fn ptrs(&self,
    //         name: &Symbol,
    //         function: &mut Function) -> ValuePtrT {
    //     let idx = self.mapping.get(name).expect(&format!("symbol not mapped to a global `{}`", name));
    //     self.ptrs_idx(*idx, function)
    // }
    //
    // fn ptrs_idx(&self,
    //             idx: i32,
    //             function: &mut Function) -> ValuePtrT {
    //     unsafe {
    //         let alloc_ptr = (*self.global_scalar_allocation).as_ptr();
    //         let tag = alloc_ptr.offset((3 * idx) as isize);
    //         let float = alloc_ptr.offset((3 * idx + 1) as isize);
    //         let ptr = alloc_ptr.offset((3 * idx + 2) as isize);
    //         let tag_ptr_const = function.create_void_ptr_constant(tag as *mut c_void);
    //         let float_ptr_const = function.create_void_ptr_constant(float as *mut c_void);
    //         let ptr_ptr_const = function.create_void_ptr_constant(ptr as *mut c_void);
    //         ValuePtrT::new(tag_ptr_const, float_ptr_const, ptr_ptr_const, ScalarType::Variable)
    //     }
    // }

    pub fn set(&self,
               function: &mut Function,
               name: &Symbol,
               value: &ValueT) {
        let existing_value = self.scalars.get(name).unwrap();


        function.insn_store(&existing_value.tag, &value.tag);
        function.insn_store(&existing_value.float, &value.float);
        function.insn_store(&existing_value.pointer, &value.pointer);
    }

    pub fn get(&self, name: &Symbol, function: &mut Function) -> Result<ValueT, PrintableError> {
        let mut scalar = self.scalars.get(name).expect("global to exist").clone();
        scalar.typ = ScalarType::Variable;
        Ok(scalar)
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
        let mut values: Vec<ValueT> = Vec::with_capacity(self.scalars.len());
        for (_name, value) in &self.scalars {
            let mut cloned = value.clone();
            cloned.typ = ScalarType::Variable;
            values.push(cloned);
        }
        values
    }

    pub fn get_array(&self, name: &Symbol, function: &mut Function) -> Result<Value, PrintableError> {
        let idx = self.arrays.get(name).expect(&format!("expected array to exist `{}`", name));
        Ok(function.create_int_constant(*idx))
    }
}