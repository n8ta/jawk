use std::io::Write;
use std::os::raw::c_void;
use std::rc::Rc;
use lru_cache::LruCache;
use mawk_regex::Regex;
use crate::awk_str::{AwkStr, unwrap_awkstr_rc};
use crate::codegen::{Tag};
use crate::lexer::BinOp;
use crate::runtime::array_split::{split_on_regex, split_on_string};
use crate::runtime::ErrorCode;
use crate::runtime::release_runtime::{cast_to_runtime_data, RuntimeData};
use crate::runtime::util::{clamp_to_max_len, clamp_to_slice_index};
use crate::runtime::value::RuntimeValue;
use crate::util::index_of;

pub extern "C" fn print_string(data: *mut c_void, value: *mut AwkStr) {
    let data = cast_to_runtime_data(data);
    let str = unsafe { Rc::from_raw(value) };
    if str.bytes().ends_with(&[10]) {
        data.stdout.write_all(&str).expect("failed to write to stdout")
    } else {
        data.stdout.write_all(&str).expect("failed to write to stdout");
        data.stdout.write_all(&[10]).expect("failed to write to stdout");
    }
    // implicitly consuming str here.
}

pub extern "C" fn print_float(data: *mut c_void, value: f64) {
    let data = cast_to_runtime_data(data);
    let bytes = data.converter.num_to_str_output(value);
    data.stdout.write_all(bytes).unwrap();
    data.stdout.write_all("\n".as_bytes()).unwrap();
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
    tag: Tag,
    value: f64,
    pointer: *const AwkStr,
) -> *mut AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    let idx = match tag {
        Tag::FloatTag => value,
        Tag::StringTag | Tag::StrnumTag => string_to_number(data_ptr, pointer),
    };
    let idx = idx.round() as usize;
    let str =
        if let Some( mut current) = data.hacky_alloc.take() {
            {
                let mutable = match Rc::get_mut(&mut current) {
                    None => panic!("rc should be unique"),
                    Some(some) => some,
                };
                let buf = mutable.as_mut_vec();
                data.columns.get_into_buf(idx, buf);
            }
            current
        } else {
            Rc::new(data.columns.get(idx))
        };
    Rc::into_raw(str) as *mut AwkStr
}

pub extern "C" fn free_string(data_ptr: *mut c_void, string: *const AwkStr) {
    let data = cast_to_runtime_data(data_ptr);
    let str = unsafe { Rc::from_raw(string) };
    data.hacky_alloc.drop(str);
}

pub extern "C" fn free_if_string(data_ptr: *mut c_void, tag: Tag, string: *const AwkStr) {
    if tag.has_ptr()  {
        free_string(data_ptr, string);
    }
}

pub extern "C" fn sub(data_ptr: *mut c_void,
                      ere: *const AwkStr,
                      repl: *const AwkStr,
                      input_str: *const AwkStr,
                      is_global: i32,
                      out_float_ptr: *mut f64) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    let is_global = is_global == 1;
    let (ere, repl, input_str) = unsafe { (Rc::from_raw(ere), Rc::from_raw(repl), Rc::from_raw(input_str)) };

    let regex = get_from_regex_cache(&mut data.regex_cache, ere);
    let input_str = unwrap_awkstr_rc(input_str);

    let (num_substitutions, result_str) = if is_global {
        todo!()
    } else {
        let matched = regex.match_idx(&*input_str);
        if let Some(mtc) = matched {
            let input_bytes = input_str.bytes();
            let mut new_string = AwkStr::new((&input_bytes[0..mtc.start]).to_vec());
            new_string.push_str(repl.bytes());
            new_string.push_str(&input_bytes[mtc.start + mtc.len..]);
            (1.0, new_string)
        } else {
            (0.0, input_str)
        }
    };
    unsafe { *out_float_ptr = num_substitutions; };
    Rc::into_raw(Rc::new(result_str))
}
pub extern "C" fn concat(
    data_ptr: *mut c_void,
    left: *const AwkStr,
    right: *const AwkStr,
) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };
    let mut lhs = unwrap_awkstr_rc(lhs);
    lhs.push_str(&rhs);
    data.hacky_alloc.drop(rhs);
    Rc::into_raw(Rc::new(lhs))
}

pub extern "C" fn empty_string(data_ptr: *mut c_void) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    Rc::into_raw(data.hacky_alloc.alloc_awkstr(&[]))
}

pub fn get_from_regex_cache(regex_cache: &mut LruCache<AwkStr, Regex>, reg_str: Rc<AwkStr>) -> &mut Regex {
    if regex_cache.contains_key(&*reg_str) {
        regex_cache.get_mut(&*reg_str).unwrap()
    }  else {
        let re = Regex::new(&reg_str);
        regex_cache.insert((&*reg_str).clone(), re);
        regex_cache.get_mut(&*reg_str).unwrap()
    }
}

fn to_number(data: &mut RuntimeData, value: RuntimeValue) -> Option<f64> {
    match value {
        RuntimeValue::Float(f) => Some(f),
        RuntimeValue::Str(ptr) => {
            data.converter.str_to_num(&*ptr)
        }
        RuntimeValue::StrNum(ptr) => {
            data.converter.str_to_num(&*ptr)
        }
    }
}

fn to_string(data: &mut RuntimeData, value: RuntimeValue) -> Rc<AwkStr> {
    match value {
        RuntimeValue::Float(f) => {
            let bytes = data.converter.num_to_str_internal(f);
            data.hacky_alloc.alloc_awkstr(bytes)
        }
        RuntimeValue::Str(ptr) => ptr,
        RuntimeValue::StrNum(ptr) => ptr,
    }
}

pub extern "C" fn binop(
    data_ptr: *mut c_void,
    l_tag: Tag,
    l_flt: f64,
    l_ptr: *const AwkStr,
    r_tag: Tag,
    r_flt: f64,
    r_ptr: *const AwkStr,
    binop: BinOp,
) -> std::os::raw::c_double {
    let data = cast_to_runtime_data(data_ptr);
    let left = RuntimeValue::new(l_tag, l_flt, l_ptr);
    let right = RuntimeValue::new(r_tag, r_flt, r_ptr);
    let res =

        if left.is_numeric(&mut data.converter) && right.is_numeric(&mut data.converter) && binop != BinOp::MatchedBy && binop != BinOp::NotMatchedBy {
            // to_number drops the string ptr if it's a strnum
            let left = to_number(data,left);
            let right = to_number(data, right);
            match binop {
                BinOp::Greater => left > right,
                BinOp::GreaterEq => left >= right,
                BinOp::Less => left < right,
                BinOp::LessEq => left < right,
                BinOp::BangEq => left != right,
                BinOp::EqEq => left == right,
                _ => unreachable!(),
            }
        } else {
            // String comparisons
            let left = to_string(data, left);
            let right = to_string(data, right);
            match binop {
                BinOp::Greater => left > right,
                BinOp::GreaterEq => left >= right,
                BinOp::Less => left < right,
                BinOp::LessEq => left <= right,
                BinOp::BangEq => left != right,
                BinOp::EqEq => left == right,
                BinOp::MatchedBy => {
                    let reg = get_from_regex_cache(&mut data.regex_cache, right);
                    reg.matches(&left)
                }
                BinOp::NotMatchedBy => {
                    let reg = get_from_regex_cache(&mut data.regex_cache, right);
                    !reg.matches(&left)
                }
            }
        };
    let res = if res { 1.0 } else { 0.0 };
    res
}

pub extern "C" fn string_to_number(data_ptr: *mut c_void, ptr: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let string = unsafe { Rc::from_raw(ptr) };
    let res = data.converter.str_to_num(&*string).unwrap_or(0.0);
    Rc::into_raw(string);
    res
}

pub extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    let str = data.converter.num_to_str_internal(value);
    Rc::into_raw(data.hacky_alloc.alloc_awkstr(str))
}

pub extern "C" fn copy_string(_data_ptr: *mut c_void, ptr: *mut AwkStr) -> *const AwkStr {
    let original = unsafe { Rc::from_raw(ptr) };
    let copy = original.clone();
    Rc::into_raw(original);
    Rc::into_raw(copy)
}

pub extern "C" fn copy_if_string(data_ptr: *mut c_void, tag: Tag, ptr: *mut AwkStr) -> *const AwkStr {
    if tag.has_ptr() {
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
    key_tag: Tag,
    key_num: f64,
    key_ptr: *mut AwkStr,
    tag: Tag,
    float: f64,
    ptr: *mut AwkStr,
) {
    let data = cast_to_runtime_data(data_ptr);
    let key = RuntimeValue::new(key_tag, key_num, key_ptr);
    let key = to_string(data, key);
    let val = RuntimeValue::new(tag, float, ptr);
    let res = data.arrays.assign(array, key, val);
    data.hacky_alloc.drop_opt_rtval(res);
}

pub extern "C" fn array_access(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    in_tag: Tag,
    in_float: f64,
    in_ptr: *mut AwkStr,
    out_tag: *mut Tag,
    out_float: *mut f64,
    out_value: *mut *const AwkStr,
) {
    let data = cast_to_runtime_data(data_ptr);
    let key = RuntimeValue::new(in_tag, in_float, in_ptr);
    let key = to_string(data, key);
    match data.arrays.access(array, key) {
        None => unsafe {
            *out_tag = Tag::StringTag;
            *out_value = empty_string(data_ptr) as *mut AwkStr;
        },
        Some(existing) => unsafe {
            let cloned = existing.clone();
            *out_tag = cloned.0;
            *out_float = cloned.1;
            *out_value = cloned.2;
        },
    }
}

pub extern "C" fn in_array(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    in_tag: Tag,
    in_float: f64,
    in_ptr: *const AwkStr,
) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let idx = RuntimeValue::new(in_tag, in_float, in_ptr);
    let idx = to_string(data, idx);
    if data.arrays.in_array(array, idx) {
        1.0
    } else {
        0.0
    }
}

pub extern "C" fn concat_array_indices(
    _data: *mut c_void,
    left: *const AwkStr,
    right: *const AwkStr,
) -> *const AwkStr {
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };

    let mut lhs = match Rc::try_unwrap(lhs) {
        Ok(str) => str,
        Err(rc) => (*rc).clone(),
    };
    lhs.push_str("-".as_bytes());
    lhs.push_str(&rhs);
    Rc::into_raw(Rc::new(lhs))
}

pub extern "C" fn printf(data: *mut c_void, fstring: *mut AwkStr, nargs: i32, args: *mut c_void) {
    // let mut args = vec![];
    let data = cast_to_runtime_data(data);
    let base_ptr = args as *mut f64;
    unsafe {
        let fstring = Rc::from_raw(fstring);
        data.stdout.write_all(fstring.bytes()).expect("to be able to write to stdout");
        for i in 0..(nargs as isize) {
            // let tag = *(base_ptr.offset(i * 3) as *const i8);
            // let float = *(base_ptr.offset(i * 3 + 1) as *const f64);
            let ptr = *(base_ptr.offset(i * 3 + 2) as *const *mut AwkStr);
            // args.push((tag, float, ptr));
            let str = Rc::from_raw(ptr);
            data.stdout.write_all(&str).expect("to be able to write to stdout");
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

pub extern "C" fn split(data_ptr: *mut c_void, string: *const AwkStr, array: i32) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let rc = unsafe { Rc::from_raw(string) };
    let mut count: f64 = 0.0;
    let _ =  data.arrays.clear(array);
    for (idx, elem) in split_on_string(data.columns.get_field_sep(), &rc).enumerate()
    {
        count += 1.0;
        let string = Rc::into_raw(Rc::new(AwkStr::new(elem.to_vec())));
        let _ = data.arrays.assign(array,
                                   Rc::new(AwkStr::new(format!("{}", idx+1).into_bytes())),
                                   RuntimeValue::new(Tag::StrnumTag, 0.0, string));
    }
    count
}

pub extern "C" fn split_ere(data_ptr: *mut c_void, string: *const AwkStr, array: i32, ere_split: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let str = unsafe { Rc::from_raw(string) };
    let reg_str = unsafe { Rc::from_raw(ere_split) };
    let reg = get_from_regex_cache(&mut data.regex_cache, reg_str);
    let mut count: f64 = 0.0;
    let _ = data.arrays.clear(array);
    for (idx, elem) in split_on_regex(&reg, &str).enumerate()
    {
        count += 1.0;
        let string = Rc::into_raw(Rc::new(AwkStr::new(elem.to_vec())));
        let _ = data.arrays.assign(array,
                                   Rc::new(AwkStr::new(format!("{}", idx+1).into_bytes())),
                                   RuntimeValue::new(Tag::StrnumTag, 0.0, string));
    }
    count
}


pub extern "C" fn substr(_data_ptr: *mut c_void, string_ptr: *const AwkStr, start_idx: f64) -> *const AwkStr {
    // TODO: utf-8 support for start_idx
    let string = unsafe { Rc::from_raw(string_ptr) };
    let start_idx = clamp_to_slice_index(start_idx-1.0, string.bytes().len());
    let output = Rc::new(AwkStr::new(string.bytes()[start_idx..].to_vec()));
    Rc::into_raw(output)
}

pub extern "C" fn substr_max_chars(data_ptr: *mut c_void, string_ptr: *const AwkStr, start_idx: f64, max_chars: f64) -> *const AwkStr {
    // TODO: utf-8 support for start_idx and max_chars
    let data = cast_to_runtime_data(data_ptr);
    let string = unsafe { Rc::from_raw(string_ptr) };
    let str_len = string.bytes().len();
    let start_idx = clamp_to_slice_index(start_idx-1.0, str_len);
    let max_chars = clamp_to_max_len(max_chars, start_idx, str_len);
    let awk_str = data.hacky_alloc.alloc_awkstr(&string.bytes()[start_idx..start_idx+max_chars]);
    Rc::into_raw(awk_str)
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

pub extern "C" fn index(_data_ptr: *mut c_void, needle: *const AwkStr, haystack: *const AwkStr) -> f64 {
    let (needle, haystack) = unsafe { (Rc::from_raw(needle), Rc::from_raw(haystack)) };
    if let Some(idx) = index_of(needle.bytes(), haystack.bytes()) {
        (idx + 1) as f64
    } else {
        0.0
    }
}