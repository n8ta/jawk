use std::ffi::CString;
use std::os::raw::c_uint;
use gnu_libjit_sys::{jit_function_compile, jit_insn_pow, jit_insn_acos, jit_insn_asin, jit_insn_atan, jit_insn_cos, jit_insn_cosh, jit_insn_log, jit_insn_log10, jit_insn_sin, jit_insn_sinh, jit_insn_sqrt, jit_insn_tan, jit_insn_tanh, jit_value_create_float64_constant, jit_insn_not, jit_insn_ge, jit_insn_le, jit_insn_gt, jit_insn_lt, jit_insn_ne, jit_insn_and, jit_insn_or, jit_insn_xor, jit_function_t, jit_value_create_long_constant, jit_value_create_float32_constant, jit_value_create_nint_constant, jit_value_create, jit_type_float32, jit_insn_eq, jit_type_nint, jit_type_int, jit_type_uint, jit_type_ushort, jit_type_short, jit_insn_add, jit_insn_div, jit_insn_sub, jit_insn_call_native, jit_insn_mul, jit_insn_return, jit_type_create_signature, jit_type_void, jit_value_get_param, jit_dump_function, jit_abi_t, jit_function_to_closure, jit_insn_branch_if, jit_label_t, jit_insn_label, jit_insn_branch_if_not, jit_type_long, jit_type_ulong, jit_type_sbyte, jit_type_float64, jit_type_ubyte, jit_type_void_ptr, jit_insn_alloca, jit_insn_load, jit_insn_store, jit_insn_branch, jit_insn_load_relative, jit_insn_call, jit_value_t, jit_type_t, jit_insn_rem, jit_insn_exp, jit_float32_rint, jit_insn_ceil, jit_insn_floor, jit_insn_rint, jit_insn_round, jit_insn_trunc, jit_insn_load_elem_address, jit_insn_store_relative, jit_insn_address_of};
use libc::{c_char, c_void};
use crate::context::Exception;
use crate::{Abi, JitType};
use crate::label::Label;
use crate::util::dump;
use crate::value::Value;

macro_rules! op {
    ($fn_name:ident, $jit_op:ident) => {
        pub fn $fn_name(&mut self, left: &Value, right: &Value) -> Value {
            unsafe {
                let v = $jit_op(self.function, left.value, right.value);
                Value::new(v)
            }
        }
    }
}

macro_rules! ret_op {
    ($fn_name:ident, $jit_op:ident) => {
        pub fn $fn_name(&mut self, value:& Value) {
            unsafe {
                $jit_op(self.function, value.value);
            }
        }
    }
}

macro_rules! unary_op {
    ($fn_name:ident, $jit_op:ident) => {
        pub fn $fn_name(&mut self, value: &Value) -> Value {
            unsafe {
                let v =$jit_op(self.function, value.value);
                Value::new(v)
            }
        }
    }
}

#[derive(Clone)]
pub struct Function {
    params: Vec<JitType>,
    function: jit_function_t,
    signature: jit_type_t,
}

macro_rules! constant_fn {
    ($fn_name:ident, $const_ty:ty, $typ: expr, $creator:expr) => {
        pub fn $fn_name(&mut self, constant_value: $const_ty) -> Value {
            let val = unsafe {
                $creator(self.function, $typ, constant_value)
            };
            Value::new(val)

        }
    }
}

macro_rules! constant_fn_long {
    ($fn_name:ident, $const_ty:ty, $typ: expr, $creator:expr) => {
        pub fn $fn_name(&mut self, constant_value: $const_ty) -> Value {
            let val = unsafe {
                $creator(self.function, $typ, constant_value as ::std::os::raw::c_long)
            };
            Value::new(val)

        }
    }
}


impl Function {
    // Use Context::new().function  to create a new function. This method is private.
    pub(crate) fn new(function: jit_function_t, signature: jit_type_t, params: Vec<JitType>) -> Function {
        Function { function, params, signature }
    }

    pub fn compile(&self) {
        unsafe {
            if jit_function_compile(self.function) == 0 {
                panic!("Failed to compile function");
            }
        }
    }

    pub fn address_of(&mut self, value: &mut Value) -> Value {
        if !value.addressable {
            value.set_addressable();
        }
        let ret = unsafe {
            jit_insn_address_of(self.function, value.value)
        };
        Value::new(ret)
    }

    pub fn alloca(&self, size: ::std::os::raw::c_long) -> Value {
        unsafe {
            let bytes = jit_value_create_nint_constant(self.function, jit_type_ubyte, size);
            Value::new(jit_insn_alloca(self.function, bytes))
        }
    }

    pub fn dump(&self) -> Result<String, std::fmt::Error> {
        dump(|fd| unsafe {
            jit_dump_function(std::mem::transmute(fd), self.function, "no-name-func".as_ptr() as *const ::std::os::raw::c_char);
        })
    }

    // T must be a extern "C" fn() pointer to avoid disaster.
    // Also don't mess up the arg/return types lest you invite chaos.
    pub fn to_closure<T>(&self) -> T {
        unsafe {
            let void_ptr = jit_function_to_closure(self.function);
            std::mem::transmute_copy::<*mut c_void, T>(&void_ptr)
        }
    }

    // Call a native rust function
    pub fn insn_call_native(&self,
                            native_func: *mut ::std::os::raw::c_void,
                            params: Vec<Value>,
                            return_type: Option<JitType>,
                            abi: Abi) -> Value {
        let c_str = CString::new("native-func").unwrap();
        let c_str_ptr = c_str.as_ptr();
        let mut sig_args = vec![];
        let mut args = vec![];
        for param in params.iter() {
            sig_args.push(param.value_type().inner);
            args.push(param.value);
        }
        unsafe {
            let signature = jit_type_create_signature(
                abi as jit_abi_t,
                if let Some(jtype) = return_type { jtype.inner } else { jit_type_void },
                sig_args.as_mut_ptr(),
                params.len() as c_uint,
                1,
            );
            Value::new(jit_insn_call_native(self.function,
                                            c_str_ptr,
                                            native_func,
                                            signature,
                                            args.as_mut_ptr(),
                                            params.len() as c_uint,
                                            0,
            ))
        }
    }

    // Get the value of the idx'th arg to the function
    pub fn arg(&self, idx: i32) -> Result<Value, Exception> {
        let _arg_type = match self.params.get(idx as usize) {
            Some(arg_type) => arg_type,
            None => {
                return Err(Exception::ArgIndexTooLarge(format!("Function has {} args but you requested index {}", self.params.len(), idx)));
            }
        };
        let value = unsafe {
            jit_value_get_param(self.function, idx as c_uint)
        };
        Ok(Value::new(value))
    }

    op!(insn_mult, jit_insn_mul);
    op!(insn_add, jit_insn_add);
    op!(insn_pow, jit_insn_pow);
    op!(insn_rem, jit_insn_rem);
    op!(insn_div, jit_insn_div);
    op!(insn_sub, jit_insn_sub);
    op!(insn_eq, jit_insn_eq);
    op!(insn_and, jit_insn_and);
    op!(insn_or, jit_insn_or);
    op!(insn_xor, jit_insn_xor);
    op!(insn_le, jit_insn_le);
    op!(insn_lt, jit_insn_lt);
    op!(insn_ge, jit_insn_ge);
    op!(insn_gt, jit_insn_gt);
    op!(insn_ne, jit_insn_ne);

    unary_op!(insn_acos, jit_insn_acos);
    unary_op!(insn_asin, jit_insn_asin);
    unary_op!(insn_atan, jit_insn_atan);
    unary_op!(insn_cos, jit_insn_cos);
    unary_op!(insn_cosh, jit_insn_cosh);
    unary_op!(insn_exp, jit_insn_exp);
    unary_op!(insn_log, jit_insn_log);
    unary_op!(insn_log10, jit_insn_log10);
    unary_op!(insn_sin, jit_insn_sin);
    unary_op!(insn_sinh, jit_insn_sinh);
    unary_op!(insn_sqrt, jit_insn_sqrt);
    unary_op!(insn_tan, jit_insn_tan);
    unary_op!(insn_tanh, jit_insn_tanh);

    ret_op!(insn_return, jit_insn_return);

    unary_op!(insn_not, jit_insn_not);

    pub fn insn_branch(&self, label: &mut Label) {
        unsafe { jit_insn_branch(self.function, label.into()) };
    }

    pub fn insn_branch_if(&self, value: &Value, label: &mut Label) {
        unsafe { jit_insn_branch_if(self.function, value.value, label.into()) };
    }

    pub fn insn_branch_if_not(&self, value: &Value, label: &mut Label) {
        unsafe { jit_insn_branch_if_not(self.function, value.value, label.into()); }
    }

    pub fn insn_load(&mut self, ptr: &Value) -> Value {
        unsafe {
            let value = jit_insn_load(self.function, ptr.value);
            // let value_type = jit_value_get_type(value);
            Value::new(value)
        }
    }

    pub fn insn_store(&mut self, ptr: &Value, value: &Value) {
        unsafe {
            jit_insn_store(self.function, ptr.value, value.value);
        }
    }

    pub fn insn_call(&mut self, function: &Function, args: Vec<Value>) -> Value {
        let mut values: Vec<jit_value_t> = args.iter().map(|arg| arg.value.clone()).collect();
        let val = unsafe {
            jit_insn_call(self.function, "-no-name-".as_ptr() as *mut c_char, function.function, function.signature, values.as_mut_ptr(), args.len() as c_uint, 0)
        };
        Value::new(val)
    }

    pub fn insn_load_relative(&mut self, base_ptr: &Value, offset_bytes: ::std::os::raw::c_long, typ: &JitType) -> Value {
        let res = unsafe {
            jit_insn_load_relative(self.function, base_ptr.value, offset_bytes, typ.inner)
        };
        Value::new(res)
    }

    pub fn insn_store_relative(&mut self, dest_ptr: &Value, offset_bytes: ::std::os::raw::c_long, value: &Value) {
        unsafe {
            jit_insn_store_relative(self.function, dest_ptr.value, offset_bytes, value.value)
        };
    }

    pub fn create_value(&mut self, typ: &JitType) -> Value {
        Value::new(unsafe { jit_value_create(self.function, typ.inner) })
    }

    pub fn create_value_int(&mut self) -> Value {
        Value::new(unsafe {
            jit_value_create(self.function, jit_type_int)
        })
    }

    pub fn create_value_void_ptr(&mut self) -> Value {
        Value::new(unsafe {
            jit_value_create(self.function, jit_type_void_ptr)
        })
    }
    pub fn create_value_float32(&mut self) -> Value {
        Value::new(unsafe {
            jit_value_create(self.function, jit_type_float32)
        })
    }
    pub fn create_value_float64(&mut self) -> Value {
        Value::new(unsafe {
            jit_value_create(self.function, jit_type_float64)
        })
    }

    pub fn insn_label(&self, label: &mut Label) {
        unsafe { jit_insn_label(self.function, label.into()); }
    }

    // Round value up towards positive infinity.
    pub fn insn_ceil(&self, value: &Value) -> Value {
        Value::new(unsafe { jit_insn_ceil(self.function, value.value) })
    }

    // Round value down towards negative infinity.
    pub fn insn_floor(&self, value: &Value) -> Value {
        Value::new(unsafe { jit_insn_floor(self.function, value.value) })
    }

    //  Round value to the nearest integer. Half-way cases are rounded to the even number.
    pub fn insn_rint(&self, value: &Value) -> Value {
        Value::new(unsafe { jit_insn_rint(self.function, value.value) })
    }

    // Round value to the nearest integer. Half-way cases are rounded away from zero.
    pub fn insn_round(&self, value: &Value) -> Value {
        Value::new(unsafe { jit_insn_round(self.function, value.value) })
    }

    // Round value towards zero.
    pub fn insn_trunc(&self, value: &Value) -> Value {
        Value::new(unsafe { jit_insn_trunc(self.function, value.value) })
    }

    // Load the effective address of an element of type elem_type at position index within the
    // array starting at base_addr. Essentially, this computes the expression base_addr + index *
    // sizeof(elem_type), but may be more efficient than performing the steps with jit_insn_mul and jit_insn_add.
    pub fn insn_load_elem_address(&self, base_addr: &Value, index: &Value, elem_type: &JitType) -> Value {
        Value::new(unsafe { jit_insn_load_elem_address(self.function, base_addr.value, index.value, elem_type.inner) })
    }

    constant_fn!(create_float32_constant, ::std::os::raw::c_float, jit_type_float32, jit_value_create_float32_constant);
    constant_fn!(create_float64_constant, ::std::os::raw::c_double, jit_type_float64, jit_value_create_float64_constant);
    constant_fn!(create_long_constant, ::std::os::raw::c_long, jit_type_long, jit_value_create_long_constant);

    constant_fn_long!(create_sbyte_constant, ::std::os::raw::c_char, jit_type_sbyte, jit_value_create_long_constant);
    constant_fn_long!(create_ubyte_constant, ::std::os::raw::c_uchar, jit_type_ubyte, jit_value_create_long_constant);
    constant_fn_long!(create_short_constant, ::std::os::raw::c_long, jit_type_short, jit_value_create_long_constant);
    constant_fn_long!(create_ushort_constant, ::std::os::raw::c_ulong, jit_type_ushort, jit_value_create_long_constant);
    constant_fn_long!(create_int_constant, ::std::os::raw::c_int, jit_type_int, jit_value_create_long_constant);
    constant_fn_long!(create_ulong_constant, ::std::os::raw::c_ulong, jit_type_ulong, jit_value_create_long_constant);
    constant_fn_long!(create_uint_constant, ::std::os::raw::c_uint, jit_type_uint, jit_value_create_long_constant);
    constant_fn_long!(create_nint_constant, ::std::os::raw::c_int, jit_type_nint, jit_value_create_long_constant);
    constant_fn_long!(create_nuint_constant, ::std::os::raw::c_int, jit_type_nint, jit_value_create_long_constant);
    constant_fn_long!(create_void_ptr_constant, *mut ::std::os::raw::c_void, jit_type_void_ptr, jit_value_create_long_constant);
}

