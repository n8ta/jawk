
use gnu_libjit::{Abi, Context, Label};
fn main() {
    let mut context = Context::new();
    context.build_start();


    let float_type = Context::float64_type();
    let mut func = context.function(Abi::Cdecl, &float_type, vec![float_type]).unwrap();

    // Return 1 if arg0 == 4
    // else return 0
    let is_four_result = func.create_float64_constant(1.0);
    let not_four_result = func.create_float64_constant(0.0);

    let x = func.arg(0).unwrap();
    let four = func.create_float64_constant(4.0);
    let mut label = Label::new();
    let eq_to_4 = func.insn_eq(&x, &four);
    func.insn_branch_if(&eq_to_4, &mut label);
    func.insn_return(&not_four_result);
    func.insn_label(&mut label);
    func.insn_return(&is_four_result);
    func.compile();
    context.build_end();
    let result: extern "C" fn(f64) -> f64 = func.to_closure();
    println!("{}", result(4.0));
}
