use std::os::raw::c_void;
use std::rc::Rc;
use mawk_regex::Regex;
use crate::codegen::{FLOAT_TAG, STRING_TAG};
use crate::lexer::BinOp;
use crate::runtime::call_log::Call;
use crate::runtime::debug_runtime::{cast_to_runtime_data, RuntimeData};
use crate::runtime::ErrorCode;
use crate::runtime::float_parser::string_to_float;

pub extern "C" fn print_string(data: *mut c_void, value: *mut String) {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::PrintString);
    let str = unsafe { Rc::from_raw(value) };

    let res = if str.ends_with("\n") {
        format!("{}", str)
    } else {
        format!("{}\n", str)
    };
    data.output.push_str(&res);
    println!("{}", str);
    data.string_in("print_string", &str)
}

pub extern "C" fn print_float(data: *mut c_void, value: f64) {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::PrintFloat);
    let res = format!("{}\n", value);
    data.output.push_str(&res);
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

pub extern "C" fn to_lower(data_ptr: *mut c_void, ptr: *const String) -> *const String {
    let ptr = unsafe { Rc::from_raw(ptr) };
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ToLower);
    data.string_in("to_lower", &*ptr);
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => unsafe {
            if str.is_ascii() {
                let bytes = str.as_bytes_mut();
                bytes.make_ascii_lowercase();
                Rc::into_raw(Rc::new(str))
            } else {
                let lowercased = Rc::new(str.to_lowercase());
                Rc::into_raw(lowercased)
            }
        },
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_lowercase())),
    };
    let str = unsafe { Rc::from_raw(str) };
    data.string_out("to_lower", &*str);
    Rc::into_raw(str)
}

pub extern "C" fn split(data_ptr: *mut c_void, string: *const String, array: i32) {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Split);

}

pub extern "C" fn split_ere(data_ptr: *mut c_void, string: *const String, array: i32, ere_split: *const String) {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::SplitEre)
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

pub extern "C" fn length(data_ptr: *mut c_void, str: *const String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::Length);
    let str = unsafe { Rc::from_raw(str) };
    data.string_in("length ptr", &*str);
    str.chars().count() as f64
    // Drop str
}

pub extern "C" fn to_upper(data_ptr: *mut c_void, ptr: *const String) -> *const String {
    let ptr = unsafe { Rc::from_raw(ptr) };
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ToUpper);
    data.string_in("to_lower", &*ptr);
    let str = match Rc::try_unwrap(ptr) {
        Ok(mut str) => unsafe {
            if str.is_ascii() {
                let bytes = str.as_bytes_mut();
                bytes.make_ascii_uppercase();
                Rc::into_raw(Rc::new(str))
            } else {
                let uppercased = Rc::new(str.to_uppercase());
                Rc::into_raw(uppercased)
            }
        },
        Err(ptr) => Rc::into_raw(Rc::new(ptr.to_uppercase())),
    };
    let str = unsafe { Rc::from_raw(str) };
    data.string_out("to_upper", &*str);
    Rc::into_raw(str)
}

pub extern "C" fn column(
    data_ptr: *mut c_void,
    tag: i8,
    value: f64,
    pointer: *const String,
) -> *const String {
    let data = cast_to_runtime_data(data_ptr);
    let idx_f = if tag == FLOAT_TAG {
        value
    } else {
        string_to_number(data_ptr, pointer)
    };
    let idx = idx_f.round() as usize;
    let str = data.columns.get(idx);
    data.calls.log(Call::Column(idx_f, str.clone()));
    println!(
        "\tgetting column tag:{} float:{} ptr:{:?}",
        tag, value, pointer
    );
    data.string_out("column", &str);
    Rc::into_raw(Rc::new(str))
}

pub extern "C" fn free_string(data_ptr: *mut c_void, ptr: *const String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::FreeString);
    println!("\tfreeing ptr {:?}", ptr);

    let string_data = unsafe { Rc::from_raw(ptr) };
    data.string_in("free_string", &*string_data);
    if Rc::strong_count(&string_data) > 1000 {
        panic!("count is very large! {}", Rc::strong_count(&string_data));
    }
    println!(
        "\tstring is: '{}' count is now: {}",
        string_data,
        Rc::strong_count(&string_data).saturating_sub(1)
    );
    0.0
}

pub extern "C" fn free_if_string(data_ptr: *mut c_void, tag: i8, string: *const String) {
    if tag == STRING_TAG {
        free_string(data_ptr, string);
    }
}

pub extern "C" fn concat(
    data: *mut c_void,
    left: *const String,
    right: *const String,
) -> *const String {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::Concat);
    println!("\t{:?}, {:?}", left, right);
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };

    let mut lhs: String = match Rc::try_unwrap(lhs) {
        Ok(str) => {
            println!("\tDowngraded RC into box for string {}", str);
            str
        }
        Err(rc) => (*rc).clone(),
    };
    data.string_in("concat lhs", &*lhs);
    data.string_in("concat rhs", &*rhs);

    lhs.push_str(&rhs);
    println!("\tResult: '{}'", lhs);
    data.string_out("concat result", &lhs);
    Rc::into_raw(Rc::new(lhs))
}

pub extern "C" fn empty_string(data: *mut c_void) -> *const String {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::EmptyString);
    let rc = Rc::new("".to_string());
    data.string_out("empty_string", &*rc);
    let ptr = Rc::into_raw(rc);
    println!("\tempty string is {:?}", ptr);
    ptr
}

pub extern "C" fn string_to_number(data_ptr: *mut c_void, ptr: *const String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::StringToNumber);

    let string = unsafe { Rc::from_raw(ptr) };
    println!("\tstring_to_number {:?} '{}'", ptr, string);
    let res = string_to_float(&*string);
    Rc::into_raw(string);
    println!("\tret {}", res);
    res
}

pub extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const String {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::NumberToString);
    println!("\tnum: {}", value);
    let str = data.float_parser.parse(value);
    let heap_alloc_string = Rc::new(str);
    data.string_out("number_to_string", &*heap_alloc_string);
    let ptr = Rc::into_raw(heap_alloc_string);
    ptr
}

pub extern "C" fn copy_string(data_ptr: *mut c_void, ptr: *mut String) -> *const String {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::CopyString);

    let original = unsafe { Rc::from_raw(ptr as *mut String) };
    data.string_out("copy_string", &*original);
    println!(
        "\tCopying string {:?} '{}' count is {}",
        ptr,
        original,
        Rc::strong_count(&original)
    );
    let copy = original.clone();
    Rc::into_raw(original);

    println!("\tNew count is {}", Rc::strong_count(&copy));
    let copy = Rc::into_raw(copy);
    println!("\tCopy is: {:?}", copy);
    copy
}

pub extern "C" fn copy_if_string(_data: *mut c_void, tag: i8, ptr: *mut String) -> *const String {
    if tag == STRING_TAG {
        copy_string(_data, ptr)
    } else {
        ptr
    }
}

pub extern "C" fn binop(
    data: *mut c_void,
    l_ptr: *const String,
    r_ptr: *const String,
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
    println!(
        "\tBinop called: '{}' {:?} '{}' == {}",
        left, binop, right, res
    );
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
    key_ptr: *mut String,
    tag: i8,
    float: f64,
    ptr: *mut String,
) {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ArrayAssign);
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
    in_ptr: *const String,
    out_tag: *mut i8,
    out_float: *mut f64,
    out_value: *mut *mut String,
) {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::ArrayAccess);
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
                data.string_out("array_access", &*cloned);

                Rc::into_raw(rc);

                *out_value = Rc::into_raw(cloned) as *mut String;
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
    in_ptr: *const String,
) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::InArray);
    let res = data.arrays.in_array(array, (in_tag, in_float, in_ptr));
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
    left: *const String,
    right: *const String,
) -> *const String {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::ConcatArrayIndices);
    println!("\t{:?}, {:?}", left, right);
    let lhs = unsafe { Rc::from_raw(left) };
    let rhs = unsafe { Rc::from_raw(right) };

    let mut lhs: String = match Rc::try_unwrap(lhs) {
        Ok(str) => {
            println!("\tDowngraded RC into box for string {}", str);
            str
        }
        Err(rc) => (*rc).clone(),
    };
    data.string_in("concat-indices lhs", &*lhs);
    data.string_in("concat-indices rhs", &*rhs);

    lhs.push_str("-");
    lhs.push_str(&rhs);
    data.string_out("concat indices result", &lhs);
    let res = Rc::into_raw(Rc::new(lhs));
    println!("\treturning {:?}", res);
    res
}

pub extern "C" fn printf(data: *mut c_void, fstring: *mut String, nargs: i32, args: *mut c_void) {
    let data = cast_to_runtime_data(data);
    // let mut args = vec![];
    let base_ptr = args as *mut f64;
    unsafe {
        let fstring = Rc::from_raw(fstring);
        data.string_in("printf fstring", &*fstring);
        data.output.push_str(&*fstring);
        print!("{}", fstring);
        for i in 0..(nargs as isize) {
            let _tag = *(base_ptr.offset(i * 3) as *const i8);
            let _float = *(base_ptr.offset(i * 3 + 1) as *const f64);
            let ptr = *(base_ptr.offset(i * 3 + 2) as *const *mut String);
            // args.push((tag, float, ptr));
            let str = Rc::from_raw(ptr);
            data.output.push_str(&*str);
            data.string_in("printf in arg", &*str);
            print!("{}", str)
        }
        // Rc::from_raw(fstring)
    };
}