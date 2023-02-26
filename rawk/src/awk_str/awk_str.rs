use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use crate::awk_str::awk_byte_str::AwkByteStr;
use crate::awk_str::RcAwkStr;
use crate::util::{unwrap, unwrap_err};

pub struct AwkStr {
    // Invariant: RcAwkStrs here have 1 strong reference and thus can
    // be safelty mutated via get_mut_vec function

    // It's helpful to keep a collection of OwnedRcAwkStr's around
    // as it's faster than mallocing new Rc's every time we want a
    // new empty string.
    backing: Rc<AwkByteStr>,
}

impl AwkStr {
    pub fn new_empty() -> Self {
        Self {
            backing: Rc::new(AwkByteStr::new(vec![]))
        }
    }
    pub fn new(str: RcAwkStr) -> Option<Self> {
        if str.strong_count() == 1 && str.weak_count() == 0 {
            Some(AwkStr { backing: str.done() })
        } else {
            None
        }
    }
    pub fn new_or_clone(str: RcAwkStr) -> Self {
        if str.strong_count() == 1 && str.weak_count() == 0 {
            AwkStr { backing: str.done() }
        } else {
            AwkStr { backing: Rc::new(AwkByteStr::new(str.bytes().to_vec())) }
        }
    }
    pub fn new_string(string: String) -> Self {
        Self::new_from_vec(string.into_bytes())
    }
    pub fn new_from_vec(vec: Vec<u8>) -> Self {
        Self {
            backing: Rc::new(AwkByteStr::new(vec)),
        }
    }
    fn get_mut_awkstr(&mut self) -> &mut AwkByteStr {
        let mutable_str = unwrap(Rc::get_mut(&mut self.backing));
        mutable_str
    }
    pub fn rc(self) -> RcAwkStr {
        RcAwkStr::rc(self.backing)
    }

    // Avoid allocating an Rc by overwriting the Vec inside an existing Rc
    pub fn overwrite_with(&mut self, str: Vec<u8>) {
        let mut byte_str = AwkByteStr::new(str);
        std::mem::swap(self.get_mut_awkstr(), &mut byte_str);
    }

    pub fn clone(&self) -> Self {
        Self{ backing: Rc::new(AwkByteStr::new(self.backing.bytes().to_vec())) }
    }

    pub fn done(self) -> Vec<u8> {
        let res = unwrap_err(Rc::try_unwrap(self.backing));
        res.done()
    }
}

impl Debug for AwkStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.backing)
    }
}

impl PartialEq<&str> for AwkStr {
    fn eq(&self, other: &&str) -> bool {
        self.backing.bytes() == other.as_bytes()
    }
}

impl Deref for AwkStr {
    type Target = AwkByteStr;

    fn deref(&self) -> &Self::Target {
        &self.backing
    }
}

impl DerefMut for AwkStr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unwrap(Rc::get_mut(&mut self.backing))
    }
}