#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

include!("./trimmed_bindings.rs");

#[test]
fn test_regex() {
    unsafe {
        let reg = REcompile("a+".as_ptr() as *mut std::os::raw::c_char, 2 as ::std::os::raw::c_ulong);
        assert!(REtest("abc".as_ptr() as *mut i8, 3, reg) != 0);
        REdestroy(reg);
    }
}