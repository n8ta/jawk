use std::cell::Ref;
use std::ops::Deref;
use std::rc::Rc;
use gnu_libjit::{Abi, Context, Function};
use crate::parser::{Arg, ArgT};
use crate::typing::{ITypedFunction, TypedUserFunction};

pub struct CallableFunction {
    function: Function,
    typed_function: Rc<TypedUserFunction>,
}

impl CallableFunction {
    pub fn main(function: Function, typed_function: Rc<TypedUserFunction>) -> Self {
        Self { function, typed_function }
    }
    pub fn new(context: &Context, typed_function: Rc<TypedUserFunction>) -> Self {
        let args = typed_function.args();
        let mut params = Vec::with_capacity(args.len() * 3); // May be shorter if some args are arrays
        for arg in args.iter() {
            match arg.typ {
                ArgT::Scalar => {
                    params.push(Context::int_type());
                    params.push(Context::float64_type());
                    params.push(Context::void_ptr_type());
                }
                ArgT::Array => {
                    params.push(Context::int_type());
                }
                ArgT::Unknown => {}
            }
        }
        drop(args);
        let function = context.function(Abi::Fastcall, &Context::int_type(), params).unwrap();
        Self {
            function,
            typed_function,
        }
    }
    pub fn args(&self) -> Ref<'_, Vec<Arg>> {
        self.typed_function.args()
    }
    pub fn jit_function(&self) -> &Function {
        &self.function
    }
}

impl Deref for CallableFunction {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.function
    }
}


