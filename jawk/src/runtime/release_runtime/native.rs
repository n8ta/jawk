use std::io::Write;
use std::os::raw::c_void;
use std::rc::Rc;
use mawk_regex::Regex;
use crate::codegen::{FLOAT_TAG, STRING_TAG};
use crate::lexer::BinOp;
use crate::runtime::ErrorCode;
use crate::runtime::float_parser::string_to_float;
use crate::runtime::release_runtime::cast_to_runtime_data;

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

pub extern "C" fn next_line(data: *mut c_void) -> f64 {
    let data = cast_to_runtime_data(data);
    // TODO: remove unwrap
    if data.columns.next_line().unwrap() {
        1.0
    } else {
        0.0
    }
}

pub extern "C" fn column(
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
    let str =
        if let Some( mut current) = data.fast_alloc.take() {
            {
                let mutable = match Rc::get_mut(&mut current) {
                    None => panic!("rc should be unique"),
                    Some(some) => some,
                };
                let mut buf = unsafe { mutable.as_mut_vec() };
                data.columns.get_into_buf(idx, buf);
            }
            current
        } else {
            Rc::new(data.columns.get(idx))
        };
    Rc::into_raw(str) as *mut String
}

pub extern "C" fn free_string(data_ptr: *mut c_void, string: *const String) {
    let data = cast_to_runtime_data(data_ptr);
    let str = unsafe { Rc::from_raw(string) };
    if Rc::strong_count(&str) == 1 && Rc::weak_count(&str) == 0 {
        data.fast_alloc = Some(str);
    }
}

pub extern "C" fn free_if_string(data_ptr: *mut c_void, tag: i8, string: *const String) {
    if tag == STRING_TAG {
        free_string(data_ptr, string);
        // unsafe { Rc::from_raw(string) };
    }
}

pub extern "C" fn concat(
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

pub extern "C" fn empty_string(_data_ptr: *mut c_void) -> *const String {
    Rc::into_raw(Rc::new("".to_string()))
}

pub extern "C" fn binop(
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

pub extern "C" fn string_to_number(_data: *mut c_void, ptr: *const String) -> f64 {
    let string = unsafe { Rc::from_raw(ptr) };
    let res = string_to_float(&*string);
    Rc::into_raw(string);
    res
}

pub extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const String {
    let runtime_data = cast_to_runtime_data(data_ptr);
    Rc::into_raw(Rc::new(runtime_data.float_parser.parse(value)))
}

pub extern "C" fn copy_string(_data_ptr: *mut c_void, ptr: *mut String) -> *const String {
    let original = unsafe { Rc::from_raw(ptr) };
    let copy = original.clone();
    Rc::into_raw(original);
    Rc::into_raw(copy)
}

pub extern "C" fn copy_if_string(data_ptr: *mut c_void, tag: i8, ptr: *mut String) -> *const String {
    if tag == STRING_TAG {
        copy_string(data_ptr, ptr)
    } else {
        ptr
    }
}

pub extern "C" fn print_error(_data: *mut std::os::raw::c_void, code: ErrorCode) {
    eprintln!("error {:?}", code)
}

pub extern "C" fn array_assign(
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

pub extern "C" fn array_access(
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

pub extern "C" fn in_array(
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

pub extern "C" fn concat_array_indices(
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

pub extern "C" fn printf(data: *mut c_void, fstring: *mut String, nargs: i32, args: *mut c_void) {
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

pub extern "C" fn to_lower(_data_ptr: *mut c_void, ptr: *const String) -> *const String {
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

pub extern "C" fn to_upper(_data_ptr: *mut c_void, ptr: *const String) -> *const String {
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

pub extern "C" fn split(string: *const String, array: i32) {
    todo!()
}

pub extern "C" fn split_ere(string: *const String, array: i32, ere_split: *const String) {
    todo!()
}

pub extern "C" fn srand(data_ptr: *mut c_void, seed: f64) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let prior = data.srand_seed;
    let seed_int = (seed % (std::os::raw::c_uint::MAX as f64)) as std::os::raw::c_uint;
    unsafe { libc::srand(seed_int) }
    data.srand_seed = seed;
    prior
}

pub extern "C" fn rand(_data_ptr: *mut c_void) -> f64 {
    let rand = unsafe { libc::rand() } as f64;
    // float [0, 1)
    rand / libc::RAND_MAX as f64
}

pub extern "C" fn length(_data_ptr: *mut c_void, str: *const String) -> f64 {
    let str = unsafe { Rc::from_raw(str) };
    str.chars().count() as f64
    // Drop str
}