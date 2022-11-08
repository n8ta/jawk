use gnu_libjit_sys::jit_function_create;
use gnu_libjit::{Abi, Context};

fn main() {
    let mut context = Context::new();
    context.build_start();

    // This function takes an int_ptr and 2x's it
    let int_ptr = Context::int_type().type_create_pointer();
    let int_type = Context::int_type();
    let mut func_mult_by_2 = context.function(Abi::Cdecl, &int_type, vec![int_ptr]).unwrap();
    let zero = func_mult_by_2.create_int_constant(0);
    let two = func_mult_by_2.create_int_constant(2);

    let int_ptr_arg = func_mult_by_2.arg(0).unwrap();
    let loaded = func_mult_by_2.insn_load_relative(&int_ptr_arg, 0, &Context::int_type());
    let loaded2x = func_mult_by_2.insn_mult(&loaded, &two);
    func_mult_by_2.insn_store_relative(&int_ptr_arg, 0, &loaded2x);
    func_mult_by_2.insn_return(&zero);
    func_mult_by_2.compile();

    // This main function inits an integer to agr0 and then passes a ptr to that value to the 2x func above
    // and returns the final value of the int
    let mut func = context.function(Abi::Cdecl, &int_type, vec![int_type]).unwrap();
    let arg = func.arg(0).unwrap();
    let mut int_val = func.create_value_int();
    func.insn_store(&int_val, &arg);

    let int_ptr = func.address_of(&mut int_val);
    let res = func.insn_call(&func_mult_by_2, vec![int_ptr]);

    func.insn_return(&int_val);
    func.compile();

    context.build_end();

    let result: extern "C" fn(i32) -> i32 = func.to_closure();
    println!("4*2 == {}", result(4))
}
