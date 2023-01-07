use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::awk_str::AwkStr;
use crate::codegen::Tag;
use crate::runtime::string_converter::Converter;

pub enum RuntimeValue {
    Float(f64),
    Str(Rc<AwkStr>),
    StrNum(Rc<AwkStr>),
}

impl Debug for RuntimeValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (name, ptr) = match self {
            RuntimeValue::Float(flt) => return write!(f, "flt:{}", flt),
            RuntimeValue::Str(ptr) => ("str", ptr),
            RuntimeValue::StrNum(ptr) => ("strnum", ptr),
        };
        let bytes = ptr.bytes();
        let slice = std::str::from_utf8(bytes).unwrap();
        write!(f, "{}:`{}`", name,slice)
    }
}

impl RuntimeValue {
    pub fn new(tag: Tag, float: f64, pointer: *const AwkStr) -> Self {
        match tag {
            Tag::FloatTag => RuntimeValue::Float(float),
            Tag::StringTag => RuntimeValue::Str(unsafe { Rc::from_raw(pointer) }),
            Tag::StrnumTag => RuntimeValue::StrNum(unsafe { Rc::from_raw(pointer) }),
        }
    }
    pub fn is_numeric(&self, conv: &mut Converter) -> bool {
        match self {
            RuntimeValue::Float(_) => true,
            RuntimeValue::Str(_) => false,
            RuntimeValue::StrNum(ptr) => {
                // TODO: Changing each occurrence of the decimal point character from the current locale to a period.
                conv.str_to_num(ptr).is_some()
            }
        }
    }
    pub fn clone(&self) -> (Tag, f64, *const AwkStr) {
        let (tag, ptr) = match self {
            RuntimeValue::Float(f) => return (Tag::FloatTag, *f, 0 as *const AwkStr),
            RuntimeValue::Str(str) => (Tag::StringTag, str),
            RuntimeValue::StrNum(str) => (Tag::StrnumTag, str),
        };
        let rced = ptr.clone();
        let copy = Rc::into_raw(rced);
        (tag, 0.0, copy)
    }
}