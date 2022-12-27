#[cfg(test)]
mod tests;

extern crate core;

use std::os::raw::c_char;
use mawk_regex_sys::{REtest, REcompile, PTR, REdestroy, size_t, REmatch};

pub struct Regex {
    ptr: PTR,
}

pub struct Match {
    pub start: usize,
    pub len: usize,
}

impl Regex {
    pub fn new(regex: &[u8]) -> Self {
        unsafe {
            Regex { ptr: REcompile(regex.as_ptr() as *mut c_char, regex.len() as ::std::os::raw::c_ulong) }
        }
    }
    pub fn matches(&self, str: &[u8]) -> bool {
        unsafe {
            let res = REtest(str.as_ptr() as *mut c_char, str.len() as ::std::os::raw::c_ulong, self.ptr);
            return res != 0
        }
    }

    pub fn match_idx(&self, str: &[u8]) -> Option<Match> {
        let mut match_len: Box<size_t> = Box::new(0);
        let result_ptr = unsafe {
            REmatch(str.as_ptr() as *mut c_char,
                    str.len() as ::std::os::raw::c_ulong,
            self.ptr,
                &mut *match_len as *mut size_t,
                0
            )
        };
        if result_ptr == 0 as *mut c_char {
            return None
        } else {
            let idx = unsafe { result_ptr.offset_from(str.as_ptr() as *const c_char) };
            debug_assert!(idx >= 0);
            Some(Match { start: idx as usize, len: (*match_len) as usize })
        }
    }

}

impl Drop for Regex {
    fn drop(&mut self) {
        unsafe {
            REdestroy(self.ptr)
        }
    }
}
