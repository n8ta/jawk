use gnu_libjit::{Abi, Context};

fn main() {
    let mut context = Context::new();
    context.build_start();


    let int_type = Context::int_type();
    let params = vec![int_type, int_type, int_type];
    let mut func = context.function(Abi::Cdecl, &int_type, params).unwrap();

    let x = func.arg(0).unwrap();
    let y = func.arg(1).unwrap();
    let z = func.arg(2).unwrap();
    let temp1 = func.insn_mult(&x, &y);
    let temp2 = func.insn_add(&temp1, &z);
    func.insn_return(&temp2);
    func.compile();
    context.build_end();

    let result: extern "C" fn(i32,i32,i32) -> i32 = func.to_closure();
    println!("3*5+2 = {}", result(3,5,2))
}
