use gnu_libjit_sys::jit_function_create;
use gnu_libjit::{Abi, Context};

fn main() {
    let mut context = Context::new();
    context.build_start();

    // This function multiplies int by 2
    let int_type = Context::int_type();
    let mut func_mult_by_2 = context.function(Abi::Cdecl, &int_type, vec![int_type]).unwrap();
    let x = func_mult_by_2.arg(0).unwrap();
    let const_1 = func_mult_by_2.create_int_constant(2);
    let temp1 = func_mult_by_2.insn_mult(&x, &const_1);
    func_mult_by_2.insn_return(&temp1);
    func_mult_by_2.compile();

    // This main function has 1 arg, it adds 5 to the arg and calls func_mult_by_2 and returns that value.
    let mut func = context.function(Abi::Cdecl, &int_type, vec![int_type]).unwrap();
    let arg0 = func.arg(0).unwrap();
    let five = func.create_int_constant(5);
    let arg0_plus_5 = func.insn_add(&arg0, &five);
    let res = func.insn_call(&func_mult_by_2, vec![arg0_plus_5]);
    func.insn_return(&res);
    func.compile();

    context.build_end();

    let result: extern "C" fn(i32) -> i32 = func.to_closure();
    println!("(100+5)*2 == {}", result(100))
}
