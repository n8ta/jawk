mod native;

use crate::codegen::{ValueT};
use crate::columns::Columns;
use crate::lexer::BinOp;
use crate::parser::ScalarType;
use crate::runtime::arrays::Arrays;
use crate::runtime::{ErrorCode, Runtime};
use gnu_libjit::{Abi, Context, Function, Value};
use hashbrown::HashMap;
use lru_cache::LruCache;
use mawk_regex::Regex;
use std::ffi::c_void;
use std::io::{BufWriter, StdoutLock, Write};
use std::rc::Rc;
use native::{column, concat, array_assign, copy_string, copy_if_string, binop, print_float, print_string, print_error, printf, split, next_line, string_to_number, number_to_string, concat_array_indices, array_access, in_array, to_upper, to_lower, rand, srand, length, split_ere, free_string, free_if_string, empty_string};


use crate::runtime::float_parser::{FloatParser};
use crate::{runtime_fn, runtime_fn_no_ret};

pub struct ReleaseRuntime {
    runtime_data: *mut RuntimeData,
}

impl Drop for ReleaseRuntime {
    fn drop(&mut self) {
        unsafe {
            (*self.runtime_data)
                .stdout
                .flush()
                .expect("could not flush stdout");
        }
    }
}

// Pointer to this is passed in with every call. The reason we require it for every call instead of making it
// a rust global is so we can easily run tests fully independently of each other.
pub struct RuntimeData {
    srand_seed: f64,
    columns: Columns,
    stdout: BufWriter<StdoutLock<'static>>,
    regex_cache: LruCache<String, Regex>,
    arrays: Arrays,
    float_parser: FloatParser,
    fast_alloc: Option<Rc<String>>,
}

impl RuntimeData {
    pub fn new(files: Vec<String>) -> RuntimeData {
        unsafe { libc::srand(09171998) }
        RuntimeData {
            srand_seed: 09171998.0,
            columns: Columns::new(files),
            stdout: BufWriter::new(std::io::stdout().lock()),
            regex_cache: LruCache::new(8),
            arrays: Arrays::new(),
            float_parser: FloatParser::new(),
            fast_alloc: None,
        }
    }
}

impl ReleaseRuntime {
    fn data_ptr(&mut self, func: &mut Function) -> Value {
        func.create_void_ptr_constant(self.runtime_data as *mut c_void)
    }
}

impl Runtime for ReleaseRuntime {
    fn new(_context: &Context, files: Vec<String>) -> ReleaseRuntime {
        let data = Box::new(RuntimeData::new(files));
        let ptr = Box::leak(data);
        let ptr = ptr as *mut RuntimeData;
        ReleaseRuntime { runtime_data: ptr }
    }

    fn allocate_arrays(&mut self, count: usize) {
        unsafe { (*self.runtime_data).arrays.allocate(count) }
    }

    fn init_empty_string(&mut self) -> *const String {
        empty_string(self.runtime_data as *mut c_void)
    }

    fn binop(&mut self, func: &mut Function, ptr1: Value, ptr2: Value, binop_val: BinOp) -> Value {
        let binop_val = func.create_sbyte_constant(binop_val as i8);
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            binop as *mut c_void,
            vec![data_ptr, ptr1, ptr2, binop_val],
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
        HashMap::new()
    }

    fn split(&mut self, func: &mut Function, string: Value, array: Value, split_ere_val: Option<Value>) {
        let data_ptr = self.data_ptr(func);
        if let Some(ere) = split_ere_val {
            func.insn_call_native(
                split_ere as *mut c_void,
                vec![data_ptr, string, array, ere],
                None,
                Abi::Cdecl,
            );
        } else {
            func.insn_call_native(
                split as *mut c_void,
                vec![data_ptr, string, array],
                None,
                Abi::Cdecl,
            );
        }
    }

}

fn cast_to_runtime_data(data: *mut c_void) -> &'static mut RuntimeData {
    unsafe {
        let data = data as *mut RuntimeData;
        &mut *data
    }
}
