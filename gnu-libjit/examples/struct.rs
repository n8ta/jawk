use std::os::raw::c_long;
use gnu_libjit::{Abi, Context, JitType, Label};

fn main() {
    let mut context = Context::new();
    let val_struct = JitType::new_struct(vec![Context::sbyte_type(), Context::float64_type(), Context::int_type()]);
    context.build_start();
    let field0 = val_struct.field_offset(0);
    let field1 = val_struct.field_offset(1);
    let field2 = val_struct.field_offset(2);


    let mut inner_func = context.function(Abi::Cdecl, &Context::float64_type(), vec![val_struct.type_create_pointer()]).unwrap();
    let arg0 = inner_func.arg(0).unwrap();
    let float_const = inner_func.create_float64_constant(3.3);
    inner_func.insn_store_relative(&arg0, field1, &float_const);
    inner_func.insn_return(&float_const);
    println!("{}", inner_func.dump().unwrap());
    inner_func.compile();


    let mut func = context.function(Abi::Cdecl, &Context::float64_type(), vec![Context::sbyte_type(), Context::float64_type(), Context::int_type()]).unwrap();
    let mut val = func.create_value(&val_struct);


    let sbyte_const = func.create_sbyte_constant(1);
    let float_const = func.create_float64_constant(2.2);
    let int_const = func.create_int_constant(33);
    func.insn_store_relative(&val, field0, &sbyte_const);
    func.insn_store_relative(&val, field1, &float_const);
    func.insn_store_relative(&val, field2, &int_const);

    let addr_struct = func.address_of(&mut val);
    func.insn_call(&inner_func, vec![addr_struct]);

    // let sbyte = func.insn_load_relative(&val, field0, &Context::sbyte_type());
    let float = func.insn_load_relative(&val, field1, &Context::float64_type());
    // let int = func.insn_load_relative(&val, field2, &Context::int_type());

    func.insn_return(&float);
    println!("{}", func.dump().unwrap());
    func.compile();

    context.build_end();
    let closure: extern "C" fn(i8, f64, i32) -> f64 = func.to_closure();
    println!("3.3 === {}", closure(1,1.1,11));
}