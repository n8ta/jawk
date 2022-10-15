use crate::codegen::{FLOAT_TAG, STRING_TAG};
use crate::columns::Columns;
use crate::lexer::BinOp;
use crate::runtime::{ErrorCode, Runtime};
use gnu_libjit::{Abi, Context, Function, Value};
use std::ffi::c_void;
use std::io::{BufWriter, StdoutLock, Write};
use std::rc::Rc;
use mawk_regex::Regex;
use lru_cache::LruCache;
use crate::runtime::arrays::{Arrays};

// Live runtime used by most programs.
// A pointer to the runtime data is provided for all calls but only used for some.
// Its mainly here for the test runtime.

pub extern "C" fn print_string(data: *mut c_void, value: *mut String) {
    let data = cast_to_runtime_data(data);
    let str = unsafe { Rc::from_raw(value) };
    if str.ends_with("\n") {
        data.stdout.write_all(str.as_bytes()).expect("failed to write to stdout")
    } else {
        data.stdout.write_all(str.as_bytes()).expect("failed to write to stdout");
        data.stdout.write_all("\n".as_bytes()).expect("failed to write to stdout");
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

extern "C" fn free_string(_data: *mut c_void, string: *const String) -> f64 {
    unsafe { Rc::from_raw(string) };
    0.0
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
    let res = if res { 1.0 } else { 0.0 };
    Rc::into_raw(left);
    Rc::into_raw(right);
    res
}

extern "C" fn string_to_number(_data: *mut c_void, ptr: *const String) -> f64 {
    let string = unsafe { Rc::from_raw(ptr) };
    let res = if string.len() == 0 {
        0.0
    } else {
        // TODO: Support scientific notation
        string
            .parse()
            .expect(&format!("couldn't convert string to number {}", string))
    };
    Rc::into_raw(string);
    res
}

extern "C" fn number_to_string(_data: *mut c_void, value: f64) -> *const String {
    let value = if value.fract() == 0.0 {
        value.floor()
    } else {
        value
    };
    let str = value.to_string();
    Rc::into_raw(Rc::new(str))
}

extern "C" fn copy_string(_data: *mut c_void, ptr: *mut String) -> *const String {
    let original = unsafe { Rc::from_raw(ptr) };
    let copy = original.clone();
    Rc::into_raw(original);
    Rc::into_raw(copy)
}

extern "C" fn print_error(_data: *mut std::os::raw::c_void, code: ErrorCode) {
    eprintln!("error {:?}", code)
}

extern "C" fn array_assign(data_ptr: *mut std::os::raw::c_void,
                           array: i32,
                           key_tag: i8,
                           key_num: f64,
                           key_ptr: *mut String,
                           tag: i8,
                           float: f64,
                           ptr: *mut String) {
    let data = cast_to_runtime_data(data_ptr);
    let res = data.arrays.assign(array, (key_tag, key_num, key_ptr), (tag, float, ptr));
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
        // implicitly drop here
    };
    if tag == STRING_TAG {
        let val = unsafe { Rc::from_raw(ptr) };
        // We don't drop it here because it is now stored in the hashmap.
        Rc::into_raw(val);
    }

}

extern "C" fn array_access(data_ptr: *mut std::os::raw::c_void,
                           array: i32,
                           in_tag: i8,
                           in_float: f64,
                           in_ptr: *mut String,
                           out_tag: *mut i8,
                           out_float: *mut f64,
                           out_value: *mut *mut String) {
    let data = cast_to_runtime_data(data_ptr);
    match data.arrays.access(array, (in_tag, in_float, in_ptr)) {
        None => {
            unsafe {
                *out_tag = STRING_TAG;
                *out_value = empty_string(data_ptr) as *mut String;
            }
        }
        Some((tag, float, str)) => {
            unsafe {
                *out_tag = *tag;
                *out_float = *float;
                if *tag == STRING_TAG {
                    let rc = Rc::from_raw(*str);
                    let cloned = rc.clone();
                    Rc::into_raw(rc);
                    *out_value = Rc::into_raw(cloned) as *mut String;
                }
            }
        }
    }
    if in_tag == STRING_TAG {
        free_string(data_ptr, in_ptr);
    }
}

extern "C" fn in_array(data_ptr: *mut c_void,
                       array: i32,
                       in_tag: i8,
                       in_float: f64,
                       in_ptr: *mut String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    let ret = if data.arrays.in_array(array, (in_tag, in_float, in_ptr)) {
        1.0
    } else {
        0.0
    };
    if in_tag == STRING_TAG {
        free_string(data_ptr, in_ptr);
    }
    ret
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
        data.stdout.write_all(fstring.as_bytes()).expect("to be able to write to stdout");
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

#[allow(dead_code)]
extern "C" fn helper(_data_ptr: *mut std::os::raw::c_void, _str: usize, _size: usize) -> f64 {
    return 1.1;
}

pub struct LiveRuntime {
    runtime_data_constant: Option<Value>,
    runtime_data: *mut RuntimeData,
    pub next_line: *mut c_void,
    pub column: *mut c_void,
    pub free_string: *mut c_void,
    pub string_to_number: *mut c_void,
    pub number_to_string: *mut c_void,
    pub print_string: *mut c_void,
    pub print_float: *mut c_void,
    pub copy_string: *mut c_void,
    pub concat: *mut c_void,
    pub concat_array_indices: *mut c_void,
    pub binop: *mut c_void,
    pub empty_string: *mut c_void,
    pub print_error: *mut c_void,
    pub array_access: *mut c_void,
    pub array_assign: *mut c_void,
    pub in_array: *mut c_void,
    pub printf: *mut c_void,
}

impl Drop for LiveRuntime {
    fn drop(&mut self) {
        unsafe {
            (*self.runtime_data).stdout.flush().expect("could not flush stdout");
        }
    }
}

// Pointer to this is passed in with every call. The reason we require it for every call instead of making it
// a rust global is so we can easily run tests fully independently of each other.
pub struct RuntimeData {
    columns: Columns,
    buffer: String,
    stdout: BufWriter<StdoutLock<'static>>,
    regex_cache: LruCache<String, Regex>,
    arrays: Arrays,
}

impl RuntimeData {
    pub fn new(files: Vec<String>) -> RuntimeData {
        RuntimeData {
            buffer: String::with_capacity(1000),
            columns: Columns::new(files),
            stdout: BufWriter::new(std::io::stdout().lock()),
            regex_cache: LruCache::new(10),
            arrays: Arrays::new(),
        }
    }
}

impl LiveRuntime {
    fn data_ptr(&mut self, func: &mut Function) -> Value {
        if let Some(val) = &self.runtime_data_constant {
            val.clone()
        } else {
            let val = func.create_void_ptr_constant(self.runtime_data as *mut c_void);
            let _ = self.runtime_data_constant.insert(val.clone());
            val
        }
    }
}

impl Runtime for LiveRuntime {
    fn new(files: Vec<String>) -> LiveRuntime {
        let data = Box::new(RuntimeData::new(files));
        let ptr = Box::leak(data);
        LiveRuntime {
            runtime_data_constant: None,
            runtime_data: ptr as *mut RuntimeData,
            next_line: next_line as *mut c_void,
            column: column as *mut c_void,
            free_string: free_string as *mut c_void,
            string_to_number: string_to_number as *mut c_void,
            copy_string: copy_string as *mut c_void,
            number_to_string: number_to_string as *mut c_void,
            print_string: print_string as *mut c_void,
            concat: concat as *mut c_void,
            concat_array_indices: concat_array_indices as *mut c_void,
            print_float: print_float as *mut c_void,
            empty_string: empty_string as *mut c_void,
            binop: binop as *mut c_void,
            print_error: print_error as *mut c_void,
            array_access: array_access as *mut c_void,
            array_assign: array_assign as *mut c_void,
            in_array: in_array as *mut c_void,
            printf: printf as *mut c_void,
        }
    }

    fn allocate_arrays(&mut self, count: usize) {
        unsafe {
            (*self.runtime_data).arrays.allocate(count)
        }
    }

    fn call_next_line(&mut self, func: &mut Function) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.next_line,
            vec![data_ptr],
            Some(Context::float64_type()),
            Abi::Cdecl,
        )
    }

    fn column(&mut self, func: &mut Function, tag: Value, float: Value, pointer: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.column,
            vec![data_ptr, tag, float, pointer],
            Some(Context::void_ptr_type()),
            Abi::Cdecl,
        )
    }

    fn free_string(&mut self, func: &mut Function, ptr: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.free_string, vec![data_ptr, ptr], None, Abi::Cdecl)
    }

    fn string_to_number(&mut self, func: &mut Function, ptr: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.string_to_number,
            vec![data_ptr, ptr],
            Some(Context::float64_type()),
            Abi::Cdecl,
        )
    }

    fn copy_string(&mut self, func: &mut Function, ptr: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.copy_string,
            vec![data_ptr, ptr],
            Some(Context::void_ptr_type()), Abi::Cdecl,
        )
    }

    fn number_to_string(&mut self, func: &mut Function, number: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.number_to_string,
            vec![data_ptr, number],
            Some(Context::void_ptr_type()), Abi::Cdecl,
        )
    }

    fn print_string(&mut self, func: &mut Function, ptr: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.print_string, vec![data_ptr, ptr], None, Abi::Cdecl);
    }

    fn print_float(&mut self, func: &mut Function, number: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.print_float, vec![data_ptr, number], None, Abi::Cdecl);
    }

    fn concat(&mut self, func: &mut Function, ptr1: Value, ptr2: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.concat,
            vec![data_ptr, ptr1, ptr2],
            Some(Context::void_ptr_type()),
            Abi::Cdecl,
        )
    }

    fn concat_array_indices(&mut self, func: &mut Function, ptr1: Value, ptr2: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.concat_array_indices,
            vec![data_ptr, ptr1, ptr2],
            Some(Context::void_ptr_type()),
            Abi::Cdecl,
        )
    }

    fn empty_string(&mut self, func: &mut Function) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.empty_string,
            vec![data_ptr],
            Some(Context::void_ptr_type()),
            Abi::Cdecl,
        )
    }

    fn binop(&mut self, func: &mut Function, ptr1: Value, ptr2: Value, binop: BinOp) -> Value {
        let binop = func.create_sbyte_constant(binop as i8);
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.binop,
            vec![data_ptr, ptr1, ptr2, binop],
            Some(Context::float64_type()),
            Abi::Cdecl,
        )
    }

    fn print_error(&mut self, func: &mut Function, error: ErrorCode) {
        let binop = func.create_sbyte_constant(error as i8);
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(
            self.print_error,
            vec![data_ptr, binop],
            None,
            Abi::Cdecl,
        );
    }

    fn array_access(&mut self, func: &mut Function, array: Value,
                    key_tag: Value,
                    key_num: Value,
                    key_ptr: Value,
                    out_tag_ptr: Value, out_float_ptr: Value, out_ptr_ptr: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.array_access, vec![data_ptr, array, key_tag, key_num, key_ptr, out_tag_ptr, out_float_ptr, out_ptr_ptr], None, Abi::Cdecl);
    }

    fn array_assign(&mut self, func: &mut Function, array: Value,
                    key_tag: Value,
                    key_num: Value,
                    key_ptr: Value,
                    tag: Value, float: Value, ptr: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.array_assign, vec![data_ptr, array, key_tag, key_num, key_ptr, tag, float, ptr], None, Abi::Cdecl);
    }

    fn in_array(&mut self, func: &mut Function, array: Value,
                key_tag: Value,
                key_num: Value,
                key_ptr: Value,
    ) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.in_array, vec![data_ptr, array, key_tag, key_num, key_ptr], Some(Context::float64_type()), Abi::Cdecl)
    }

    fn printf(&mut self, func: &mut Function, fstring: Value, nargs: Value, args: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.printf, vec![data_ptr, fstring, nargs, args], None, Abi::VarArg);
    }
}

pub fn cast_to_runtime_data(data: *mut c_void) -> &'static mut RuntimeData {
    unsafe {
        let data = data as *mut RuntimeData;
        let d = &mut *data;
        d
    }
}
