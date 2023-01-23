use std::fmt::{Debug, Formatter};
use std::str::from_utf8_unchecked;
use crate::awk_str::RcAwkStr;

#[derive(Clone, PartialEq)]
pub enum RuntimeScalar {
    Str(RcAwkStr),
    StrNum(RcAwkStr),
    Num(f64),
}

impl Debug for RuntimeScalar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Unsafe but this is test only. Str might not be utf8
        match self {
            RuntimeScalar::Str(s) => {
                let str = unsafe {from_utf8_unchecked(s) };
                write!(f, "s'{}'", str)
            }
            RuntimeScalar::StrNum(s) => {
                let str = unsafe {from_utf8_unchecked(s) };
                write!(f, "n'{}'", str)
            }
            RuntimeScalar::Num(num) => {
                write!(f, "{}", num)
            }
        }

    }
}

impl RuntimeScalar {
    pub fn truthy(&self) -> bool {
        match self {
            RuntimeScalar::Str(s) => s.len() != 0,
            RuntimeScalar::StrNum(s) => s.len() != 0,
            RuntimeScalar::Num(n) => *n != 0.0,
        }
    }
}
