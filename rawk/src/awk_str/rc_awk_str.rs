use std::ops::Deref;
use std::rc::Rc;
use crate::awk_str::awk_byte_str::AwkByteStr;
use crate::awk_str::AwkStr;

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct RcAwkStr {
    str: Rc<AwkByteStr>,
}

impl Deref for RcAwkStr {
    type Target = AwkByteStr;

    fn deref(&self) -> &Self::Target {
        &self.str
    }
}

impl RcAwkStr {
    pub unsafe fn into_raw(self) -> *const AwkByteStr {
        Rc::into_raw(self.str)
    }
    pub unsafe fn from_raw(string: *const AwkByteStr) -> RcAwkStr {
        let original = unsafe { Rc::from_raw(string) };
        let copy = original.clone();
        Rc::into_raw(original);
        Self {
            str: copy
        }
    }
    pub fn rc(str: Rc<AwkByteStr>) -> Self {
        Self { str }
    }
    pub fn new(str: AwkByteStr) -> Self {
        Self {
            str: Rc::new(str),
        }
    }
    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        Self { str: Rc::new(AwkByteStr::new(bytes)) }
    }
    pub fn new_str(bytes: &str) -> Self {
        Self { str: Rc::new(AwkByteStr::new(bytes.as_bytes().to_vec())) }
    }

    // Builds an owned awk str by downgrading the Rc or by cloning the underlying data
    pub fn downgrade_or_clone(self) -> AwkStr {
        AwkStr::new_or_clone(self)
    }
    pub fn downgrade_or_clone_to_vec(self) -> Vec<u8> {
        match Rc::try_unwrap(self.str) {
            Ok(awk_byte_str) => awk_byte_str.done(), // use the vec we already have
            Err(rc) => rc.to_vec(), // make a copy
        }
    }

    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.str)
    }
    pub fn weak_count(&self) -> usize {
        Rc::weak_count(&self.str)
    }
    pub fn done(self) -> Rc<AwkByteStr> {
        self.str
    }
}