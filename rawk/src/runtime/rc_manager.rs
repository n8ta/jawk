use std::rc::Rc;
use crate::awk_str::{AwkStr, RcAwkStr};
use crate::vm::{RuntimeScalar, StringScalar};

pub struct RcManager {
    cache: Vec<AwkStr>,
}

impl RcManager {
    pub fn new() -> Self {
        Self { cache: Vec::with_capacity(8) }
    }
    pub fn drop_scalar(&mut self, scalar: RuntimeScalar) {
        let str = match scalar {
            RuntimeScalar::Str(s) => s,
            RuntimeScalar::StrNum(s) => s,
            RuntimeScalar::Num(_) => return,
        };
        self.drop(str);
    }
    pub fn drop_str(&mut self, scalar: StringScalar) {
        let s= match scalar {
            StringScalar::Str(s) => s,
            StringScalar::StrNum(s) => s,
        };
        self.drop(s);
    }
    pub fn drop(&mut self, str: RcAwkStr) {
        if let Some(mut owned) = AwkStr::new(str) {
            owned.clear();
            self.cache.push(owned)
        }
    }
    pub fn get(&mut self) -> AwkStr {
        self.cache.pop().unwrap_or_else(|| AwkStr::new_empty())
    }
    pub fn from_vec(&mut self, bytes: Vec<u8>) -> AwkStr {
        let mut rc = self.get();
        rc.overwrite_with(bytes);
        rc
    }
    pub fn copy_from_slice(&mut self, bytes: &[u8]) -> AwkStr {
        let mut rc = self.get();
        rc.push_str(bytes);
        rc
    }
}