#[cfg(test)]
mod tests;

extern crate core;

use std::os::raw::c_char;

#[cfg(feature="thread_safe")]
use std::sync::Mutex;
#[cfg(feature="thread_safe")]
use once_cell::sync::Lazy;

use mawk_regex_sys::{REtest, REcompile, PTR, REdestroy, size_t, REmatch};

pub struct Regex {
    ptr: PTR,
}

#[derive(Debug, PartialEq)]
pub struct Match {
    pub start: usize,
    pub len: usize,
}

// Mawk is not thread safe.... But it really slows down my rust tests to have to
// run them single threaded. In thread_safe mode use a global mutex to prevent crashes and allow concurrent
// tests
#[cfg(feature = "thread_safe")]
static GLOBAL_MUTEX: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

impl Regex {
    pub fn new(regex: &[u8]) -> Self {
        #[cfg(feature="thread_safe")]
        let lck = GLOBAL_MUTEX.lock().unwrap();
        let reg = unsafe {
            Regex { ptr: REcompile(regex.as_ptr() as *mut c_char, regex.len() as ::std::os::raw::c_ulong) }
        };
        #[cfg(feature="thread_safe")]
        drop(lck);
        reg
    }
    pub fn matches(&self, str: &[u8]) -> bool {
        #[cfg(feature="thread_safe")]
            let lck = GLOBAL_MUTEX.lock().unwrap();
        let is_match = unsafe {
            let res = REtest(str.as_ptr() as *mut c_char, str.len() as ::std::os::raw::c_ulong, self.ptr);
            res != 0
        };
        #[cfg(feature="thread_safe")]
        drop(lck);
        is_match
    }

    pub fn match_idx(&self, str: &[u8]) -> Option<Match> {
        #[cfg(feature="thread_safe")]
        let lck = GLOBAL_MUTEX.lock().unwrap();
        let mut match_len: Box<size_t> = Box::new(0);
        let result_ptr = unsafe {
            REmatch(str.as_ptr() as *mut c_char,
                    str.len() as ::std::os::raw::c_ulong,
                    self.ptr,
                    &mut *match_len as *mut size_t,
                    0,
            )
        };
        let res = if result_ptr == 0 as *mut c_char {
            return None;
        } else {
            let idx = unsafe { result_ptr.offset_from(str.as_ptr() as *const c_char) };
            debug_assert!(idx >= 0);
            let idx = idx as usize;
            let match_len = *match_len as usize;
            let len = std::cmp::min(match_len, str.len() - idx); // TODO: why does mawk sometimes return len 1 longer than len of str

            Some(Match { start: idx as usize, len })
        };
        #[cfg(feature="thread_safe")]
        drop(lck);
        res
    }
}

impl Drop for Regex {
    fn drop(&mut self) {
        #[cfg(feature="thread_safe")]
        let lck = GLOBAL_MUTEX.lock().unwrap();
        unsafe {
            REdestroy(self.ptr)
        }
        #[cfg(feature="thread_safe")]
        drop(lck)
    }
}
