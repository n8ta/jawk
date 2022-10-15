use gnu_libjit::Value;
use crate::parser::ScalarType;

#[derive(Clone)]
pub struct ValueT {
    pub tag: Value,
    pub float: Value,
    pub pointer: Value,
    pub typ: ScalarType,
}

impl ValueT {
    pub fn new(tag: Value, float: Value, pointer: Value, typ: ScalarType) -> ValueT {
        ValueT {
            typ,
            tag,
            float,
            pointer,
        }
    }
    pub fn string(tag: Value, float: Value, pointer: Value) -> ValueT {
        Self::new(tag, float, pointer, ScalarType::String)

    }
    pub fn float(tag: Value, float: Value, pointer: Value) -> ValueT {
        Self::new(tag, float, pointer, ScalarType::Float)

    }
    pub fn var(tag: Value, float: Value, pointer: Value) -> ValueT {
        Self::new(tag, float, pointer, ScalarType::Variable)
    }

}
impl Into<Vec<Value>> for &ValueT {
    fn into(self) -> Vec<Value> {
        vec![self.tag.clone(), self.float.clone(), self.pointer.clone()]
    }
}

pub type ValuePtrT = ValueT;
