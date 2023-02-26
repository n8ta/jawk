use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use crate::awk_str::RcAwkStr;

#[derive(PartialEq, PartialOrd, Clone, Eq, Hash)]
pub struct AwkByteStr {
    bytes: Vec<u8>,
}

impl Deref for AwkByteStr {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl Debug for AwkByteStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = String::from_utf8(self.bytes.to_vec()).unwrap();
        f.write_str(&s)
    }
}

impl From<&str> for AwkByteStr {
    fn from(s: &str) -> Self {
        AwkByteStr { bytes: s.as_bytes().to_vec() }
    }
}

impl From<String> for AwkByteStr {
    fn from(s: String) -> Self {
        AwkByteStr { bytes: s.into_bytes() }
    }
}

impl AwkByteStr {
    pub fn new(bytes: Vec<u8>) -> AwkByteStr {
        Self { bytes }
    }
    fn new_rc(bytes: Vec<u8>) -> RcAwkStr {
        RcAwkStr::new(AwkByteStr::new(bytes))
    }
    fn with_capacity(cap: usize) -> AwkByteStr {
        Self { bytes: Vec::with_capacity(cap) }
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn bytes_mut(&mut self) -> &mut Vec<u8> {
        &mut self.bytes
    }

    pub fn make_ascii_lowercase(&mut self) {
        self.bytes.make_ascii_lowercase()
    }
    pub fn make_ascii_uppercase(&mut self) {
        self.bytes.make_ascii_uppercase()
    }
    pub fn to_ascii_lowercase(&self) -> Self {
        AwkByteStr::new(self.bytes.to_ascii_lowercase())
    }
    pub fn to_ascii_uppercase(&self) -> Self {
        AwkByteStr::new(self.bytes.to_ascii_uppercase())
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
    pub fn done(self) -> Vec<u8> {
        self.bytes
    }
}