use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::str::from_utf8_unchecked;
use crate::awk_str::{AwkStr, RcAwkStr};


#[derive(Clone, PartialEq)]
pub enum RuntimeScalar {
    Str(RcAwkStr),
    StrNum(RcAwkStr),
    Num(f64),
}
impl RuntimeScalar {
    pub fn truthy(&self) -> bool {
        match self {
            RuntimeScalar::Str(s) => s.truthy(),
            RuntimeScalar::StrNum(s) => s.truthy(),
            RuntimeScalar::Num(num) => *num != 0.0,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum StringScalar {
    Str(RcAwkStr),
    StrNum(RcAwkStr),
}
impl StringScalar {
    pub fn downgrade_or_clone(self) -> AwkStr {
        let s = match self {
            StringScalar::Str(s) => s,
            StringScalar::StrNum(s) => s,
        };
        s.downgrade_or_clone()
    }
    pub fn truthy(&self) -> bool {
       match self {
           StringScalar::Str(s) => s.truthy(),
           StringScalar::StrNum(s) => s.truthy(),
       }
    }
}
impl Deref for StringScalar {
    type Target = RcAwkStr;

    fn deref(&self) -> &Self::Target {
        match self {
            StringScalar::Str(s) => s,
            StringScalar::StrNum(s) => s,
        }
    }
}
impl Into<RuntimeScalar> for StringScalar {
    fn into(self) -> RuntimeScalar {
        match self {
            StringScalar::Str(s) => RuntimeScalar::Str(s),
            StringScalar::StrNum(s) => RuntimeScalar::StrNum(s),
        }
    }
}
impl Debug for StringScalar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let runtime_scalar: RuntimeScalar = self.clone().into();
        write!(f, "{:?}", runtime_scalar)
    }
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