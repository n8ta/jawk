use std::ops::Deref;
use gnu_libjit::{Abi, Context, Function};
use crate::parser::{Arg, ArgT};

pub struct CallableFunction {
    pub args: Vec<Arg>,
    pub function: Function,
}

impl CallableFunction {
    pub fn new(context: &mut Context, args: &Vec<Arg>) -> CallableFunction {
        let mut params = Vec::with_capacity(args.len()*3); // May be shorter if some args are arrays
        for arg in args.iter() {
            match arg.typ {
                None => {}
                Some(arg_typ) => {
                    match arg_typ {
                        ArgT::Scalar => {
                            params.push(Context::int_type());
                            params.push(Context::float64_type());
                            params.push(Context::void_ptr_type());
                        }
                        ArgT::Array => {
                            params.push(Context::int_type());
                        }
                    }
                }
            }
        }
        let function = context.function(Abi::Cdecl, &Context::int_type(), params).unwrap();
        CallableFunction { function, args: args.clone() }
    }
}

impl Deref for CallableFunction {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.function
    }
}


