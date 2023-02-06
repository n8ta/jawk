use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::{Rc};

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq, Hash)]
pub struct RcAwkStr {
    str: Rc<AwkStr>,
}

impl Deref for RcAwkStr {
    type Target = AwkStr;

    fn deref(&self) -> &Self::Target {
        &self.str
    }
}

impl RcAwkStr {
    pub unsafe fn into_raw(self) -> *const AwkStr {
        Rc::into_raw(self.str)
    }
    pub unsafe fn from_raw(string: *const AwkStr) -> RcAwkStr {
        let original = unsafe { Rc::from_raw(string) };
        let copy = original.clone();
        Rc::into_raw(original);
        Self {
            str: copy
        }
    }
    pub fn new(str: AwkStr) -> Self {
        Self { str: Rc::new(str) }
    }
    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        Self { str: Rc::new(AwkStr::new(bytes)) }
    }

    pub fn downgrade_or_clone(self) -> AwkStr {
        match Rc::try_unwrap(self.str) {
            Ok(str) => str,
            Err(rc) => {
                (*rc).clone()
            }
        }
    }
}

#[derive(PartialEq, PartialOrd, Clone, Eq, Hash)]
pub struct AwkStr {
    bytes: Vec<u8>,
}

impl Deref for AwkStr {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl Debug for AwkStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8(self.bytes.to_vec()).unwrap();
        f.write_str(&s)
    }
}

impl From<&str> for AwkStr {
    fn from(s: &str) -> Self {
        AwkStr { bytes: s.as_bytes().to_vec() }
    }
}

impl From<String> for AwkStr {
    fn from(s: String) -> Self {
        AwkStr { bytes: s.into_bytes() }
    }
}

impl AwkStr {
    pub fn new(bytes: Vec<u8>) -> AwkStr {
        Self { bytes }
    }
    pub fn new_rc(bytes: Vec<u8>) -> RcAwkStr {
        RcAwkStr::new(AwkStr::new(bytes))
    }
    pub fn new_rc_str(s: &str) -> RcAwkStr {
        Self::new_rc(s.to_string().into_bytes())
    }
    pub fn with_capacity(cap: usize) -> AwkStr {
        Self { bytes: Vec::with_capacity(cap) }
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn make_ascii_lowercase(&mut self) {
        self.bytes.make_ascii_lowercase()
    }
    pub fn make_ascii_uppercase(&mut self) {
        self.bytes.make_ascii_uppercase()
    }
    pub fn to_ascii_lowercase(&self) -> Self {
        AwkStr::new(self.bytes.to_ascii_lowercase())
    }
    pub fn to_ascii_uppercase(&self) -> Self {
        AwkStr::new(self.bytes.to_ascii_uppercase())
    }
    pub fn push_str(&mut self, other: &[u8]) {
        self.bytes.extend_from_slice(&other)
    }
    pub fn as_mut_vec(&mut self) -> &mut Vec<u8> {
        &mut self.bytes
    }
    pub fn clear(&mut self) {
        self.bytes.clear();
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
    pub fn rc(self) -> RcAwkStr {
        RcAwkStr::new(self)
    }
    pub fn truthy(&self) -> bool {
        self.bytes.len() != 0
    }
}