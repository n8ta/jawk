use gnu_libjit::{Abi, Context};

fn main() {
    let mut context = Context::new();
    context.build_start();

    let float_type = Context::sbyte_type();
    let params = vec![float_type];
    let mut func = context.function(Abi::Cdecl, &float_type, params).unwrap();

    let x = func.arg(0).unwrap();
    let ptr1 = func.alloca(1);
    let ptr2 = func.alloca(8);

    let tag = func.create_sbyte_constant(111);
    let value = func.create_float64_constant(111.0);

    func.insn_store(&ptr1, &tag);
    func.insn_store(&ptr2, &value);

    let res1 = func.insn_load(&ptr1);
    let res2 = func.insn_load(&ptr2);

    let res= func.insn_add(&res1, &res2);


    func.insn_return(&res);
    println!("{}", func.dump().unwrap());
    func.compile();
    context.build_end();

    let result: extern "C" fn(f64) -> u8 = func.to_closure();
    println!("{}", result(0.23))
}