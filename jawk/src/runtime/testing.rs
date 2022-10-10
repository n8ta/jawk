use std::collections::HashMap;
use crate::codgen::{FLOAT_TAG, STRING_TAG, ValueT};
use crate::columns::Columns;
use crate::lexer::BinOp;
use crate::runtime::call_log::{Call, CallLog};
use crate::runtime::{ErrorCode, Runtime};
use gnu_libjit::{Abi, Context, Function, Value};
use std::ffi::c_void;
use std::rc::Rc;
use mawk_regex::Regex;

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
    Rc::into_raw(str);
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

extern "C" fn free_string(data_ptr: *mut c_void, ptr: *mut String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::FreeString);


    let string_data = unsafe { Rc::from_raw(ptr) };
    data.string_in("free_string", &*string_data);
    println!(
        "\tstring is: '{}' count is now: {}",
        string_data,
        Rc::strong_count(&string_data) - 1
    );
    0.0
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
    ptr
}

extern "C" fn string_to_number(data_ptr: *mut c_void, ptr: *const String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::StringToNumber);

    let string = unsafe { Rc::from_raw(ptr) };
    println!("\tstring_to_number {:?} '{}'", ptr, string);
    let res = if string.len() == 0 {
        0.0
    } else {
        string
            .parse()
            .expect(&format!("couldn't convert string to number {}", string))
    };
    Rc::into_raw(string);
    println!("\tret {}", res);
    res
}

extern "C" fn number_to_string(data_ptr: *mut c_void, value: f64) -> *const String {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::NumberToString);
    println!("\tnum: {}", value);
    let value = if value.fract() == 0.0 {
        value.floor()
    } else {
        value
    };

    let heap_alloc_string = Rc::new(value.to_string());

    data.string_out("number_to_string", &*heap_alloc_string);
    let str = (*heap_alloc_string).clone();
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
            let RE = Regex::new(&right);
            RE.matches(&left)
        }
        BinOp::NotMatchedBy => {
            let RE = Regex::new(&right);
            !RE.matches(&left)
        }
    };
    let res = if res { 1.0 } else { 0.0 };
    println!(
        "\tBinop called: '{}' {:?} '{}' == {}",
        left, binop, right, res
    );
    Rc::into_raw(left);
    Rc::into_raw(right);
    res
}

extern "C" fn print_error(data: *mut std::os::raw::c_void, code: ErrorCode) {
    eprintln!("error {:?}", code)
}

extern "C" fn array_assign(data_ptr: *mut std::os::raw::c_void,
                           array: i32,
                           index: *mut String, tag: i8, float: f64, ptr: *mut String) {
    let data = cast_to_runtime_data(data_ptr);
    let index = unsafe {
        Rc::from_raw(index)
    };
    data.calls.log(Call::ArrayAssign);
    data.string_in("array_assign_index", &*index);


    if tag == STRING_TAG {
        unsafe {
            data.string_in("array_assign_value", &*ptr);
        }
    }

    let array = &mut data.arrays[array as usize];

    if let Some(val) = array.get_mut(&*index) {
        if val.0 == STRING_TAG {
            free_string(data_ptr, val.2);
        }
        val.0 = tag;
        val.1 = float;
        val.2 = ptr;
    } else {
        array.insert(index, (tag, float, ptr));
    }
}

extern "C" fn array_access(data_ptr: *mut std::os::raw::c_void,
                           array: i32,
                           index: *mut String,
                           out_tag: *mut i8,
                           out_float: *mut f64,
                           out_value: *mut (*mut String)) {
    let data = cast_to_runtime_data(data_ptr);
    let index = unsafe {
        Rc::from_raw(index)
    };
    data.calls.log(Call::ArrayAccess);
    data.string_in("array_access_index", &*index);

    let array = &mut data.arrays[array as usize];
    unsafe {
        if let Some((tag, float, str)) = array.get(&*index) {
            *out_tag = *tag;
            *out_float = *float;
            if *tag == STRING_TAG {
                // Data stored in hashmap is a string clone it to up the ref count
                // and return the cloned ptr.
                let rc = unsafe { Rc::from_raw(*str) };
                data.string_out("array_access", &*rc);
                let cloned = rc.clone();
                Rc::into_raw(rc);
                *out_value = Rc::into_raw(cloned) as *mut String;
            }
        } else {
            println!("\tArray access returns empty string");
            let res = empty_string(data_ptr);
            *out_tag = STRING_TAG;
            *out_float = 1337.0;
            *out_value = res as *mut String;
        }
    }
}

extern "C" fn in_array(data_ptr: *mut c_void, array: i32, index: *mut String) -> f64 {
    let data = cast_to_runtime_data(data_ptr);
    data.calls.log(Call::InArray);
    let indices = unsafe { Rc::from_raw(index) };
    data.string_in("in_array_indices", &*indices);
    let array = &data.arrays[array as usize];
    unsafe {
        if array.contains_key(&*index) { 1.0 } else { 0.0 }
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
    Rc::into_raw(Rc::new(lhs))
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
            let tag = *(base_ptr.offset(i * 3) as *const i8);
            let float = *(base_ptr.offset(i * 3 + 1) as *const f64);
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

extern "C" fn helper(_data: std::os::raw::c_int) -> f64 {
    return 1.1;
}

pub struct TestRuntime {
    runtime_data_constant: Option<Value>,
    runtime_data: *mut c_void,
    next_line: *mut c_void,
    column: *mut c_void,
    free_string: *mut c_void,
    string_to_number: *mut c_void,
    number_to_string: *mut c_void,
    print_string: *mut c_void,
    print_float: *mut c_void,
    concat: *mut c_void,
    concat_array_indices: *mut c_void,
    copy_string: *mut c_void,
    binop: *mut c_void,
    empty_string: *mut c_void,
    helper: *mut c_void,
    print_error: *mut c_void,
    array_access: *mut c_void,
    array_assign: *mut c_void,
    in_array: *mut c_void,
    printf: *mut c_void,
}

pub struct RuntimeData {
    columns: Columns,
    canary: String,
    output: String,
    calls: CallLog,
    string_out: usize,
    strings_in: usize,
    arrays: Vec<HashMap<Rc<String>, (i8, f64, *mut String)>>,
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
            arrays: vec![],
        }
    }
}

impl TestRuntime {
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
        if let Some(val) = &self.runtime_data_constant {
            val.clone()
        } else {
            let val = func.create_void_ptr_constant(self.runtime_data as *mut c_void);
            self.runtime_data_constant.insert(val.clone());
            val
        }
    }
}

impl Runtime for TestRuntime {
    fn new(files: Vec<String>) -> TestRuntime {
        let data = Box::new(RuntimeData::new(files));
        let runtime_data = (Box::leak(data) as *mut RuntimeData) as *mut c_void;
        let rt = TestRuntime {
            runtime_data,
            runtime_data_constant: None,
            next_line: next_line as *mut c_void,
            column: column as *mut c_void,
            free_string: free_string as *mut c_void,
            string_to_number: string_to_number as *mut c_void,
            copy_string: copy_string as *mut c_void,
            number_to_string: number_to_string as *mut c_void,
            print_string: print_string as *mut c_void,
            print_float: print_float as *mut c_void,
            concat: concat as *mut c_void,
            empty_string: empty_string as *mut c_void,
            binop: binop as *mut c_void,
            helper: helper as *mut c_void,
            print_error: print_error as *mut c_void,
            array_access: array_access as *mut c_void,
            array_assign: array_assign as *mut c_void,
            in_array: in_array as *mut c_void,
            concat_array_indices: concat_array_indices as *mut c_void,
            printf: printf as *mut c_void,
        };
        println!("binop {:?}", rt.binop);
        rt
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

    fn array_access(&mut self, func: &mut Function, array: Value, index: Value, out_tag_ptr: Value, out_float_ptr: Value, out_ptr_ptr: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.array_access, vec![data_ptr, array, index, out_tag_ptr, out_float_ptr, out_ptr_ptr], None, Abi::Cdecl);
    }

    fn array_assign(&mut self, func: &mut Function, array: Value, index: Value, tag: Value, float: Value, ptr: Value) {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.array_assign, vec![data_ptr, array, index, tag, float, ptr], None, Abi::Cdecl);
    }

    fn in_array(&mut self, func: &mut Function, array: Value, index: Value) -> Value {
        let data_ptr = self.data_ptr(func);
        func.insn_call_native(self.in_array, vec![data_ptr, array, index], Some(Context::float64_type()), Abi::Cdecl)
    }

    fn allocate_arrays(&mut self, count: usize) {
        let data = cast_to_runtime_data(self.runtime_data);
        for _ in 0..count {
            data.arrays.push(HashMap::new());
        }
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
        if d.canary != CANARY {
            eprintln!("RUNTIME DATA LOADED WRONG. CANARY MISSING");
            std::process::exit(-1);
        }
        d
    }
}
