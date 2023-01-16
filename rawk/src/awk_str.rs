use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;

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
    pub fn new_rc(bytes: Vec<u8>) -> Rc<AwkStr> {
        Rc::new(Self { bytes })
    }
    pub fn new_rc_str(s: &str) -> Rc<AwkStr> {
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
}

// Clone underlying bytes if needed OR if Rc has 1 reference downgrade into AwkStr
pub fn unwrap_awkstr_rc(str: Rc<AwkStr>) -> AwkStr {
    match Rc::try_unwrap(str) {
        Ok(str) => str,
        Err(rc) => (*rc).clone(),
    }
}