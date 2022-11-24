extern crate core;

use std::os::raw::c_char;
use mawk_regex_sys::{REtest, REcompile, PTR, REdestroy};

fn new(regex: &str) -> PTR {
    unsafe {
        REcompile(regex.as_ptr() as *mut c_char, regex.len() as ::std::os::raw::c_ulong)
    }
}

fn matches(ptr: &PTR, str: &str) -> bool {
    unsafe {
        REtest(str.as_ptr() as *mut c_char, str.len() as ::std::os::raw::c_ulong, *ptr) != 0
    }
}


fn main() {
    // Run it a bunch, help stop any memory corruption
    for _iter in 0..10_000 {
        let regex = new("abc+");
        assert!(matches(&regex, "abc"));
        assert!(matches(&regex, "abcc"));
        assert!(matches(&regex, "abccc"));
        assert!(!matches(&regex, "ab"));
        unsafe {
            REdestroy(regex)
        }
    }
}