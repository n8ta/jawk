mod native;
mod string_tracker;

use crate::codegen::{ValueT};
use crate::columns::Columns;
use crate::lexer::BinOp;
use crate::parser::ScalarType;
use crate::runtime::arrays::Arrays;
use crate::runtime::call_log::CallLog;
use crate::runtime::{ErrorCode, Runtime};
use crate::{runtime_fn, runtime_fn_no_ret};
use gnu_libjit::{Abi, Context, Function, Value};
use hashbrown::HashMap;
use std::ffi::c_void;
use crate::awk_str::AwkStr;
use crate::runtime::debug_runtime::native::{index, array_access, array_assign, binop, column, concat, concat_array_indices, copy_if_string, copy_string, empty_string, free_if_string, free_string, in_array, length, next_line, number_to_string, print_error, print_float, print_string, printf, rand, split, split_ere, srand, string_to_number, substr, substr_max_chars, to_lower, to_upper};
use crate::runtime::debug_runtime::string_tracker::StringTracker;
use crate::runtime::string_converter::Converter;

pub const CANARY: &str = "this is the canary!";

// Helper for build debug mapping form pointers to their runtime function
fn insert(mapping: &mut HashMap<String, String>, ptr: *mut c_void, name: &str) {
    let ptr_hex = format!("0x{:x}", ptr as i64);
    let with_name = format!("{} 0x{:x}", name, ptr as i64);
    mapping.insert(ptr_hex, with_name);
}

pub struct DebugRuntime {
    runtime_data: *mut c_void,
}

pub struct RuntimeData {
    srand_seed: f64,
    columns: Columns,
    canary: String,
    output: Vec<u8>,
    calls: CallLog,
    str_tracker: StringTracker,
    arrays: Arrays,
    converter: Converter,
}

impl RuntimeData {

    pub fn new(files: Vec<String>) -> RuntimeData {
        unsafe { libc::srand(09171998) }
        RuntimeData {
            canary: String::from(CANARY),
            columns: Columns::new(files),
            output: vec![],
            calls: CallLog::new(),
            str_tracker: StringTracker::new(),
            arrays: Arrays::new(),
            srand_seed: 09171998.0,
            converter: Converter::new(),
        }
    }
}

impl DebugRuntime {
    #[allow(dead_code)]
    pub fn output(&self) -> String {
        String::from_utf8(cast_to_runtime_data(self.runtime_data).output.clone()).unwrap()
    }
    #[allow(dead_code)]

    pub fn output_bytes(&self) -> String {
        String::from_utf8(cast_to_runtime_data(self.runtime_data).output.clone()).unwrap()
    }
    #[allow(dead_code)]
    pub fn strings_in(&self) -> usize {
        cast_to_runtime_data(self.runtime_data).str_tracker.strings_in
    }
    #[allow(dead_code)]
    pub fn strings_out(&self) -> usize {
        cast_to_runtime_data(self.runtime_data).str_tracker.string_out
    }

    #[allow(dead_code)]
    fn data_ptr(&mut self, func: &mut Function) -> Value {
        func.create_void_ptr_constant(self.runtime_data as *mut c_void)
    }
}

impl Runtime for DebugRuntime {
    fn new(_context: &Context, files: Vec<String>) -> DebugRuntime {
        let data = Box::new(RuntimeData::new(files));
        let runtime_data = (Box::leak(data) as *mut RuntimeData) as *mut c_void;
        DebugRuntime { runtime_data }
    }

    fn init_empty_string(&mut self) -> *const AwkStr {
        empty_string(self.runtime_data as *mut c_void)
    }

    fn binop(&mut self, func: &mut Function, left: ValueT, right: ValueT, binop_val: BinOp) -> Value {
        let binop_val = func.create_sbyte_constant(binop_val as i8);
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            binop as *mut c_void,
            vec![data_ptr, left.tag, left.float, left.pointer, right.tag, right.float, right.pointer, binop_val],
            Some(Context::float64_type()),
            Abi::Cdecl,
        )
    }

    fn print_error(&mut self, func: &mut Function, error: ErrorCode) {
        let binop = func.create_sbyte_constant(error as i8);
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            print_error as *mut c_void,
            vec![data_ptr, binop],
            None,
            Abi::Cdecl,
        );
    }

    fn allocate_arrays(&mut self, count: usize) {
        let data = cast_to_runtime_data(self.runtime_data);
        data.arrays.allocate(count);
    }

    fn printf(&mut self, func: &mut Function, fstring: Value, nargs: Value, args: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            printf as *mut c_void,
            vec![data_ptr, fstring, nargs, args],
            None,
            Abi::VarArg,
        );
    }

    runtime_fn!(call_next_line, next_line, Some(Context::float64_type()),);
    runtime_fn!(column,column,Some(Context::void_ptr_type()),tag: Value,float: Value,pointer: Value);
    runtime_fn!(string_to_number,string_to_number,Some(Context::float64_type()),arg0: Value);
    runtime_fn!(number_to_string,number_to_string,Some(Context::void_ptr_type()),arg0: Value);
    runtime_fn_no_ret!(print_string, print_string, None, arg0: Value);
    runtime_fn_no_ret!(print_float, print_float, None, arg0: Value);
    runtime_fn!(concat,concat,Some(Context::void_ptr_type()),arg0: Value,arg1: Value);
    runtime_fn!(concat_array_indices,concat_array_indices,Some(Context::void_ptr_type()),arg0: Value,arg1: Value);
    runtime_fn!(empty_string, empty_string, Some(Context::void_ptr_type()),);
    runtime_fn_no_ret!(array_access,array_access,None,array: Value,key_tag: Value,key_num: Value,key_ptr: Value,out_tag_ptr: Value,out_float_ptr: Value,out_ptr_ptr: Value);
    runtime_fn_no_ret!(array_assign,array_assign,None,array: Value,key_tag: Value,key_num: Value,key_ptr: Value,tag: Value,float: Value,ptr: Value);
    runtime_fn!(in_array,in_array,Some(Context::float64_type()),array: Value,key_tag: Value,key_num: Value,key_ptr: Value);
    runtime_fn!(to_upper,to_upper,Some(Context::void_ptr_type()),ptr: Value);
    runtime_fn!(to_lower,to_lower,Some(Context::void_ptr_type()),ptr: Value);
    runtime_fn!(rand, rand, Some(Context::float64_type()),);
    runtime_fn!(srand, srand, Some(Context::float64_type()), seed: Value);
    runtime_fn!(length, length, Some(Context::float64_type()), string: Value);
    runtime_fn!(index, index, Some(Context::float64_type()), needle: Value, haystack: Value);
    // runtime_fn!(sub, sub, Some(Context::void_ptr_type()), needle: Value, haystack: Value);

    fn split(&mut self, func: &mut Function, string: Value, array: Value, split_ere_val: Option<Value>) -> Value{
        let data_ptr = self.data_ptr(func);
        if let Some(ere) = split_ere_val {
            func.insn_call_native(
                split_ere as *mut c_void,
                vec![data_ptr, string, array, ere],
                Some(Context::float64_type()),
                Abi::Cdecl,
            )
        } else {
            func.insn_call_native(
                split as *mut c_void,
                vec![data_ptr, string, array],
                Some(Context::float64_type()),
                Abi::Cdecl,
            )
        }
    }

    fn substr(&mut self, func: &mut Function, string: Value, start_idx: Value, max_chars: Option<Value>) -> Value{
        let data_ptr = self.data_ptr(func);
        if let Some(max) = max_chars {
            func.insn_call_native(
                substr_max_chars as *mut c_void,
                vec![data_ptr, string, start_idx, max],
                Some(Context::void_ptr_type()),
                Abi::Cdecl,
            )
        } else {
            func.insn_call_native(
                substr as *mut c_void,
                vec![data_ptr, string, start_idx],
                Some(Context::void_ptr_type()),
                Abi::Cdecl,
            )
        }
    }

    fn free_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType) {
        let data_ptr = self.data_ptr(func);
        match typ {
            ScalarType::String => {
                func.insn_call_native(
                    free_string as *mut c_void,
                    &[data_ptr, value.pointer],
                    None,
                    Abi::Cdecl,
                );
            }
            ScalarType::Float => {}
            ScalarType::Variable => {
                func.insn_call_native(
                    free_if_string as *mut c_void,
                    &[data_ptr, value.tag, value.pointer],
                    None,
                    Abi::Cdecl,
                );
            }
        };
    }
    fn copy_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType) -> ValueT {
        let data_ptr = self.data_ptr(func);
        let ptr = match typ {
            ScalarType::String => func.insn_call_native(
                copy_string as *mut c_void,
                &[data_ptr, value.pointer],
                Some(Context::void_ptr_type()),
                Abi::Cdecl,
            ),
            ScalarType::Float => value.pointer,
            ScalarType::Variable => func.insn_call_native(
                copy_if_string as *mut c_void,
                &[data_ptr, value.tag.clone(), value.pointer],
                Some(Context::void_ptr_type()),
                Abi::Cdecl,
            ),
        };
        ValueT::new(value.tag, value.float, ptr)
    }

    fn pointer_to_name_mapping(&self) -> HashMap<String, String> {
        let mut mapping = HashMap::new();
        insert(&mut mapping, self.runtime_data, "runtime_data");
        insert(&mut mapping, next_line as *mut c_void, "next_line");
        insert(&mut mapping, column as *mut c_void, "column");
        insert(&mut mapping, free_string as *mut c_void, "free_string");
        insert(
            &mut mapping,
            free_if_string as *mut c_void,
            "free_if_string",
        );
        insert(
            &mut mapping,
            string_to_number as *mut c_void,
            "string_to_number",
        );
        insert(&mut mapping, copy_string as *mut c_void, "copy_string");
        insert(
            &mut mapping,
            copy_if_string as *mut c_void,
            "copy_if_string",
        );
        insert(
            &mut mapping,
            number_to_string as *mut c_void,
            "number_to_string",
        );
        insert(&mut mapping, print_string as *mut c_void, "print_string");
        insert(&mut mapping, print_float as *mut c_void, "print_float");
        insert(&mut mapping, concat as *mut c_void, "concat");
        insert(&mut mapping, empty_string as *mut c_void, "empty_string");
        insert(&mut mapping, binop as *mut c_void, "binop");
        insert(&mut mapping, print_error as *mut c_void, "print_error");
        insert(&mut mapping, array_access as *mut c_void, "array_access");
        insert(&mut mapping, array_assign as *mut c_void, "array_assign");
        insert(&mut mapping, in_array as *mut c_void, "in_array");
        insert(&mut mapping, copy_if_string as *mut c_void, "copy_if_string");
        insert(&mut mapping, free_if_string as *mut c_void, "free_if_string");
        insert(&mut mapping, substr as *mut c_void, "substr");
        insert(&mut mapping, split as *mut c_void, "split");
        insert(&mut mapping, index as *mut c_void, "index");
        // insert(&mut mapping, sub as *mut c_void, "sub");
        insert(&mut mapping, concat_array_indices as *mut c_void, "concat_array_indices", );
        insert(&mut mapping, printf as *mut c_void, "printf");
        mapping
    }
}

fn cast_to_runtime_data(data: *mut c_void) -> &'static mut RuntimeData {
    unsafe {
        let data = data as *mut RuntimeData;
        let d = &mut *data;
        if d.canary != CANARY {
            eprintln!("RUNTIME DATA LOADED WRONG. CANARY MISSING");
            std::process::exit(-1);
        }
        d
    }
}
