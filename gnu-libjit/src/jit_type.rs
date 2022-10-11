use std::os::raw::{c_long, c_uint};
use gnu_libjit_sys::{jit_type_create_pointer, jit_type_create_struct, jit_type_get_offset, jit_type_t};

#[derive(Clone, Copy, Debug)]
pub struct JitType {
    pub(crate) inner: jit_type_t,
}

impl JitType {
    pub(crate) fn new(inner: jit_type_t) -> JitType {
        JitType { inner }
    }

    // For input type T returns a *T type.
    pub fn type_create_pointer(&self) -> JitType {
        let ptr_type = unsafe { jit_type_create_pointer(self.inner, 1) };
        JitType::new(ptr_type)
    }

    pub fn new_struct(fields: Vec<JitType>) -> Self {
        let mut fields_raw: Vec<jit_type_t> = fields.iter().map(|f| f.inner).collect();
        let fields_ptr = fields_raw.as_mut_ptr();
        let typ = unsafe { jit_type_create_struct(fields_ptr, fields.len() as c_uint, 1) };
        JitType { inner: typ }
    }

    // Returns 0 for non-structs
    pub fn field_offset(&self, field_idx: usize) -> c_long {
        let offset = unsafe {
            jit_type_get_offset(self.inner, field_idx as c_uint)
        };
        offset as c_long
    }
}
