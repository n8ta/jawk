pub type size_t = ::std::os::raw::c_ulong;
pub type PTR = *mut ::std::os::raw::c_void;

extern "C" {
    pub fn REmatch(
        arg1: *mut ::std::os::raw::c_char,
        arg2: size_t,
        arg3: PTR,
        arg4: *mut size_t,
        arg5: ::std::os::raw::c_int,
    ) -> *mut ::std::os::raw::c_char;
}
extern "C" {
    pub fn REdestroy(arg1: PTR);
}
extern "C" {
    pub fn REcompile(arg1: *mut ::std::os::raw::c_char, arg2: size_t) -> PTR;
}
extern "C" {
    pub fn REtest(
        arg1: *mut ::std::os::raw::c_char,
        arg2: size_t,
        arg3: PTR,
    ) -> ::std::os::raw::c_int;
}
