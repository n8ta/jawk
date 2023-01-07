use std::rc::Rc;
use crate::awk_str::AwkStr;
use crate::runtime::value::RuntimeValue;

pub struct HackyAlloc {
    fast_alloc: Vec<Rc<AwkStr>>,
}

impl HackyAlloc {
    pub fn new() -> Self {
        Self {
            fast_alloc: vec![],
        }
    }
    pub fn take(&mut self) -> Option<Rc<AwkStr>> {
        self.fast_alloc.pop()
    }
    pub fn alloc_awkstr(&mut self, bytes: &[u8]) -> Rc<AwkStr> {
        if let Some(mut current) = self.fast_alloc.pop() {
            {
                let mutable = match Rc::get_mut(&mut current) {
                    None => panic!("rc should be unique"),
                    Some(some) => some,
                };
                mutable.clear();
                mutable.push_str(bytes);
            }
            current
        } else {
            Rc::new(AwkStr::new(bytes.to_vec()))
        }
    }
    pub fn drop(&mut self, str: Rc<AwkStr>) {
        if Rc::strong_count(&str) == 1 && Rc::weak_count(&str) == 0
            && self.fast_alloc.len() < 128 {
            self.fast_alloc.push(str)
        }
    }
    pub fn drop_opt_rtval(&mut self, str: Option<RuntimeValue>) {
        if let Some(rt) = str {
            self.drop_rtval(rt)
        }
    }
    pub fn drop_rtval(&mut self, str: RuntimeValue) {
        match str {
            RuntimeValue::Float(_) => {}
            RuntimeValue::Str(ptr) => self.drop(ptr),
            RuntimeValue::StrNum(ptr) => self.drop(ptr),
        }
    }
}