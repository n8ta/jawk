use std::os::raw::c_void;
use gnu_libjit::{Abi, Context, Function, Label};
use crate::codegen::{STRING_TAG, ValueT};

pub fn build_free_if_string_helper(context: &Context, free_string: *mut c_void, runtime_data: *mut c_void) -> Function {
    let mut function = context.function(Abi::Cdecl,
                                    &Context::int_type(),
                                    vec![Context::int_type(), Context::void_ptr_type()]).expect("to be able to create function");
    let runtime_data = function.create_void_ptr_constant(runtime_data);
    let tag = function.arg(0).unwrap();
    let pointer = function.arg(1).unwrap();
    let str_tag = function.create_sbyte_constant(STRING_TAG);
    let mut done_lbl = Label::new();
    let is_string = function.insn_eq(&str_tag, &tag);
    function.insn_branch_if_not(&is_string, &mut done_lbl);
    function.insn_call_native(free_string, vec![runtime_data, pointer], None, Abi::Cdecl);
    function.insn_label(&mut done_lbl);
    let zero = function.create_int_constant(0);;
    function.insn_return(&zero);
    function.compile();
    function
}

pub fn build_copy_if_str_helper(context: &Context, copy_string: *mut c_void, runtime_data: *mut c_void) -> Function {
    let mut function = context.function(Abi::Cdecl, &Context::void_ptr_type(), vec![Context::int_type(), Context::void_ptr_type()]).expect("to be able to create function");
    let runtime_data = function.create_void_ptr_constant(runtime_data);
    let string_tag = function.create_sbyte_constant(STRING_TAG);
    let tag = function.arg(0).unwrap();
    let ptr = function.arg(1).unwrap();
    let zero_ptr = function.create_void_ptr_constant(123 as *mut c_void); // using a non zero value makes bugs easier to track down on segfault
    let is_string = function.insn_eq(&string_tag, &tag);
    let mut is_not_str_lbl = Label::new();
    function.insn_branch_if_not(&is_string, &mut is_not_str_lbl);
    let new_ptr = function.insn_call_native(copy_string , vec![runtime_data, ptr] , Some(Context::void_ptr_type()), Abi::Cdecl);
    function.insn_return(&new_ptr);

    function.insn_label(&mut is_not_str_lbl);
    function.insn_return( &zero_ptr);

    function.compile();
    function

}