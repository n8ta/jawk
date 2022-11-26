use crate::codegen::{ValueT, FLOAT_TAG, STRING_TAG};
use crate::columns::Columns;
use crate::lexer::BinOp;
use crate::parser::ScalarType;
use crate::runtime::arrays::Arrays;
use crate::runtime::call_log::{Call, CallLog};
use crate::runtime::float_parser::{FloatParser, string_to_float};
use crate::runtime::{ErrorCode, Runtime};
use gnu_libjit::{Abi, Context, Function, Value};
use hashbrown::HashMap;
use mawk_regex::Regex;
use std::ffi::c_void;
use std::rc::Rc;
use crate::{runtime_fn, runtime_fn_no_args, runtime_fn_no_ret};

pub const CANARY: &str = "this is the canary!";

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

extern "C" fn next_line(data: *mut c_void) -> f64 {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::NextLine);
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

extern "C" fn free_string(data_ptr: *mut c_void, ptr: *const String) -> f64 {
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

extern "C" fn free_if_string(data_ptr: *mut c_void, tag: i8, string: *const String) {
    if tag == STRING_TAG {
        free_string(data_ptr, string);
    }
}

extern "C" fn concat(
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

extern "C" fn empty_string(data: *mut c_void) -> *const String {
    let data = cast_to_runtime_data(data);
    data.calls.log(Call::EmptyString);
    let rc = Rc::new("".to_string());
    data.string_out("empty_string", &*rc);
    let ptr = Rc::into_raw(rc);
    println!("\tempty string is {:?}", ptr);
    ptr
}

extern "C" fn string_to_number(data_ptr: *mut c_void, ptr: *const String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::StringToNumber);

    let string = unsafe { Rc::from_raw(ptr) };
    println!("\tstring_to_number {:?} '{}'", ptr, string);
    let res = string_to_float(&*string);
    Rc::into_raw(string);
    println!("\tret {}", res);
    res
}

extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const String {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::NumberToString);
    println!("\tnum: {}", value);
    let str = data.float_parser.parse(value);
    let heap_alloc_string = Rc::new(str);
    data.string_out("number_to_string", &*heap_alloc_string);
    let ptr = Rc::into_raw(heap_alloc_string);
    ptr
}

extern "C" fn copy_string(data_ptr: *mut c_void, ptr: *mut String) -> *const String {
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

extern "C" fn copy_if_string(_data: *mut c_void, tag: i8, ptr: *mut String) -> *const String {
    if tag == STRING_TAG {
        copy_string(_data, ptr)
    } else {
        ptr
    }
}

extern "C" fn binop(
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

extern "C" fn print_error(_data_ptr: *mut std::os::raw::c_void, code: ErrorCode) {
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

extern "C" fn array_access(
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

extern "C" fn in_array(
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

extern "C" fn concat_array_indices(
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

extern "C" fn printf(data: *mut c_void, fstring: *mut String, nargs: i32, args: *mut c_void) {
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

// Helper for build debug mapping form pointers to their runtime function
fn insert(mapping: &mut HashMap<String, String>, ptr: *mut c_void, name: &str) {
    let ptr_hex = format!("0x{:x}", ptr as i64);
    let with_name = format!("{} 0x{:x}", name, ptr as i64);
    mapping.insert(ptr_hex, with_name);
}

pub struct DebugRuntime {
    runtime_data: *mut c_void,
}

pub struct RuntimeData {
    columns: Columns,
    canary: String,
    output: String,
    calls: CallLog,
    string_out: usize,
    strings_in: usize,
    arrays: Arrays,
    float_parser: FloatParser,
}

impl RuntimeData {
    pub fn string_out(&mut self, src: &str, string: &str) {
        println!("\t===> {} '{}'", src, string);
        self.string_out += 1;
    }
    pub fn string_in(&mut self, src: &str, string: &str) {
        println!("\t<=== {} '{}'", src, string);
        self.strings_in += 1;
    }
    pub fn new(files: Vec<String>) -> RuntimeData {
        RuntimeData {
            canary: String::from(CANARY),
            columns: Columns::new(files),
            output: String::new(),
            calls: CallLog::new(),
            string_out: 0,
            strings_in: 0,
            arrays: Arrays::new(),
            float_parser: FloatParser::new(),
        }
    }
}

impl DebugRuntime {
    #[allow(dead_code)]
    pub fn output(&self) -> String {
        cast_to_runtime_data(self.runtime_data).output.clone()
    }
    #[allow(dead_code)]
    pub fn strings_in(&self) -> usize {
        cast_to_runtime_data(self.runtime_data).strings_in
    }
    #[allow(dead_code)]
    pub fn strings_out(&self) -> usize {
        cast_to_runtime_data(self.runtime_data).string_out
    }

    #[allow(dead_code)]
    fn data_ptr(&mut self, func: &mut Function) -> Value {
        func.create_void_ptr_constant(self.runtime_data as *mut c_void)
    }
}

/*
    fn string_to_number(&mut self, func: &mut Function, ptr: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            string_to_number as *mut c_void,
            vec![data_ptr, ptr],
            Some(Context::float64_type()),
            Abi::Cdecl,
        )
    }

 */


impl Runtime for DebugRuntime {
    fn new(_context: &Context, files: Vec<String>) -> DebugRuntime {
        let data = Box::new(RuntimeData::new(files));
        let runtime_data = (Box::leak(data) as *mut RuntimeData) as *mut c_void;
        DebugRuntime { runtime_data }
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

    fn allocate_arrays(&mut self, count: usize) {
        let data = cast_to_runtime_data(self.runtime_data);
        data.arrays.allocate(count);
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

    runtime_fn_no_args!(call_next_line, next_line, Some(Context::float64_type()));
    runtime_fn!(column, column, Some(Context::void_ptr_type()), tag: Value, float: Value, pointer: Value);
    runtime_fn!(string_to_number, string_to_number, Some(Context::float64_type()), arg0: Value);
    runtime_fn!(number_to_string, number_to_string, Some(Context::void_ptr_type()), arg0: Value);
    runtime_fn_no_ret!(print_string, print_string, None, arg0: Value);
    runtime_fn_no_ret!(print_float, print_float, None, arg0: Value);
    runtime_fn!(concat, concat, Some(Context::void_ptr_type()), arg0: Value, arg1: Value);
    runtime_fn!(concat_array_indices, concat_array_indices, Some(Context::void_ptr_type()), arg0: Value, arg1: Value);
    runtime_fn_no_args!(empty_string, empty_string, Some(Context::void_ptr_type()));
    runtime_fn_no_ret!(array_access, array_access, None,array: Value,key_tag: Value,key_num: Value,key_ptr: Value,out_tag_ptr: Value,out_float_ptr: Value,out_ptr_ptr: Value);
    runtime_fn_no_ret!(array_assign, array_assign, None,array: Value,key_tag: Value,key_num: Value,key_ptr: Value,tag: Value,float: Value,ptr: Value);
    runtime_fn!(in_array, in_array, Some(Context::float64_type()),array: Value,key_tag: Value,key_num: Value,key_ptr: Value);

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
        let mut mapping = HashMap::new();
        insert(&mut mapping, self.runtime_data, "runtime_data");
        insert(&mut mapping, next_line as *mut c_void, "next_line");
        insert(&mut mapping, column as *mut c_void, "column");
        insert(&mut mapping, free_string as *mut c_void, "free_string");
        insert(
            &mut mapping,
            free_if_string as *mut c_void,
            "free_if_string",
        );
        insert(
            &mut mapping,
            string_to_number as *mut c_void,
            "string_to_number",
        );
        insert(&mut mapping, copy_string as *mut c_void, "copy_string");
        insert(
            &mut mapping,
            copy_if_string as *mut c_void,
            "copy_if_string",
        );
        insert(
            &mut mapping,
            number_to_string as *mut c_void,
            "number_to_string",
        );
        insert(&mut mapping, print_string as *mut c_void, "print_string");
        insert(&mut mapping, print_float as *mut c_void, "print_float");
        insert(&mut mapping, concat as *mut c_void, "concat");
        insert(&mut mapping, empty_string as *mut c_void, "empty_string");
        insert(&mut mapping, binop as *mut c_void, "binop");
        insert(&mut mapping, print_error as *mut c_void, "print_error");
        insert(&mut mapping, array_access as *mut c_void, "array_access");
        insert(&mut mapping, array_assign as *mut c_void, "array_assign");
        insert(&mut mapping, in_array as *mut c_void, "in_array");
        insert(
            &mut mapping,
            concat_array_indices as *mut c_void,
            "concat_array_indices",
        );
        insert(&mut mapping, printf as *mut c_void, "printf");
        mapping
    }
}

fn cast_to_runtime_data(data: *mut c_void) -> &'static mut RuntimeData {
    unsafe {
        let data = data as *mut RuntimeData;
        let d = &mut *data;
        if d.canary != CANARY {
            eprintln!("RUNTIME DATA LOADED WRONG. CANARY MISSING");
            std::process::exit(-1);
        }
        d
    }
}
