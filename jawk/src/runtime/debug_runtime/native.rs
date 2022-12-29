use std::io::{stdout, Write};
use std::os::raw::c_void;
use std::rc::Rc;
use mawk_regex::Regex;
use crate::awk_str::AwkStr;
use crate::codegen::{FLOAT_TAG, STRING_TAG};
use crate::lexer::BinOp;
use crate::runtime::array_split::{split_on_regex, split_on_string};
use crate::runtime::arrays::MapValue;
use crate::runtime::call_log::Call;
use crate::runtime::debug_runtime::{cast_to_runtime_data, RuntimeData};
use crate::runtime::ErrorCode;
use crate::runtime::float_parser::string_to_float;

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
    data.string_in("print_string", &*str)
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

    // TODO: remove unwrap
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
    data.string_in("split_ere string", &rc);
    let mut count: f64 = 0.0;
    for (_key, val) in data.arrays.clear(array) {
        if val.tag == STRING_TAG {
            unsafe { Rc::from_raw(val.ptr) };
        }
    }
    for (idx, elem) in split_on_string(data.columns.get_field_sep(), &rc).enumerate()
    {
        count += 1.0;
        let string = Rc::into_raw(Rc::new(AwkStr::new(elem.to_vec())));
        let res = data.arrays.assign(array,
                                     MapValue::new(FLOAT_TAG, (idx + 1) as f64, 0 as *const AwkStr),
                                     MapValue::new(STRING_TAG, 0.0, string));
    }
    count
}

pub extern "C" fn split_ere(data_ptr: *mut c_void, string: *const AwkStr, array: i32, ere_split: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Split);
    let str = unsafe { Rc::from_raw(string) };
    let reg_str = unsafe { Rc::from_raw(ere_split) };
    data.string_in("split_ere string", &str);
    data.string_in("split_ere regex", &reg_str);
    let reg = Regex::new(&reg_str);
    let mut count: f64 = 0.0;
    for (_key, val) in data.arrays.clear(array) {
        if val.tag == STRING_TAG {
            unsafe { Rc::from_raw(val.ptr) };
        }
    }
    for (idx, elem) in split_on_regex(&reg, &str).enumerate()
    {
        count += 1.0;
        let string = Rc::into_raw(Rc::new(AwkStr::new(elem.to_vec())));
        let _ = data.arrays.assign(array,
                                     MapValue::new(FLOAT_TAG, (idx + 1) as f64, 0 as *const AwkStr),
                                     MapValue::new(STRING_TAG, 0.0, string));
    }
    count
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
    data.string_in("length ptr", str.bytes());
    let len = match String::from_utf8(str.bytes().to_vec()) {
        Ok(s) => s.len(),
        Err(err) => {
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
    data.string_in("to_lower", ptr.bytes());
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => unsafe {
            // TODO: non-ascii lower case
            str.make_ascii_lowercase();
            Rc::into_raw(Rc::new(str))
        },
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_ascii_lowercase())),
    };
    let str = unsafe { Rc::from_raw(str) };
    data.string_out("to_lower", str.bytes());
    Rc::into_raw(str)
}

pub extern "C" fn to_upper(data_ptr: *mut c_void, ptr: *const AwkStr) -> *const AwkStr {
    let ptr = unsafe { Rc::from_raw(ptr) };
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ToUpper);
    data.string_in("to_lower", ptr.bytes());
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => unsafe {
            // TODO: non-ascii lower case
            str.make_ascii_uppercase();
            Rc::into_raw(Rc::new(str))
        },
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_ascii_uppercase())),
    };
    let str = unsafe { Rc::from_raw(str) };
    data.string_out("to_upper", str.bytes());
    Rc::into_raw(str)
}

pub extern "C" fn column(
    data_ptr: *mut c_void,
    tag: i8,
    value: f64,
    pointer: *const AwkStr,
) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    let idx_f = if tag == FLOAT_TAG {
        value
    } else {
        string_to_number(data_ptr, pointer)
    };
    let idx = idx_f.round() as usize;
    let str = data.columns.get(idx);
    data.calls.log(Call::Column(idx_f));
    println!(
        "\tgetting column tag:{} float:{} ptr:{:?}",
        tag, value, pointer
    );
    data.string_out("column", str.bytes());
    Rc::into_raw(Rc::new(str))
}

pub extern "C" fn free_string(data_ptr: *mut c_void, ptr: *const AwkStr) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::FreeString);
    println!("\tfreeing ptr {:?}", ptr);

    let string_data = unsafe { Rc::from_raw(ptr) };
    data.string_in("free_string", string_data.bytes());
    if Rc::strong_count(&string_data) > 1000 {
        panic!("count is very large! {}", Rc::strong_count(&string_data));
    }
    print!("\tstring is: '");
    stdout().write_all(string_data.bytes()).unwrap();
    println!("' count is now: {}", Rc::strong_count(&string_data).saturating_sub(1));
    0.0
}

pub extern "C" fn free_if_string(data_ptr: *mut c_void, tag: i8, string: *const AwkStr) {
    if tag == STRING_TAG {
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
    data.string_in("concat lhs", &*lhs);
    data.string_in("concat rhs", &*rhs);

    lhs.push_str(&rhs);
    println!("\tResult: ");
    stdout().write_all(&*lhs).unwrap();
    println!();
    data.string_out("concat result", &*lhs);
    Rc::into_raw(Rc::new(lhs))
}

pub extern "C" fn empty_string(data: *mut c_void) -> *const AwkStr {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::EmptyString);
    let rc = Rc::new(AwkStr::new(vec![]));
    data.string_out("empty_string", rc.bytes());
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
    let res = string_to_float(&*string);
    Rc::into_raw(string);
    println!("\tret {}", res);
    res
}

pub extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::NumberToString);
    println!("\tnum: {}", value);
    let str = AwkStr::new(data.float_parser.parse(value));
    let heap_alloc_string = Rc::new(str);
    data.string_out("number_to_string", &*heap_alloc_string);
    let ptr = Rc::into_raw(heap_alloc_string);
    ptr
}

pub extern "C" fn copy_string(data_ptr: *mut c_void, ptr: *mut AwkStr) -> *const AwkStr {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::CopyString);

    let original = unsafe { Rc::from_raw(ptr) };
    data.string_out("copy_string", &*original);
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

pub extern "C" fn copy_if_string(_data: *mut c_void, tag: i8, ptr: *mut AwkStr) -> *const AwkStr {
    if tag == STRING_TAG {
        copy_string(_data, ptr)
    } else {
        ptr
    }
}

pub extern "C" fn binop(
    data: *mut c_void,
    l_ptr: *const AwkStr,
    r_ptr: *const AwkStr,
    binop: BinOp,
) -> std::os::raw::c_double {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::BinOp);
    let left = unsafe { Rc::from_raw(l_ptr) };
    let right = unsafe { Rc::from_raw(r_ptr) };

    let res = match binop {
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
    };
    let res = if res { 1.0 } else { 0.0 };
    print!("\tBinop called: '");
    stdout().write_all(left.bytes()).unwrap();
    print!(" {} ", binop);
    stdout().write_all(right.bytes()).unwrap();
    println!(" = {}", res);
    data.string_in("binop left", &*left);
    data.string_in("binop right", &*right);
    // Implicitly drop left and right
    res
}

pub extern "C" fn print_error(_data_ptr: *mut std::os::raw::c_void, code: ErrorCode) {
    eprintln!("error {:?}", code)
}

pub extern "C" fn array_assign(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    key_tag: i8,
    key_num: f64,
    key_ptr: *mut AwkStr,
    tag: i8,
    float: f64,
    ptr: *mut AwkStr,
) {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ArrayAssign);
    let res = data.arrays.assign(array, MapValue::new(key_tag, key_num, key_ptr), MapValue::new(tag, float, ptr));
    match res {
        None => {}
        Some(existing) => {
            if existing.tag == STRING_TAG {
                println!("\tfreeing prior value from array");
                unsafe { Rc::from_raw(existing.ptr) };
                // implicitly drop RC here. Do not report as a string_in our out since it was
                // already stored in the runtime and droped from the runtime.
            }
        }
    }
    if key_tag == STRING_TAG {
        let rc = unsafe { Rc::from_raw(key_ptr) };
        data.string_in("array_access_key", &*rc);
        // implicitly drop here
    };
    if tag == STRING_TAG {
        let val = unsafe { Rc::from_raw(ptr) };
        data.string_in("array_access_val", &*val);
        // We don't drop it here because it is now stored in the hashmap.
        Rc::into_raw(val);
    }
}

pub extern "C" fn array_access(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    in_tag: i8,
    in_float: f64,
    in_ptr: *const AwkStr,
    out_tag: *mut i8,
    out_float: *mut f64,
    out_value: *mut *mut AwkStr,
) {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ArrayAccess);
    println!("\tarray access for key tag:{} flt:{} ptr:{:?}", in_tag, in_float, in_ptr);
    match data.arrays.access(array, MapValue::new(in_tag, in_float, in_ptr)) {
        None => unsafe {
            println!("\tarray access for non-existant key");
            *out_tag = STRING_TAG;
            *out_value = empty_string(data_ptr) as *mut AwkStr;
        },
        Some(value) => unsafe {
            *out_tag = value.tag;
            *out_float = value.float;
            if value.tag == STRING_TAG {
                let rc = Rc::from_raw(value.ptr);
                let cloned = rc.clone();
                data.string_out("array_access", &*cloned);

                Rc::into_raw(rc);

                *out_value = Rc::into_raw(cloned) as *mut AwkStr;
            }
        },
    }
    if in_tag == STRING_TAG {
        let rc = unsafe { Rc::from_raw(in_ptr) };
        data.string_in("input_str_to_array_access", &*rc);
    }
}

pub extern "C" fn in_array(
    data_ptr: *mut std::os::raw::c_void,
    array: i32,
    in_tag: i8,
    in_float: f64,
    in_ptr: *const AwkStr,
) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::InArray);
    let res = data.arrays.in_array(array, MapValue::new(in_tag, in_float, in_ptr));
    if in_tag == STRING_TAG {
        let rc = unsafe { Rc::from_raw(in_ptr) };
        data.string_in("input_str_to_array_access", &*rc);
    }
    if res {
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
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };

    let mut lhs = match Rc::try_unwrap(lhs) {
        Ok(str) => {
            print!("\tDowngraded RC into box for string ");
            stdout().write_all(str.bytes()).unwrap();
            println!();
            str
        }
        Err(rc) => (*rc).clone(),
    };
    data.string_in("concat-indices lhs", &*lhs);
    data.string_in("concat-indices rhs", &*rhs);

    lhs.push_str("-".as_bytes());
    lhs.push_str(&rhs);
    data.string_out("concat indices result", &lhs);
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
        data.string_in("printf fstring", &*fstring);
        data.output.extend_from_slice(&*fstring);
        stdout().write_all(fstring.bytes()).unwrap();
        for i in 0..(nargs as isize) {
            let _tag = *(base_ptr.offset(i * 3) as *const i8);
            let _float = *(base_ptr.offset(i * 3 + 1) as *const f64);
            let ptr = *(base_ptr.offset(i * 3 + 2) as *const *mut AwkStr);
            // args.push((tag, float, ptr));
            let str = Rc::from_raw(ptr);
            data.output.extend_from_slice(&*str);
            data.string_in("printf in arg", &*str);
            stdout().write_all(str.bytes()).unwrap();
        }
        // Rc::from_raw(fstring)
    };
}