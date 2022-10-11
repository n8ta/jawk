use std::os::raw::c_long;
use gnu_libjit::{Abi, Context, JitType, Label};

fn main() {
    let mut context = Context::new();
    let val_struct = JitType::new_struct(vec![Context::sbyte_type(), Context::float64_type(), Context::int_type()]);
    context.build_start();
    let field0 = val_struct.field_offset(0);
    let field1 = val_struct.field_offset(1);
    let field2 = val_struct.field_offset(2);


    let inner_func = {
        let mut inner_func = context.function(Abi::Cdecl, &val_struct, vec![]).unwrap();
        let mut val = inner_func.create_value(&val_struct);
        let sbyte_const = inner_func.create_sbyte_constant(1);
        let float_const = inner_func.create_float64_constant(22.22);
        let int_const = inner_func.create_int_constant(33);
        let addr = inner_func.address_of(&mut val);
        inner_func.insn_store_relative(&addr, field0, &sbyte_const);
        inner_func.insn_store_relative(&addr, field1, &float_const);
        inner_func.insn_store_relative(&addr, field2, &int_const);
        inner_func.insn_return(&val);
        inner_func.compile();
        inner_func
    };

    let mut func = context.function(Abi::Cdecl, &Context::float64_type(), vec![]).unwrap();
    let mut ret = func.insn_call(&inner_func, vec![]);
    let ret_ptr = func.address_of(&mut ret);
    let ret = func.insn_load_relative(&ret_ptr, field1, &Context::float64_type());
    func.insn_return(&ret);
    func.compile();

    context.build_end();
    let closure: extern "C" fn() -> f64 = func.to_closure();
    println!("22.22 === {}", closure());
}