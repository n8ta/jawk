mod arrays;
mod call_log;
mod debug_runtime;
mod release_runtime;
mod array_split;
mod value;
mod string_converter;
mod util;

use crate::codegen::ValueT;
use crate::lexer::BinOp;
use crate::parser::ScalarType;
pub use debug_runtime::DebugRuntime;
use gnu_libjit::{Context, Function, Value};
use hashbrown::HashMap;
pub use release_runtime::ReleaseRuntime;
use crate::awk_str::AwkStr;

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorCode {
    Error1,
    Error2,
}

#[macro_export]
macro_rules! runtime_fn {
    ($fn_name:ident, $native_fn_ptr: expr, $jit_ret_type:expr, $($v:ident: $t:ty),*) => {
        fn $fn_name(&mut self, func: &mut Function, $($v: $t),*) -> Value {
            let data_ptr = self.data_ptr(func);
            func.insn_call_native(
                $native_fn_ptr as *mut c_void,
                vec![data_ptr, $($v),*],
                $jit_ret_type,
                Abi::Cdecl,
            )
        }
    }
}

#[macro_export]
macro_rules! runtime_fn_no_ret {
    ($fn_name:ident, $native_fn_ptr: expr, $jit_ret_type:expr, $($v:ident: $t:ty),*) => {
        fn $fn_name(&mut self, func: &mut Function, $($v: $t),*) {
            let data_ptr = self.data_ptr(func);
            func.insn_call_native(
                $native_fn_ptr as *mut c_void,
                vec![data_ptr, $($v),*],
                $jit_ret_type,
                Abi::Cdecl,
            );
        }
    }
}

pub trait Runtime {
    fn new(context: &Context, files: Vec<String>) -> Self
    where
        Self: Sized;
    fn call_next_line(&mut self, func: &mut Function) -> Value;
    fn to_lower(&mut self, func: &mut Function, ptr: Value) -> Value;
    fn to_upper(&mut self, func: &mut Function, ptr: Value) -> Value;
    fn rand(&mut self, func: &mut Function) -> Value;
    fn srand(&mut self, func: &mut Function, flt: Value) -> Value;
    fn length(&mut self, func: &mut Function, ptr: Value) -> Value;
    fn index(&mut self, func: &mut Function, needle: Value, haystack: Value) -> Value;
    // fn sub(&mut self, func: &mut Function, ere: Value, replacement: Value, input: Value) -> Value;
    fn column(&mut self, func: &mut Function, tag: Value, float: Value, ptr: Value) -> Value;
    fn free_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType);
    fn split(&mut self, func: &mut Function, string: Value, array: Value, ere_string: Option<Value>) -> Value;
    fn substr(&mut self, func: &mut Function, string: Value, start_idx: Value, max_chars: Option<Value>) -> Value;
    fn string_to_number(&mut self, func: &mut Function, ptr: Value) -> Value;
    fn copy_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType) -> ValueT;
    fn number_to_string(&mut self, func: &mut Function, number: Value) -> Value;
    fn print_string(&mut self, func: &mut Function, ptr: Value);
    fn print_float(&mut self, func: &mut Function, number: Value);
    fn concat(&mut self, func: &mut Function, ptr1: Value, ptr2: Value) -> Value;
    fn empty_string(&mut self, func: &mut Function) -> Value;
    fn init_empty_string(&mut self) -> *const AwkStr;
    fn binop(&mut self, func: &mut Function, left: ValueT, right: ValueT, binop: BinOp) -> Value;
    fn print_error(&mut self, func: &mut Function, code: ErrorCode);
    fn allocate_arrays(&mut self, count: usize);
    fn array_access(
        &mut self,
        func: &mut Function,
        array_id: Value,
        key_tag: Value,
        key_num: Value,
        key_ptr: Value,
        out_tag_ptr: Value,
        out_float_ptr: Value,
        out_ptr_ptr: Value,
    );
    fn array_assign(
        &mut self,
        func: &mut Function,
        array_id: Value,
        key_tag: Value,
        key_num: Value,
        key_ptr: Value,
        tag: Value,
        float: Value,
        ptr: Value,
    );
    fn in_array(
        &mut self,
        func: &mut Function,
        array_id: Value,
        key_tag: Value,
        key_num: Value,
        key_ptr: Value,
    ) -> Value;
    fn concat_array_indices(&mut self, func: &mut Function, lhs: Value, rhs: Value) -> Value;
    fn printf(&mut self, func: &mut Function, fstring: Value, nargs: Value, args: Value);
    fn pointer_to_name_mapping(&self) -> HashMap<String, String>;
}
