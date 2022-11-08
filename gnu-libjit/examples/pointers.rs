use gnu_libjit::{Abi, Context, Label};

fn main() {

    /*
     inner_func(arg: *mut f64) {
         *arg = 1.1;
         return 1.1
     }

     func(arg: f64) {
        let stack_var: *f64 = 2.2;
        inner_func(&stack_var);
        return *stack_var; // 1.1
     }
     */


    let mut context = Context::new();
    context.build_start();
    let mut func = context.function(Abi::Cdecl, &Context::float64_type(), vec![Context::float64_type()]).unwrap();

    let inner_func = {
        let mut inner_func = context.function(Abi::Cdecl, &Context::float64_type(), vec![Context::float64_type().type_create_pointer()]).unwrap();
        let arg0 = inner_func.arg(0).unwrap();
        let const_1 = inner_func.create_float64_constant(1.1);
        inner_func.insn_store_relative(&arg0, 0, &const_1);
        inner_func.insn_return(&const_1);
        inner_func.compile();
        inner_func
    };

    let const_2 = func.create_float64_constant(2.2);
    let mut float_val = func.create_value_float64();
    let float_addr = func.address_of(&mut float_val);
    func.insn_store_relative(&float_addr, 0,&const_2);

    func.insn_call(&inner_func, vec![float_addr.clone()]);

    let ret = func.insn_load_relative(&float_addr, 0, &Context::float64_type());
    func.insn_return(&ret);
    func.compile();

    context.build_end();
    let closure: extern "C" fn(f64) -> f64 = func.to_closure();
    println!("123.123 => {}", closure(123.123));
    assert_eq!(closure(123.123), 1.1);

}