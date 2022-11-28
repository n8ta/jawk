use crate::codegen::{ValueT, FLOAT_TAG, STRING_TAG};
use crate::columns::Columns;
use crate::lexer::BinOp;
use crate::parser::ScalarType;
use crate::runtime::arrays::Arrays;
use crate::runtime::{ErrorCode, Runtime};
use gnu_libjit::{Abi, Context, Function, Value};
use hashbrown::HashMap;
use lru_cache::LruCache;
use mawk_regex::Regex;
use std::ffi::c_void;
use std::io::{BufWriter, StdoutLock, Write};
use std::rc::Rc;

use crate::runtime::float_parser::{string_to_float, FloatParser};
use crate::{runtime_fn, runtime_fn_no_args, runtime_fn_no_ret};

pub extern "C" fn print_string(data: *mut c_void, value: *mut String) {
    let data = cast_to_runtime_data(data);
    let str = unsafe { Rc::from_raw(value) };
    if str.ends_with("\n") {
        data.stdout
            .write_all(str.as_bytes())
            .expect("failed to write to stdout")
    } else {
        data.stdout
            .write_all(str.as_bytes())
            .expect("failed to write to stdout");
        data.stdout
            .write_all("\n".as_bytes())
            .expect("failed to write to stdout");
    }
    // implicitly consuming str here.
}

pub extern "C" fn print_float(data: *mut c_void, value: f64) {
    let data = cast_to_runtime_data(data);

    data.stdout.write_fmt(format_args!("{}\n", value)).unwrap();
}

extern "C" fn next_line(data: *mut c_void) -> f64 {
    let data = cast_to_runtime_data(data);
    if data.columns.next_line() {
        1.0
    } else {
        0.0
    }
}

extern "C" fn column(
    data_ptr: *mut c_void,
    tag: i8,
    value: f64,
    pointer: *const String,
) -> *mut String {
    let data = cast_to_runtime_data(data_ptr);
    let idx = if tag == FLOAT_TAG {
        value
    } else {
        string_to_number(data_ptr, pointer)
    };
    let idx = idx.round() as usize;
    Rc::into_raw(Rc::new(data.columns.get(idx))) as *mut String
}

extern "C" fn free_string(_data: *mut c_void, string: *const String) {
    unsafe { Rc::from_raw(string) };
}

extern "C" fn free_if_string(_data: *mut c_void, tag: i8, string: *const String) {
    if tag == STRING_TAG {
        unsafe { Rc::from_raw(string) };
    }
}

extern "C" fn concat(
    _data_ptr: *mut c_void,
    left: *const String,
    right: *const String,
) -> *const String {
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };
    let mut lhs: String = match Rc::try_unwrap(lhs) {
        Ok(str) => str,
        Err(rc) => (*rc).clone(),
    };
    lhs.push_str(&rhs);
    Rc::into_raw(Rc::new(lhs))
}

extern "C" fn empty_string(_data_ptr: *mut c_void) -> *const String {
    Rc::into_raw(Rc::new("".to_string()))
}

extern "C" fn binop(
    data: *mut c_void,
    l_ptr: *const String,
    r_ptr: *const String,
    binop: BinOp,
) -> std::os::raw::c_double {
    let left = unsafe { Rc::from_raw(l_ptr) };
    let right = unsafe { Rc::from_raw(r_ptr) };
    let data = cast_to_runtime_data(data);

    let res = match binop {
        BinOp::Greater => left > right,
        BinOp::GreaterEq => left >= right,
        BinOp::Less => left < right,
        BinOp::LessEq => left <= right,
        BinOp::BangEq => left != right,
        BinOp::EqEq => left == right,
        BinOp::MatchedBy => {
            let reg = match data.regex_cache.get_mut(&*right) {
                Some(cached_regex) => cached_regex,
                None => {
                    let re = Regex::new(&right);
                    data.regex_cache.insert((&*right).clone(), re);
                    data.regex_cache.get_mut(&*right).unwrap()
                }
            };
            reg.matches(&left)
        }
        BinOp::NotMatchedBy => {
            let reg = match data.regex_cache.get_mut(&*right) {
                Some(cached_regex) => cached_regex,
                None => {
                    let re = Regex::new(&right);
                    data.regex_cache.insert((&*right).clone(), re);
                    data.regex_cache.get_mut(&*right).unwrap()
                }
            };
            !reg.matches(&left)
        }
    };
    if res {
        1.0
    } else {
        0.0
    }
    // Implicitly drop left and right
}

extern "C" fn string_to_number(_data: *mut c_void, ptr: *const String) -> f64 {
    let string = unsafe { Rc::from_raw(ptr) };
    let res = string_to_float(&*string);
    Rc::into_raw(string);
    res
}

extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const String {
    let runtime_data = cast_to_runtime_data(data_ptr);
    Rc::into_raw(Rc::new(runtime_data.float_parser.parse(value)))
}

extern "C" fn copy_string(_data_ptr: *mut c_void, ptr: *mut String) -> *const String {
    let original = unsafe { Rc::from_raw(ptr) };
    let copy = original.clone();
    Rc::into_raw(original);
    Rc::into_raw(copy)
}

extern "C" fn copy_if_string(data_ptr: *mut c_void, tag: i8, ptr: *mut String) -> *const String {
    if tag == STRING_TAG {
        copy_string(data_ptr, ptr)
    } else {
        ptr
    }
}

extern "C" fn print_error(_data: *mut std::os::raw::c_void, code: ErrorCode) {
    eprintln!("error {:?}", code)
}

extern "C" fn array_assign(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    key_tag: i8,
    key_num: f64,
    key_ptr: *mut String,
    tag: i8,
    float: f64,
    ptr: *mut String,
) {
    let data = cast_to_runtime_data(data_ptr);
    let res = data
        .arrays
        .assign(array, (key_tag, key_num, key_ptr), (tag, float, ptr));
    match res {
        None => {}
        Some((existing_tag, _existing_float, existing_ptr)) => {
            if existing_tag == STRING_TAG {
                unsafe { Rc::from_raw(existing_ptr) };
                // implicitly drop RC here. Do not report as a string_in our out since it was
                // already stored in the runtime and droped from the runtime.
            }
        }
    }
    if key_tag == STRING_TAG {
        let _rc = unsafe { Rc::from_raw(key_ptr) };
        // implicitly drop here
    };
    if tag == STRING_TAG {
        let val = unsafe { Rc::from_raw(ptr) };
        // We don't drop it here because it is now stored in the hashmap.
        Rc::into_raw(val);
    }
}

extern "C" fn array_access(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    in_tag: i8,
    in_float: f64,
    in_ptr: *mut String,
    out_tag: *mut i8,
    out_float: *mut f64,
    out_value: *mut *mut String,
) {
    let data = cast_to_runtime_data(data_ptr);
    match data.arrays.access(array, (in_tag, in_float, in_ptr)) {
        None => unsafe {
            *out_tag = STRING_TAG;
            *out_value = empty_string(data_ptr) as *mut String;
        },
        Some((tag, float, str)) => unsafe {
            *out_tag = *tag;
            *out_float = *float;
            if *tag == STRING_TAG {
                let rc = Rc::from_raw(*str);
                let cloned = rc.clone();
                Rc::into_raw(rc);
                *out_value = Rc::into_raw(cloned) as *mut String;
            }
        },
    }
    if in_tag == STRING_TAG {
        free_string(data_ptr, in_ptr);
    }
}

extern "C" fn in_array(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    in_tag: i8,
    in_float: f64,
    in_ptr: *const String,
) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let res = data.arrays.in_array(array, (in_tag, in_float, in_ptr));
    if in_tag == STRING_TAG {
        unsafe { Rc::from_raw(in_ptr) };
    }
    if res {
        1.0
    } else {
        0.0
    }
}

extern "C" fn concat_array_indices(
    _data: *mut c_void,
    left: *const String,
    right: *const String,
) -> *const String {
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };

    let mut lhs: String = match Rc::try_unwrap(lhs) {
        Ok(str) => str,
        Err(rc) => (*rc).clone(),
    };
    lhs.push_str("-");
    lhs.push_str(&rhs);
    Rc::into_raw(Rc::new(lhs))
}

extern "C" fn printf(data: *mut c_void, fstring: *mut String, nargs: i32, args: *mut c_void) {
    // let mut args = vec![];
    let data = cast_to_runtime_data(data);
    let base_ptr = args as *mut f64;
    unsafe {
        let fstring = Rc::from_raw(fstring);
        data.stdout
            .write_all(fstring.as_bytes())
            .expect("to be able to write to stdout");
        for i in 0..(nargs as isize) {
            // let tag = *(base_ptr.offset(i * 3) as *const i8);
            // let float = *(base_ptr.offset(i * 3 + 1) as *const f64);
            let ptr = *(base_ptr.offset(i * 3 + 2) as *const *mut String);
            // args.push((tag, float, ptr));
            let str = Rc::from_raw(ptr);
            print!("{}", str);
        }
        // Rc::from_raw(fstring)
    };
}

extern "C" fn to_lower(_data_ptr: *mut c_void, ptr: *const String) -> *const String {
    let ptr = unsafe { Rc::from_raw(ptr) };
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => unsafe {
            if str.is_ascii() {
                let bytes = str.as_bytes_mut();
                bytes.make_ascii_lowercase();
                Rc::into_raw(Rc::new(str))
            } else {
                let lower = Rc::new(str.to_lowercase());
                Rc::into_raw(lower)
            }
        },
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_lowercase())),
    };
    str
}

extern "C" fn to_upper(_data_ptr: *mut c_void, ptr: *const String) -> *const String {
    let ptr = unsafe { Rc::from_raw(ptr) };
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => unsafe {
            if str.is_ascii() {
                let bytes = str.as_bytes_mut();
                bytes.make_ascii_uppercase();
                Rc::into_raw(Rc::new(str))
            } else {
                let upper = Rc::new(str.to_uppercase());
                Rc::into_raw(upper)
            }
        },
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_uppercase())),
    };
    str
}

extern "C" fn srand(data_ptr: *mut c_void, seed: f64) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let prior = data.srand_seed;
    let seed_int = seed as std::os::raw::c_uint;
    unsafe { libc::srand(seed_int) }
    data.srand_seed = seed;
    prior
}
extern "C" fn rand(_data_ptr: *mut c_void) -> f64 {
    let rand = unsafe { libc::rand() } as f64;
    // float [0, 1)
    rand / libc::RAND_MAX as f64
}

pub struct ReleaseRuntime {
    runtime_data: *mut RuntimeData,
}

impl Drop for ReleaseRuntime {
    fn drop(&mut self) {
        unsafe {
            (*self.runtime_data)
                .stdout
                .flush()
                .expect("could not flush stdout");
        }
    }
}

// Pointer to this is passed in with every call. The reason we require it for every call instead of making it
// a rust global is so we can easily run tests fully independently of each other.
pub struct RuntimeData {
    srand_seed: f64,
    columns: Columns,
    stdout: BufWriter<StdoutLock<'static>>,
    regex_cache: LruCache<String, Regex>,
    arrays: Arrays,
    float_parser: FloatParser,
}

impl RuntimeData {
    pub fn new(files: Vec<String>) -> RuntimeData {
        unsafe { libc::srand(091998) }
        RuntimeData {
            srand_seed: 091998.0,
            columns: Columns::new(files),
            stdout: BufWriter::new(std::io::stdout().lock()),
            regex_cache: LruCache::new(8),
            arrays: Arrays::new(),
            float_parser: FloatParser::new(),
        }
    }
}

impl ReleaseRuntime {
    fn data_ptr(&mut self, func: &mut Function) -> Value {
        func.create_void_ptr_constant(self.runtime_data as *mut c_void)
    }
}

impl Runtime for ReleaseRuntime {
    fn new(_context: &Context, files: Vec<String>) -> ReleaseRuntime {
        let data = Box::new(RuntimeData::new(files));
        let ptr = Box::leak(data);
        let ptr = ptr as *mut RuntimeData;
        ReleaseRuntime { runtime_data: ptr }
    }

    fn allocate_arrays(&mut self, count: usize) {
        unsafe { (*self.runtime_data).arrays.allocate(count) }
    }

    fn init_empty_string(&mut self) -> *const String {
        empty_string(self.runtime_data as *mut c_void)
    }

    fn binop(&mut self, func: &mut Function, ptr1: Value, ptr2: Value, binop_val: BinOp) -> Value {
        let binop_val = func.create_sbyte_constant(binop_val as i8);
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            binop as *mut c_void,
            vec![data_ptr, ptr1, ptr2, binop_val],
            Some(Context::float64_type()),
            Abi::Cdecl,
        )
    }

    fn print_error(&mut self, func: &mut Function, error: ErrorCode) {
        let binop = func.create_sbyte_constant(error as i8);
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            print_error as *mut c_void,
            vec![data_ptr, binop],
            None,
            Abi::Cdecl,
        );
    }

    fn printf(&mut self, func: &mut Function, fstring: Value, nargs: Value, args: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            printf as *mut c_void,
            vec![data_ptr, fstring, nargs, args],
            None,
            Abi::VarArg,
        );
    }

    runtime_fn!(call_next_line, next_line, Some(Context::float64_type()),);
    runtime_fn!(column,column,Some(Context::void_ptr_type()),tag: Value,float: Value,pointer: Value);
    runtime_fn!(string_to_number,string_to_number,Some(Context::float64_type()),arg0: Value);
    runtime_fn!(number_to_string,number_to_string,Some(Context::void_ptr_type()),arg0: Value);
    runtime_fn_no_ret!(print_string, print_string, None, arg0: Value);
    runtime_fn_no_ret!(print_float, print_float, None, arg0: Value);
    runtime_fn!(concat,concat,Some(Context::void_ptr_type()),arg0: Value,arg1: Value);
    runtime_fn!(concat_array_indices,concat_array_indices,Some(Context::void_ptr_type()),arg0: Value,arg1: Value);
    runtime_fn!(empty_string, empty_string, Some(Context::void_ptr_type()),);
    runtime_fn_no_ret!(array_access,array_access,None,array: Value,key_tag: Value,key_num: Value,key_ptr: Value,out_tag_ptr: Value,out_float_ptr: Value,out_ptr_ptr: Value);
    runtime_fn_no_ret!(array_assign,array_assign,None,array: Value,key_tag: Value,key_num: Value,key_ptr: Value,tag: Value,float: Value,ptr: Value);
    runtime_fn!(in_array,in_array,Some(Context::float64_type()),array: Value,key_tag: Value,key_num: Value,key_ptr: Value);
    runtime_fn!(to_upper,to_upper,Some(Context::void_ptr_type()),ptr: Value);
    runtime_fn!(to_lower,to_lower,Some(Context::void_ptr_type()),ptr: Value);
    runtime_fn!(rand, rand, Some(Context::float64_type()),);
    runtime_fn!(srand, srand, Some(Context::float64_type()), seed: Value);

    fn free_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType) {
        let data_ptr = self.data_ptr(func);
        match typ {
            ScalarType::String => {
                func.insn_call_native(
                    free_string as *mut c_void,
                    &[data_ptr, value.pointer],
                    None,
                    Abi::Cdecl,
                );
            }
            ScalarType::Float => {}
            ScalarType::Variable => {
                func.insn_call_native(
                    free_if_string as *mut c_void,
                    &[data_ptr, value.tag, value.pointer],
                    None,
                    Abi::Cdecl,
                );
            }
        };
    }
    fn copy_if_string(&mut self, func: &mut Function, value: ValueT, typ: ScalarType) -> ValueT {
        let data_ptr = self.data_ptr(func);
        let ptr = match typ {
            ScalarType::String => func.insn_call_native(
                copy_string as *mut c_void,
                &[data_ptr, value.pointer],
                Some(Context::void_ptr_type()),
                Abi::Cdecl,
            ),
            ScalarType::Float => value.pointer,
            ScalarType::Variable => func.insn_call_native(
                copy_if_string as *mut c_void,
                &[data_ptr, value.tag.clone(), value.pointer],
                Some(Context::void_ptr_type()),
                Abi::Cdecl,
            ),
        };
        ValueT::new(value.tag, value.float, ptr)
    }

    fn pointer_to_name_mapping(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

fn cast_to_runtime_data(data: *mut c_void) -> &'static mut RuntimeData {
    unsafe {
        let data = data as *mut RuntimeData;
        &mut *data
    }
}
