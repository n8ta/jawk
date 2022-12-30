use std::rc::Rc;
use crate::awk_str::AwkStr;
use crate::codegen::Tag;



pub enum RuntimeValue {
    Float(f64),
    Str(Rc<AwkStr>),
    StrNum(Rc<AwkStr>),
}

impl RuntimeValue {
    pub fn new(tag: Tag, float: f64, pointer: *const AwkStr) -> Self {
        match tag {
            Tag::FloatTag => RuntimeValue::Float(float),
            Tag::StringTag => RuntimeValue::Str(unsafe { Rc::from_raw(pointer) }),
            Tag::StrnumTag => RuntimeValue::StrNum(unsafe { Rc::from_raw(pointer) }),
        }
    }
    pub fn is_numeric(&self) -> bool {
        match self {
            RuntimeValue::Float(_) => true,
            RuntimeValue::Str(_) => false,
            RuntimeValue::StrNum(_) => true,
        }

    }
}