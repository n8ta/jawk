use std::time::Instant;
use gnu_libjit::{Abi, Context, Label};

fn main() {
    let mut context = Context::new();
    context.build_start();

    // INNER FUNC
    // Tried messing with this ABI but it really made no difference *shrug*
    let mut inner_func = context.function(Abi::VarArg   , &Context::float64_type(), vec![Context::float64_type(), Context::float64_type()]).unwrap();
    let arg = inner_func.arg(0).unwrap();
    let arg2 = inner_func.arg(1).unwrap();
    let arg_plus_arg = inner_func.insn_add(&arg, &arg2);
    inner_func.insn_return(&arg_plus_arg);
    inner_func.compile();

    // DONE


    let mut func = context.function(Abi::Cdecl, &Context::float64_type(), vec![Context::float64_type()]).unwrap();
    let arg = func.arg(0).unwrap();
    let zero = func.create_int_constant(0);
    let one = func.create_int_constant(1);
    let hundred_k = func.create_int_constant(50_000_000);
    let i = func.create_value_int();
    let f = func.create_value_float64();
    let zero_f = func.create_float64_constant(0.0);
    func.insn_store(&i, &zero);
    func.insn_store(&f, &zero_f);
    let mut done_lbl = Label::new();
    let mut test_lbl = Label::new();
    func.insn_label(&mut test_lbl);

    let is_greater = func.insn_gt(&i, &hundred_k);
    func.insn_branch_if(&is_greater, &mut done_lbl);

    let load_i = func.insn_load(&i);
    let i_plus_1 = func.insn_add(&load_i, &one);
    func.insn_store(&i, &i_plus_1);

    let res = func.insn_call(&inner_func, vec![f.clone(), arg.clone()]);
    func.insn_store(&f, &res);
    func.insn_branch(&mut test_lbl);

    func.insn_label(&mut done_lbl);

    func.insn_return(&f);
    println!("func: {}", func.dump().unwrap());
    func.compile();
    println!("func: {}", func.dump().unwrap());


    context.build_end();



    let func: extern "C" fn(f64) -> f64 =  func.to_closure();
    let now = Instant::now();
    println!("{}", func(1.1));
    println!("Elapsed: {}ms", now.elapsed().as_millis());
}