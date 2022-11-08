use gnu_libjit::{Abi, Context};

fn main() {
    let mut context = Context::new();
    context.build_start();
    let mut func = context.function(Abi::Cdecl, &Context::float64_type(), vec![]).unwrap();
    let zero = func.create_float64_constant(0.0);
    func.insn_return(&zero);
    func.compile();
    context.build_end();
    let function: fn() -> f64  = func.to_closure();
    println!("{}", function());
}
