mod call_log;
mod live;
mod testing;
mod arrays;
mod helpers;

use std::os::raw::c_void;
use crate::lexer::BinOp;
use gnu_libjit::{Context, Function, Value};
use hashbrown::HashMap;
pub use live::LiveRuntime;
pub use testing::TestRuntime;
use crate::codegen::ValueT;
use crate::parser::ScalarType;

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum ErrorCode {
    Error1,
    Error2,
}

pub trait Runtime {
    fn new(context: &Context, files: Vec<String>) -> Self;
    fn call_next_line(&mut self, func: &mut Function) -> Value;
    fn column(&mut self, func: &mut Function, tag: Value, float: Value, ptr: Value) -> Value;
    fn free_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType);
    fn free_string(&mut self, func: &mut Function, ptr: Value) -> Value;
    fn string_to_number(&mut self, func: &mut Function, ptr: Value) -> Value;
    fn copy_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType) -> ValueT;
    fn copy_string(&mut self, func: &mut Function, ptr: Value) -> Value;
    fn number_to_string(&mut self, func: &mut Function, number: Value) -> Value;
    fn print_string(&mut self, func: &mut Function, ptr: Value);
    fn print_float(&mut self, func: &mut Function, number: Value);
    fn concat(&mut self, func: &mut Function, ptr1: Value, ptr2: Value) -> Value;
    fn empty_string(&mut self, func: &mut Function) -> Value;
    fn init_empty_string(&mut self) -> *const String;
    fn binop(&mut self, func: &mut Function, ptr1: Value, ptr2: Value, binop: BinOp) -> Value;
    fn print_error(&mut self, func: &mut Function, code: ErrorCode);
    fn allocate_arrays(&mut self, count: usize);
    fn array_access(&mut self, func: &mut Function, array_id: Value,
                    key_tag: Value, key_num: Value, key_ptr: Value,
                    out_tag_ptr: Value, out_float_ptr: Value, out_ptr_ptr: Value);
    fn array_assign(&mut self, func: &mut Function, array_id: Value,
                    key_tag: Value, key_num: Value, key_ptr: Value,
                    tag: Value, float: Value, ptr: Value);
    fn in_array(&mut self, func: &mut Function, array_id: Value, key_tag: Value, key_num: Value, key_ptr: Value) -> Value;
    fn concat_array_indices(&mut self, func: &mut Function, lhs: Value, rhs: Value) -> Value;
    fn printf(&mut self, func: &mut Function, fstring: Value, nargs: Value, args: Value);
    fn pointer_to_name_mapping(&self) -> HashMap<String, String>;
}