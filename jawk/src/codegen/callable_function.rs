use std::ops::Deref;
use gnu_libjit::Function;
use crate::parser::{Arg};

pub struct CallableFunction {
    pub args: Vec<Arg>,
    pub function: Function,
}

impl CallableFunction {
    pub fn new(function: Function, args: Vec<Arg>) -> Self {
        Self { args, function }
    }
}

impl Deref for CallableFunction {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.function
    }
}

