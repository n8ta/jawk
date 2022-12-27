use std::fmt::{Debug, Formatter};
use std::mem;
use std::ops::Deref;
use std::str::Utf8Error;

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
}