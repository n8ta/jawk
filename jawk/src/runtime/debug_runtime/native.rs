use std::cmp::{max, min};
use std::io::{stdout, Write};
use std::os::raw::c_void;
use std::rc::Rc;
use mawk_regex::Regex;
use crate::awk_str::AwkStr;
use crate::codegen::{Tag};
use crate::lexer::BinOp;
use crate::runtime::array_split::{split_on_regex, split_on_string};
use crate::runtime::call_log::Call;
use crate::runtime::debug_runtime::{cast_to_runtime_data, RuntimeData};
use crate::runtime::ErrorCode;
use crate::runtime::util::{clamp_to_max_len, clamp_to_slice_index};
use crate::runtime::value::RuntimeValue;

pub extern "C" fn print_string(data: *mut c_void, value: *mut AwkStr) {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::PrintString);
    let str = unsafe { Rc::from_raw(value) };

    let res = if str.bytes().ends_with(&[10]) {
        str.bytes().to_vec()
    } else {
        let mut bytes = str.bytes().to_vec();
        bytes.push(10);
        bytes
    };
    data.output.extend_from_slice(&res);
    stdout().write_all(&res).unwrap();
    data.str_tracker.string_in("print_string", &*str)
}

pub extern "C" fn print_float(data: *mut c_void, value: f64) {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::PrintFloat);
    let mut res = value.to_string();
    res.push_str("\n");
    data.output.extend_from_slice(res.as_bytes());
    println!("{}", value);
}

pub extern "C" fn next_line(data: *mut c_void) -> f64 {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::NextLine);

    // TODO: remove unwrap handle the error
    if data.columns.next_line().unwrap() {
        1.0
    } else {
        0.0
    }
}

pub extern "C" fn split(data_ptr: *mut c_void, string: *const AwkStr, array: i32) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Split);
    let rc = unsafe { Rc::from_raw(string) };
    data.str_tracker.string_in("split_ere string", &rc);
    let mut count: f64 = 0.0;
    let _ = data.arrays.clear(array);
    for (idx, elem) in split_on_string(data.columns.get_field_sep(), &rc).enumerate()
    {
        count += 1.0;
        let string = Rc::into_raw(Rc::new(AwkStr::new(elem.to_vec())));
        let _ = data.arrays.assign(array,
                                   Rc::new(AwkStr::new(format!("{}", idx + 1).into_bytes())),
                                   RuntimeValue::new(Tag::StrnumTag, 0.0, string));
    }
    count
}

pub extern "C" fn split_ere(data_ptr: *mut c_void, string: *const AwkStr, array: i32, ere_split: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::SplitEre);
    let str = unsafe { Rc::from_raw(string) };
    let reg_str = unsafe { Rc::from_raw(ere_split) };
    data.str_tracker.string_in("split_ere string", &str);
    data.str_tracker.string_in("split_ere regex", &reg_str);
    let reg = Regex::new(&reg_str);
    let mut count: f64 = 0.0;
    let _ = data.arrays.clear(array);
    for (idx, elem) in split_on_regex(&reg, &str).enumerate()
    {
        count += 1.0;
        let string = Rc::into_raw(Rc::new(AwkStr::new(elem.to_vec())));
        let _ = data.arrays.assign(array,
                                   Rc::new(AwkStr::new(format!("{}", idx + 1).into_bytes())),
                                   RuntimeValue::new(Tag::StrnumTag, 0.0, string));
    }
    count
}

pub extern "C" fn substr(data_ptr: *mut c_void, string_ptr: *const AwkStr, start_idx: f64) -> *const AwkStr {
    // TODO: utf-8 support for start_idx
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Substr);
    let string = unsafe { Rc::from_raw(string_ptr) };
    data.str_tracker.string_in("substr string", &string);
    let start_idx = clamp_to_slice_index(start_idx-1.0, string.bytes().len());
    let output = Rc::new(AwkStr::new(string.bytes()[start_idx..].to_vec()));
    data.str_tracker.string_out("substr out", &*output);
    Rc::into_raw(output)
}

pub extern "C" fn substr_max_chars(data_ptr: *mut c_void, string_ptr: *const AwkStr, start_idx: f64, max_chars: f64) -> *const AwkStr {
    // TODO: utf-8 support for start_idx and max_chars
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::SubstrMaxChars);
    let string = unsafe { Rc::from_raw(string_ptr) };
    data.str_tracker.string_in("substr_max_chars string", &string);

    let str_len = string.bytes().len();
    let start_idx = clamp_to_slice_index(start_idx-1.0, str_len);
    let max_chars = clamp_to_max_len(max_chars, start_idx, str_len);
    let output = Rc::new(AwkStr::new(string.bytes()[start_idx..start_idx+max_chars].to_vec()));
    data.str_tracker.string_out("substr_max_chars out", &*output);
    Rc::into_raw(output)
}

pub extern "C" fn srand(data_ptr: *mut c_void, seed: f64) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Srand);
    let prior = data.srand_seed;
    let seed_int = (seed % (std::os::raw::c_uint::MAX as f64)) as std::os::raw::c_uint;
    unsafe { libc::srand(seed_int) }
    data.srand_seed = seed;
    prior
}

pub extern "C" fn rand(data_ptr: *mut c_void) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Rand);
    let rand = unsafe { libc::rand() } as f64;
    // float [0, 1)
    rand / libc::RAND_MAX as f64
}

pub extern "C" fn length(data_ptr: *mut c_void, str: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Length);
    let str = unsafe { Rc::from_raw(str) };
    data.str_tracker.string_in("length ptr", str.bytes());
    let len = match String::from_utf8(str.bytes().to_vec()) {
        Ok(s) => s.len(),
        Err(_err) => {
            eprintln!("String is not validate utf-8 falling back to length in bytes");
            str.bytes().len()
        }
    };
    len as f64
}

pub extern "C" fn to_lower(data_ptr: *mut c_void, ptr: *const AwkStr) -> *const AwkStr {
    let ptr = unsafe { Rc::from_raw(ptr) };
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ToLower);
    data.str_tracker.string_in("to_lower", ptr.bytes());
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => {
            // TODO: non-ascii lower case
            str.make_ascii_lowercase();
            Rc::into_raw(Rc::new(str))
        }
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_ascii_lowercase())),
    };
    let str = unsafe { Rc::from_raw(str) };
    data.str_tracker.string_out("to_lower", str.bytes());
    Rc::into_raw(str)
}

pub extern "C" fn to_upper(data_ptr: *mut c_void, ptr: *const AwkStr) -> *const AwkStr {
    let ptr = unsafe { Rc::from_raw(ptr) };
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ToUpper);
    data.str_tracker.string_in("to_lower", ptr.bytes());
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => {
            // TODO: non-ascii lower case
            str.make_ascii_uppercase();
            Rc::into_raw(Rc::new(str))
        }
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_ascii_uppercase())),
    };
    let str = unsafe { Rc::from_raw(str) };
    data.str_tracker.string_out("to_upper", str.bytes());
    Rc::into_raw(str)
}

pub extern "C" fn column(
    data_ptr: *mut c_void,
    tag: Tag,
    value: f64,
    pointer: *const AwkStr,
) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    let idx = match tag {
        Tag::FloatTag => value,
        Tag::StringTag | Tag::StrnumTag => string_to_number(data_ptr, pointer),
    };
    let idx = idx.round() as usize;
    let str = data.columns.get(idx);
    data.calls.log(Call::Column(idx as f64));
    println!("\tgetting column tag:{} float:{} ptr:{:?} GOT: `{}`", tag, value, pointer, String::from_utf8(str.bytes().to_vec()).unwrap());
    data.str_tracker.string_out("column", str.bytes());
    Rc::into_raw(Rc::new(str))
}

pub extern "C" fn free_string(data_ptr: *mut c_void, ptr: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::FreeString);
    println!("\tfreeing ptr {:?}", ptr);

    let string_data = unsafe { Rc::from_raw(ptr) };
    data.str_tracker.string_in("free_string", string_data.bytes());
    if Rc::strong_count(&string_data) > 1000 {
        // This isn't truly an error condition but if we use-after-free a string this is often hit
        // and is great for catching uaf early.
        panic!("count is very large! {}", Rc::strong_count(&string_data));
    }
    print!("\tstring is: '");
    stdout().write_all(string_data.bytes()).unwrap();
    println!("' count is now: {}", Rc::strong_count(&string_data).saturating_sub(1));
    0.0
}

pub extern "C" fn free_if_string(data_ptr: *mut c_void, tag: Tag, string: *const AwkStr) {
    if tag.has_ptr() {
        free_string(data_ptr, string);
    }
}

pub extern "C" fn concat(
    data: *mut c_void,
    left: *const AwkStr,
    right: *const AwkStr,
) -> *const AwkStr {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::Concat);
    println!("\t{:?}, {:?}", left, right);
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };

    let mut lhs: AwkStr = match Rc::try_unwrap(lhs) {
        Ok(str) => {
            print!("\tDowngraded RC into box for: ");
            stdout().write_all(&str.bytes()).unwrap();
            println!();
            str
        }
        Err(rc) => (*rc).clone(),
    };
    data.str_tracker.string_in("concat lhs", &*lhs);
    data.str_tracker.string_in("concat rhs", &*rhs);

    lhs.push_str(&rhs);
    println!("\tResult: ");
    stdout().write_all(&*lhs).unwrap();
    println!();
    data.str_tracker.string_out("concat result", &*lhs);
    Rc::into_raw(Rc::new(lhs))
}

pub extern "C" fn empty_string(data: *mut c_void) -> *const AwkStr {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::EmptyString);
    let rc = Rc::new(AwkStr::new(vec![]));
    data.str_tracker.string_out("empty_string", rc.bytes());
    let ptr = Rc::into_raw(rc);
    println!("\tempty string is {:?}", ptr);
    ptr
}

pub extern "C" fn string_to_number(data_ptr: *mut c_void, ptr: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::StringToNumber);

    let string = unsafe { Rc::from_raw(ptr) };
    print!("\tstring_to_number '");
    stdout().write_all(string.bytes()).unwrap();
    println!("'{:?}", ptr);
    let res = data.converter.str_to_num(&*string).unwrap_or(0.0);
    Rc::into_raw(string);
    println!("\tret {}", res);
    res
}

pub extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::NumberToString);
    println!("\tnum: {}", value);
    let str = AwkStr::new(data.converter.num_to_str_internal(value).to_vec());
    let heap_alloc_string = Rc::new(str);
    data.str_tracker.string_out("number_to_string", &*heap_alloc_string);
    let ptr = Rc::into_raw(heap_alloc_string);
    ptr
}

pub extern "C" fn copy_string(data_ptr: *mut c_void, ptr: *mut AwkStr) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::CopyString);

    let original = unsafe { Rc::from_raw(ptr) };
    data.str_tracker.string_out("copy_string", &*original);
    print!("\tCopying string {:?} '", ptr);
    stdout().write_all(original.bytes()).unwrap();
    println!("' count is {}", Rc::strong_count(&original));
    let copy = original.clone();
    Rc::into_raw(original);

    println!("\tNew count is {}", Rc::strong_count(&copy));
    let copy = Rc::into_raw(copy);
    println!("\tCopy is: {:?}", copy);
    copy
}

pub extern "C" fn copy_if_string(_data: *mut c_void, tag: Tag, ptr: *mut AwkStr) -> *const AwkStr {
    if tag.has_ptr() {
        copy_string(_data, ptr)
    } else {
        ptr
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
            let str = AwkStr::new(data.converter.num_to_str_internal(f).to_vec());
            Rc::new(str)
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
    let left = data.str_tracker.value_from_ffi(l_tag, l_flt, l_ptr, "binop-left");
    let right = data.str_tracker.value_from_ffi(r_tag, r_flt, r_ptr, "binop-right");
    data.calls.log(Call::BinOp);
    println!("\tBinop called {:?} {} {:?}", &left, binop, &right);


    let res =
        if left.is_numeric(&mut data.converter) && right.is_numeric(&mut data.converter) && binop != BinOp::MatchedBy && binop != BinOp::NotMatchedBy {
            // to_number drops the string ptr if it's a strnum
            let left = to_number(data, left);
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
                    let regex = Regex::new(&right);
                    regex.matches(&left)
                }
                BinOp::NotMatchedBy => {
                    let regex = Regex::new(&right);
                    !regex.matches(&left)
                }
            }
        };
    let res = if res { 1.0 } else { 0.0 };
    print!("\tBinop called: '");
    res
}

pub extern "C" fn print_error(_data_ptr: *mut std::os::raw::c_void, code: ErrorCode) {
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
    data.calls.log(Call::ArrayAssign);
    let key = data.str_tracker.value_from_ffi(key_tag, key_num, key_ptr, "array_assign_key");
    let key = to_string(data, key);
    let value = data.str_tracker.value_from_ffi(tag, float, ptr, "array_assign_value");
    let _ = data.arrays.assign(array, key, value);
}

pub extern "C" fn array_access(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    in_tag: Tag,
    in_float: f64,
    in_ptr: *const AwkStr,
    out_tag: *mut Tag,
    out_float: *mut f64,
    out_value: *mut *const AwkStr,
) {
    let data = cast_to_runtime_data(data_ptr);
    let idx = data.str_tracker.value_from_ffi(in_tag, in_float, in_ptr, "array_access_idx");
    let idx = to_string(data, idx);
    println!("\tarray access for key tag:{} flt:{} ptr:{:?}", in_tag, in_float, in_ptr);
    data.calls.log(Call::ArrayAccess);
    match data.arrays.access(array, idx) {
        None => unsafe {
            println!("\tarray access for non-existent key");
            *out_tag = Tag::StringTag;
            *out_value = empty_string(data_ptr) as *mut AwkStr;
        },
        Some(value) => unsafe {
            let cloned = data.str_tracker.clone_to_ffi(value, "array_access_value");
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
    data.calls.log(Call::InArray);
    let idx = data.str_tracker.value_from_ffi(in_tag, in_float, in_ptr, "in_array");
    let idx = to_string(data, idx);
    if data.arrays.in_array(array, idx) {
        1.0
    } else {
        0.0
    }
}

pub extern "C" fn concat_array_indices(
    data: *mut c_void,
    left: *const AwkStr,
    right: *const AwkStr,
) -> *const AwkStr {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::ConcatArrayIndices);
    println!("\t{:?}, {:?}", left, right);
    let lhs = data.str_tracker.string_from_ffi(left, "concat-indices lhs");
    let rhs = data.str_tracker.string_from_ffi(right, "concat-indices rhs");

    let mut lhs = match Rc::try_unwrap(lhs) {
        Ok(str) => {
            print!("\tDowngraded RC into box for string ");
            stdout().write_all(str.bytes()).unwrap();
            println!();
            str
        }
        Err(rc) => (*rc).clone(),
    };

    lhs.push_str("-".as_bytes());
    lhs.push_str(&rhs);
    data.str_tracker.string_out("concat indices result", &lhs);
    let res = Rc::into_raw(Rc::new(lhs));
    println!("\treturning {:?}", res);
    res
}

pub extern "C" fn printf(data: *mut c_void, fstring: *mut AwkStr, nargs: i32, args: *mut c_void) {
    let data = cast_to_runtime_data(data);
    // let mut args = vec![];
    let base_ptr = args as *mut f64;
    unsafe {
        let fstring = Rc::from_raw(fstring);
        data.str_tracker.string_in("printf fstring", &*fstring);
        data.output.extend_from_slice(&*fstring);
        stdout().write_all(fstring.bytes()).unwrap();
        for i in 0..(nargs as isize) {
            let _tag = *(base_ptr.offset(i * 3) as *const i8);
            let _float = *(base_ptr.offset(i * 3 + 1) as *const f64);
            let ptr = *(base_ptr.offset(i * 3 + 2) as *const *mut AwkStr);
            // args.push((tag, float, ptr));
            let str = Rc::from_raw(ptr);
            data.output.extend_from_slice(&*str);
            data.str_tracker.string_in("printf in arg", &*str);
            stdout().write_all(str.bytes()).unwrap();
        }
        // Rc::from_raw(fstring)
    };
}