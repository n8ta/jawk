use std::num::TryFromIntError;
use std::os::raw::c_uint;
use gnu_libjit_sys::{jit_context_build_end, jit_context_build_start, jit_type_ulong, jit_context_create, jit_context_t, jit_type_long, jit_function_create, jit_type_create_signature, jit_type_float32, jit_type_float64, jit_type_int, jit_type_t, jit_abi_t, jit_type_sbyte, jit_type_ubyte, jit_type_void_ptr};
use crate::{Abi, Function, JitType};

pub struct Context {
    context: jit_context_t,
}

#[derive(Clone, Debug)]
pub enum Exception {
    TooManyParams(TryFromIntError),
    ParamIndexToLarge(TryFromIntError),
    ArgIndexTooLarge(String),
}

impl Context {
    pub fn new() -> Context {
        unsafe {
            Context { context: jit_context_create() }
        }
    }

    pub fn build_start(&self) {
        unsafe {
            jit_context_build_start(self.context);
        }
    }
    pub fn build_end(&self) {
        unsafe {
            jit_context_build_end(self.context);
        }
    }

    // Adds a new function to the context
    // TODO: This could fail on systems with non-32bit unsigned ints
    /// let mut context = Context::new();
    /// let params = vec![JitType::Int]
    /// let function = context.function(Abi::Cdecl, JitType::Int, params.as_slice());
    pub fn function(&mut self, abi: Abi, return_type: &JitType, params: Vec<JitType>) -> Result<Function, Exception> {
        let mut params_libjit: Vec<jit_type_t> = params.iter().map(|p| p.inner).collect();

        unsafe {
            let signature = jit_type_create_signature(
                abi as jit_abi_t,
                return_type.inner,
                params_libjit.as_mut_ptr(),
                params.len() as c_uint,
                1,
            );
            let function = Function::new(jit_function_create(self.context, signature),  signature, params);
            Ok(function)
        }
    }

    pub fn int_type() -> JitType {
        unsafe { JitType::new(jit_type_int) }
    }
    pub fn long_type() -> JitType {
        unsafe { JitType::new(jit_type_long) }
    }
    pub fn ulong_type() -> JitType {
        unsafe { JitType::new(jit_type_ulong) }
    }
    pub fn float32_type() -> JitType {
        unsafe { JitType::new(jit_type_float32) }
    }
    pub fn float64_type() -> JitType { unsafe { JitType::new(jit_type_float64) } }
    pub fn sbyte_type() -> JitType { unsafe { JitType::new(jit_type_sbyte) } }
    pub fn ubyte_type() -> JitType { unsafe { JitType::new(jit_type_ubyte) } }
    pub fn void_ptr_type() -> JitType { unsafe { JitType::new(jit_type_void_ptr) } }
}