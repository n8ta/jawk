use gnu_libjit::Value;

#[derive(Clone)]
pub struct ValueT {
    pub tag: Value,
    pub float: Value,
    pub pointer: Value,
}

impl ValueT {
    pub fn new(tag: Value, float: Value, pointer: Value) -> ValueT {
        ValueT {
            tag,
            float,
            pointer,
        }
    }
}
impl Into<Vec<Value>> for &ValueT {
    fn into(self) -> Vec<Value> {
        vec![self.tag.clone(), self.float.clone(), self.pointer.clone()]
    }
}

pub type ValuePtrT = ValueT;
