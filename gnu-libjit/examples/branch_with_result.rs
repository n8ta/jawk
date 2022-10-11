use std::ptr::eq;
use gnu_libjit::{Abi, Context, Label};
fn main() {
    let mut context = Context::new();
    context.build_start();


    let i32_type = Context::int_type();
    let mut func = context.function(Abi::Cdecl, &i32_type, vec![i32_type, i32_type]).unwrap();


    let result = func.alloca(4);

    let a = func.arg(0).unwrap();
    let b = func.arg(1).unwrap();
    let a_eq_b = func.insn_eq(&a, &b);
    let mut done_label = Label::new();
    let mut eq_label = Label::new();

    func.insn_branch_if(&a_eq_b, &mut eq_label);
    let a_minus_b = func.insn_sub(&a, &b);
    func.insn_store(&result, &a_minus_b);
    func.insn_branch(&mut done_label);
    func.insn_label(&mut eq_label);
    let a_plus_b = func.insn_add(&a, &b);
    func.insn_store(&result, &a_plus_b);
    func.insn_label(&mut done_label);
    let ret = func.insn_load(&result);
    let hundred = func.create_int_constant(100);
    let ret = func.insn_add(&ret, &hundred);
    func.insn_return(&ret);

    println!("{}", func.dump().unwrap());

    func.compile();
    println!("{}", func.dump().unwrap());
    context.build_end();


    let result: extern "C" fn(i32, i32) -> i32 = func.to_closure();
    println!("{} == 102", result(1,1));
    println!("{} == 101", result(3,2));
}

/*
fn(a,b) {
    x ;
    if a == b {
        x = a + b;
    } else {
        x = a - b;
    }
    return x + 100;
 */
