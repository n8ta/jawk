use std::os::raw::{c_long, c_void};
use libc::fchflags;
use gnu_libjit::{Abi, Context, JitType, Label};

fn main() {
    let mut context = Context::new();
    let val_struct = JitType::new_struct(vec![Context::sbyte_type(), Context::float64_type(), Context::int_type()]);
    let val_struct_ptr = val_struct.type_create_pointer();

    context.build_start();
    let field0 = val_struct.field_offset(0);
    let field1 = val_struct.field_offset(1);
    let field2 = val_struct.field_offset(2);

    // inner_func(*Struct globals, int idx) {
    //   globals[idx].field1 = 33.33;
    //   return 33.33;
    // }
    let inner_func = {
        let mut inner_func = context.function(Abi::Cdecl, &Context::float64_type(), vec![val_struct_ptr.clone(), Context::int_type()]).unwrap();
        let struct_ptr = inner_func.arg(0).unwrap();
        let idx = inner_func.arg(1).unwrap();
        let float_const = inner_func.create_float64_constant(33.33);
        let float_const2 = inner_func.create_float64_constant(-123.33);
        let struct_at_idx_ptr = inner_func.insn_load_elem_address(&struct_ptr, &idx, &val_struct);
        inner_func.insn_store_relative(&struct_at_idx_ptr, field1, &float_const);
        inner_func.insn_return(&float_const2);
        inner_func.compile();
        inner_func
    };


    let mut func = context.function(Abi::Cdecl, &Context::float64_type(), vec![val_struct_ptr, Context::int_type()]).unwrap();
    let struct_ptr = func.arg(0).unwrap();
    let idx_arg = func.arg(1).unwrap();

    let ret = func.insn_call(&inner_func, vec![struct_ptr.clone(), idx_arg.clone()]);
    let struct_at_idx_ptr = inner_func.insn_load_elem_address(&struct_ptr, &idx_arg, &val_struct);
    let ret = func.insn_load_relative(&struct_at_idx_ptr, field1, &Context::float64_type());

    func.insn_return(&ret);
    func.compile();

    context.build_end();

    let memory: *mut c_void = unsafe { libc::malloc(1000) };
    unsafe { libc::memset(memory, 0, 1000) };
    let closure: extern "C" fn(*mut c_void, i32) -> f64 = func.to_closure();
    println!("33.33 === {}", closure(memory,1));
}