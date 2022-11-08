use gnu_libjit::{Abi, Context, Label};

fn main() {

    /*

    Show you CAN access a const created in one function from another. This isn't a super important flow
    I just wanted to make sure it was allowed before using it.

     inner_func() {
         return const from outer 1.1
     }

     func(arg: f64) {
        let const_float = 1.1;
        let res  = inner_func()
        return res;
     }
     */


    let mut context = Context::new();
    context.build_start();
    let mut func = context.function(Abi::Cdecl, &Context::float64_type(), vec![Context::float64_type()]).unwrap();
    let fconst = func.create_float64_constant(1.1);

    let inner_func = {
        let mut inner_func = context.function(Abi::Cdecl, &Context::float64_type(), vec![]).unwrap();
        inner_func.insn_return(&fconst);
        inner_func.compile();
        inner_func
    };

    let ret = func.insn_call(&inner_func, vec![]);

    func.insn_return(&ret);
    func.compile();

    context.build_end();
    let closure: extern "C" fn(f64) -> f64 = func.to_closure();
    println!("=> {} should be 1.1", closure(123.123));
}