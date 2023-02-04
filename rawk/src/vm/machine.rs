use std::io::{Write};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::arrays::{Arrays, split_on_regex, split_on_string};
use crate::awk_str::{AwkStr, RcAwkStr};
use crate::columns::Columns;
use crate::{binop, mathop};
use crate::typing::GlobalArrayId;
use crate::vm::{Code, VmFunc, VmProgram};
use crate::vm::converter::Converter;
use crate::vm::ops::{add, sub, div, mult, exp, modulo, lt, gt, lteq, gteq, eq, neq};
use crate::util::{clamp_to_max_len, clamp_to_slice_index, index_of};
use crate::vm::regex_cache::RegexCache;
use crate::vm::runtime_scalar::{RuntimeScalar, StringScalar};
use crate::vm::vm_special_vars::{NUM_GSCALAR_SPECIALS, GlobalScalarSpecials};


struct FunctionScope {
    unknown_stack_base_offset: usize,
    str_stack_base_offset: usize,
    num_stack_base_offset: usize,
    array_base_offset: usize,
}

pub struct VirtualMachine<'a, OutT: Write, ErrT: Write> {
    global_scalars: Vec<RuntimeScalar>,

    // Value stacks
    unknown_stack: Vec<RuntimeScalar>,
    num_stack: Vec<f64>,
    str_stack: Vec<StringScalar>,
    arr_stack: Vec<GlobalArrayId>,

    scopes: Vec<FunctionScope>,
    arrays: Arrays,
    columns: Columns,
    converter: Converter,
    regex_cache: RegexCache,

    stdout: &'a mut OutT,
    stderr: &'a mut ErrT,

    srand_seed: f64,
}


impl<'a, OutT: Write, ErrT: Write> VirtualMachine<'a, OutT, ErrT> {
    pub fn new(files: Vec<String>, stdout: &'a mut OutT, stderr: &'a mut ErrT) -> Self {
        unsafe { libc::srand(09171998) }
        let s = Self {
            arr_stack: vec![],
            unknown_stack: vec![],
            num_stack: vec![],
            str_stack: vec![],
            scopes: vec![],
            columns: Columns::new(files),
            arrays: Arrays::new(),
            converter: Converter::new(),
            global_scalars: GlobalScalarSpecials::initialize(),
            regex_cache: RegexCache::new(),
            stdout,
            stderr,
            srand_seed: 09171998.0,
        };
        debug_assert!(s.global_scalars.len() == NUM_GSCALAR_SPECIALS);
        s
    }
    pub fn run(&mut self, program: &VmProgram) {
        self.arrays.allocate(program.analysis.global_arrays.len() as u16); // TODO u16max
        for _ in 0..program.analysis.global_scalars.len() {
            self.global_scalars.push(RuntimeScalar::Str(RcAwkStr::new(AwkStr::new(vec![]))));
        }
        self.run_function(program.main(), program)
    }

    fn push_unknown(&mut self, scalar: RuntimeScalar) {
        self.unknown_stack.push(scalar)
    }
    fn push_num(&mut self, num: f64) {
        self.num_stack.push(num)
    }
    fn push_str(&mut self, str: StringScalar) {
        self.str_stack.push(str)
    }
    fn push_arr(&mut self, array_id: GlobalArrayId) {
        self.arr_stack.push(array_id)
    }
    fn push_bool(&mut self, b: bool) {
        self.push_num(if b { 1.0 } else { 0.0 })
    }

    fn pop_array(&mut self) -> GlobalArrayId {
        unsafe { self.arr_stack.pop().unwrap() }
    }
    fn pop_unknown(&mut self) -> RuntimeScalar {
        unsafe { self.unknown_stack.pop().unwrap() }
    }
    fn pop_num(&mut self) -> f64 {
        unsafe { self.num_stack.pop().unwrap() }
    }
    fn pop_string(&mut self) -> StringScalar {
        unsafe { self.str_stack.pop().unwrap() }
    }

    fn peek_unknown(&self) -> &RuntimeScalar {
        unsafe { self.unknown_stack.last().unwrap_unchecked() }
    }
    fn peek_num(&self) -> &f64 {
        unsafe { self.num_stack.last().unwrap_unchecked() }
    }
    fn peek_str(&self) -> &StringScalar {
        unsafe { self.str_stack.last().unwrap_unchecked() }
    }

    fn set_scalar_arg(&mut self, idx: usize, value: RuntimeScalar) {
        let idx = unsafe { self.scopes.last().unwrap_unchecked().unknown_stack_base_offset + idx };
        self.unknown_stack[idx] = value;
    }

    fn get_scalar_arg(&mut self, idx: usize) -> RuntimeScalar {
        let idx = unsafe { self.scopes.last().unwrap_unchecked().unknown_stack_base_offset + idx };
        self.unknown_stack[idx].clone()
    }

    fn get_array_arg(&mut self, idx: usize) -> GlobalArrayId {
        let idx = unsafe { self.scopes.last().unwrap_unchecked().array_base_offset + idx };
        self.arr_stack[idx].clone()
    }

    fn val_to_num(&mut self, value: RuntimeScalar) -> f64 {
        match value {
            RuntimeScalar::Str(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
            RuntimeScalar::StrNum(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
            RuntimeScalar::Num(n) => n,
        }
    }

    fn str_to_num(&mut self, s: &RcAwkStr) -> f64 {
        self.converter.str_to_num(&*s).unwrap_or(0.0)
    }

    fn val_to_string(&mut self, value: RuntimeScalar) -> RcAwkStr {
        match value {
            RuntimeScalar::Str(s) => s,
            RuntimeScalar::StrNum(s) => s,
            RuntimeScalar::Num(n) => AwkStr::new_rc(self.converter.num_to_str_internal(n).to_vec()),
        }
    }

    fn val_to_string_scalar(&mut self, value: RuntimeScalar) -> StringScalar {
        match value {
            RuntimeScalar::Str(s) => StringScalar::Str(s),
            RuntimeScalar::StrNum(s) => StringScalar::StrNum(s),
            RuntimeScalar::Num(n) => StringScalar::Str(AwkStr::new_rc(self.converter.num_to_str_internal(n).to_vec())),
        }
    }


    fn val_is_numeric(&mut self, value: &RuntimeScalar) -> bool {
        match value {
            RuntimeScalar::Num(_) => true,
            RuntimeScalar::Str(_) => false,
            RuntimeScalar::StrNum(ptr) => {
                // TODO: Changing each occurrence of the decimal point character from the current locale to a period.
                if ptr.len() == 0 {
                    true
                } else {
                    self.converter.str_to_num(ptr).is_some()
                }
            }
        }
    }

    fn concat_array_indices(&mut self, count: u16) -> AwkStr {
        let subsep = self.global_scalars[GlobalScalarSpecials::SUBSEP as usize].clone();
        let subsep = self.val_to_string(subsep);
        let mut string = self.pop_string().downgrade_or_clone();
        for _ in 0..count - 1 {
            let addition = self.pop_unknown();
            let addition = self.val_to_string(addition);
            string.push_str(&subsep);
            string.push_str(&*addition);
        }
        string
    }

    fn run_function(&mut self, function: &VmFunc, program: &VmProgram) {
        let mut ip = 0;

        loop {
            let code = unsafe { function.get_unchecked(ip) };
            #[cfg(test)]
            {
                // Coloring makes it easier to match up scalar stack and array stack visually when debugging
                let red = "\x1b[0;31m";
                let yellow = "\x1b[0;33m";
                let end = "\x1b[0m";
                print!("ip {:2} {} ", ip, code.pretty_print_owned());
                self.print_stacks();
            }

            match code {
                Code::FloatZero => self.push_unknown(RuntimeScalar::Num(0.0)),
                Code::FloatOne => self.push_unknown(RuntimeScalar::Num(1.0)),
                Code::Pop => { unsafe { self.unknown_stack.pop().unwrap_unchecked(); } }
                Code::Column => {
                    let index = self.pop_num();
                    let idx = index.round() as usize;
                    let field = self.columns.get(idx);
                    self.push_str(StringScalar::StrNum(field.rc()))
                }
                Code::NextLine => {
                    let more_lines = self.columns.next_line().unwrap(); // TODO: no unwrap
                    self.push_bool(more_lines);
                }
                Code::AssignGsclVar(idx) => {
                    let scalar = self.pop_unknown();
                    self.global_scalars[idx.id as usize] = scalar;
                }
                Code::AssignRetGsclVar(idx) => {
                    let scalar = self.pop_unknown();
                    self.global_scalars[idx.id as usize] = scalar.clone();
                    self.push_unknown(scalar);
                }
                Code::AssignGsclNum(idx) => {
                    let num = self.pop_num();
                    self.global_scalars[idx.id as usize] = RuntimeScalar::Num(num);
                }
                Code::AssignRetGsclNum(idx) => {
                    let num = self.pop_num();
                    self.global_scalars[idx.id as usize] = RuntimeScalar::Num(num);
                    self.push_num(num);
                }
                Code::AssignGsclStr(idx) => {
                    let str: RuntimeScalar = self.pop_string().into();
                    self.global_scalars[idx.id as usize] = str;
                }
                Code::AssignRetGsclStr(idx) => {
                    let str: RuntimeScalar = self.pop_string().into();
                    self.global_scalars[idx.id as usize] = str.clone();
                    self.push_unknown(str);
                }
                Code::GsclVar(idx) => {
                    self.push_unknown(self.global_scalars[idx.id as usize].clone())
                }
                Code::GsclNum(idx) => {
                    let scl = self.global_scalars[idx.id as usize].clone();
                    let num = self.val_to_num(scl);
                    self.push_num(num);
                }
                Code::GsclStr(idx) => {
                    let scl = self.global_scalars[idx.id as usize].clone();
                    let str = self.val_to_string_scalar(scl);
                    self.push_str(str);
                }
                Code::AssignArgVar { arg_idx } => {
                    let new_value = self.pop_unknown();
                    self.set_scalar_arg(*arg_idx as usize, new_value);
                }
                Code::AssignRetArgVar { arg_idx } => {
                    let new_value = self.pop_unknown();
                    self.set_scalar_arg(*arg_idx as usize, new_value.clone());
                    self.push_unknown(new_value);
                }
                Code::AssignArgStr { arg_idx } => {
                    let new_value = self.pop_string();
                    self.set_scalar_arg(*arg_idx as usize, new_value.into());
                }
                Code::AssignRetArgStr { arg_idx } => {
                    let new_value = self.pop_string();
                    let new_value_clone: StringScalar = new_value.clone();
                    let new_value_rt: RuntimeScalar = new_value_clone.into();
                    self.set_scalar_arg(*arg_idx as usize, new_value_rt);
                    self.push_str(new_value);
                }
                Code::AssignArgNum { arg_idx } => {
                    let new_value = self.pop_num();
                    self.set_scalar_arg(*arg_idx as usize, RuntimeScalar::Num(new_value));
                }
                Code::AssignRetArgNum { arg_idx } => {
                    let new_value = self.pop_num();
                    self.set_scalar_arg(*arg_idx as usize, RuntimeScalar::Num(new_value));
                    self.push_num(new_value);
                }
                Code::ArgVar { arg_idx } => {
                    let arg = self.get_scalar_arg(*arg_idx as usize);
                    self.push_unknown(arg);
                }
                Code::ArgStr { arg_idx } => {
                    let arg = self.get_scalar_arg(*arg_idx as usize);
                    let arg = self.val_to_string_scalar(arg);
                    self.push_str(arg);
                }
                Code::ArgNum { arg_idx } => {
                    let arg = self.get_scalar_arg(*arg_idx as usize);
                    let arg = self.val_to_num(arg);
                    self.push_num(arg);
                }
                Code::Exp => { mathop!(self, exp); }
                Code::Mult => { mathop!(self, mult); }
                Code::Div => { mathop!(self, div); }
                Code::Mod => { mathop!(self, modulo); }
                Code::Add => { mathop!(self, add); }
                Code::Minus => { mathop!(self, sub); }
                Code::Lt => { binop!(self, lt); }
                Code::Gt => { binop!(self, gt); }
                Code::LtEq => { binop!(self, lteq); }
                Code::GtEq => { binop!(self, gteq); }
                Code::EqEq => { binop!(self, eq); }
                Code::Neq => { binop!(self, neq); }
                Code::Matches => {
                    // TODO: Regex stack??
                    let regex_str = self.pop_string(); // the regex
                    let str = self.pop_string(); // the string
                    let regex = self.regex_cache.get(&*regex_str);
                    let is_match = regex.matches(&str);
                    self.push_bool(is_match);
                }
                Code::NMatches => {
                    let regex_str = self.pop_string(); // the regex
                    let str = self.pop_string(); // the string
                    let regex = self.regex_cache.get(&*regex_str);
                    let is_match = regex.matches(&str);
                    self.push_bool(!is_match);
                }
                Code::Concat { count } => {
                    debug_assert!(*count >= 2);
                    let mut string = self.pop_string().downgrade_or_clone();
                    for _ in 0..count - 1 {
                        let additional = self.pop_string();
                        string.push_str(&*additional);
                    }
                    self.push_str(StringScalar::Str(string.rc()))
                }
                Code::GlobalArr(arr) => {
                    self.push_arr(*arr);
                }
                Code::ArgArray { arg_idx } => {
                    let arr = self.get_array_arg(*arg_idx as usize);
                    self.push_arr(arr);
                }
                Code::ArrayMember { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let contains = self.arrays.in_array(array.id, RcAwkStr::new(indices));
                    self.push_bool(contains)
                }
                Code::AssignRetArray { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let value = self.pop_unknown();
                    let _ = self.arrays.assign(array.id, RcAwkStr::new(indices), value.clone());
                    self.push_unknown(value);
                }
                Code::AssignRetArrayNum { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let value = self.pop_num();
                    let _ = self.arrays.assign(array.id, RcAwkStr::new(indices), RuntimeScalar::Num(value));
                    self.push_num(value);
                }
                Code::AssignRetArrayStr { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let value = self.pop_string();
                    let _ = self.arrays.assign(array.id, RcAwkStr::new(indices), value.clone().into());
                    self.push_str(value);
                }
                Code::AssignArray { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let value = self.pop_unknown();
                    let _ = self.arrays.assign(array.id, RcAwkStr::new(indices), value);
                }
                Code::AssignArrayNum { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let value = self.pop_num();
                    let _ = self.arrays.assign(array.id, RcAwkStr::new(indices), RuntimeScalar::Num(value));
                    self.push_num(value);
                }
                Code::AssignArrayStr { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let value = self.pop_string();
                    let _ = self.arrays.assign(array.id, RcAwkStr::new(indices), value.clone().into());
                    self.push_str(value);
                }
                Code::ArrayIndex { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let result = self.arrays.access(array.id, RcAwkStr::new(indices)); // TODO: Skip this Rc::new() ?
                    let value = if let Some(result) = result {
                        result.clone()
                    } else {
                        RuntimeScalar::StrNum(AwkStr::new_rc("".as_bytes().to_vec()))
                    };
                    self.push_unknown(value);
                }
                Code::Call { target } => {
                    let target = &program.functions[*target as usize];
                    let new_scope = FunctionScope {
                        unknown_stack_base_offset: self.unknown_stack.len() - target.num_scalar_args(),
                        str_stack_base_offset: self.str_stack.len(),
                        num_stack_base_offset: self.num_stack.len(),
                        array_base_offset: self.arr_stack.len() - target.num_array_args(),
                    };
                    self.scopes.push(new_scope);

                    self.run_function(target, program);

                    let return_value = self.pop_unknown();
                    let scope = self.scopes.pop().unwrap();

                    self.unknown_stack.truncate(scope.unknown_stack_base_offset); // remove args from the stack
                    self.str_stack.truncate(scope.str_stack_base_offset);
                    self.num_stack.truncate(scope.num_stack_base_offset);
                    self.arr_stack.truncate(scope.array_base_offset); // remove array args from the stack

                    self.push_unknown(return_value);
                }
                Code::Print => {
                    let value = self.pop_string();
                    self.stdout.write_all(&value).unwrap();
                    if !value.bytes().ends_with(&[10]) {
                        self.stdout.write_all(&[10]).unwrap();
                    }
                }
                Code::Printf { num_args } => {
                    // TODO: Actually call printf
                    let fstring = self.pop_string();
                    self.stdout.write_all(&fstring).unwrap();

                    for _ in 0..*num_args {
                        let s = self.pop_string();
                        self.stdout.write_all(&s).unwrap();
                    }
                }
                Code::NoOp => {} // ez-pz
                Code::Ret => return, // it seems weird but this is it
                Code::ConstLkpStr { idx } => self.push_str(function.get_const_str(*idx)),
                Code::ConstLkpNum { idx } => self.push_num(function.get_const_float(*idx)),
                Code::JumpIfFalseLbl(_) | Code::JumpLbl(_) | Code::JumpIfTrueLbl(_) | Code::Label(_) => unsafe { std::hint::unreachable_unchecked() },
                Code::RelJumpIfFalse { offset } => {
                    if *self.peek_num() == 0.0 {
                        offset_ip(&mut ip, *offset);
                        continue;
                    }
                }
                Code::RelJumpIfTrue { offset } => {
                    if *self.peek_num() != 0.0 {
                        offset_ip(&mut ip, *offset);
                        continue;
                    }
                }
                Code::RelJump { offset } => {
                    offset_ip(&mut ip, *offset);
                    continue;
                }
                Code::BuiltinAtan2 => {
                    let arg2 = self.pop_num();
                    let arg1 = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(arg1.atan2(arg2)));
                }
                Code::BuiltinCos => {
                    let arg1 = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(arg1.cos()));
                }
                Code::BuiltinExp => {
                    let arg1 = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(arg1.exp()));
                }
                Code::BuiltinSubstr2 => {
                    let start_idx = self.pop_num();
                    let string = self.pop_string();
                    let start_idx = clamp_to_slice_index(start_idx - 1.0, string.bytes().len());
                    let output = AwkStr::new_rc(string.bytes()[start_idx..].to_vec());
                    self.push_unknown(RuntimeScalar::Str(output));
                }
                Code::BuiltinSubstr3 => {
                    let max_chars = self.pop_num();
                    let start_idx = self.pop_num();
                    let string = self.pop_string();
                    let str_len = string.bytes().len();
                    let start_idx = clamp_to_slice_index(start_idx - 1.0, str_len);
                    let max_chars = clamp_to_max_len(max_chars, start_idx, str_len);
                    let awk_str = AwkStr::new_rc(string.bytes()[start_idx..start_idx + max_chars].to_vec());
                    self.push_unknown(RuntimeScalar::Str(awk_str));
                }
                Code::BuiltinIndex => {
                    let needle = self.pop_string();
                    let haystack = self.pop_string();
                    let number = if let Some(idx) = index_of(needle.bytes(), haystack.bytes()) {
                        (idx + 1) as f64
                    } else {
                        0.0
                    };
                    self.push_unknown(RuntimeScalar::Num(number));
                }
                Code::BuiltinInt => {
                    let flt = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(flt.trunc()));
                }
                Code::BuiltinLength0 => {
                    // TODO: utf8
                    // TODO: No copy here
                    let num_fields = self.columns.get(0);
                    self.push_unknown(RuntimeScalar::Num(num_fields.len() as f64));
                }
                Code::BuiltinLength1 => {
                    let s = self.pop_string();
                    self.push_unknown(RuntimeScalar::Num(s.len() as f64))
                }
                Code::BuiltinLog => {
                    let num = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(num.ln()));
                }
                Code::BuiltinRand => {
                    let rand = unsafe { libc::rand() } as f64;
                    let num = rand / libc::RAND_MAX as f64;
                    self.push_unknown(RuntimeScalar::Num(num));
                }
                Code::BuiltinSrand0 => {
                    let prior = self.srand_seed;
                    let start = SystemTime::now();
                    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap(); // TODO: Handle no time?
                    let as_float = since_the_epoch.as_secs_f64();
                    let as_int: std::os::raw::c_uint = since_the_epoch.as_secs_f64() as std::os::raw::c_uint;
                    unsafe { libc::srand(as_int) }
                    self.srand_seed = as_float;
                    self.push_unknown(RuntimeScalar::Num(prior));
                }
                Code::BuiltinSrand1 => {
                    let seed = self.pop_num();
                    let prior = self.srand_seed;
                    let seed_int = (seed % (std::os::raw::c_uint::MAX as f64)) as std::os::raw::c_uint;
                    unsafe { libc::srand(seed_int) }
                    self.srand_seed = seed;
                    self.push_unknown(RuntimeScalar::Num(prior));
                }
                Code::BuiltinSin => {
                    let num = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(num.sin()));
                }
                Code::BuiltinSplit2 => {
                    let array = self.pop_array();
                    let string = self.pop_string();
                    let mut count: f64 = 0.0;
                    let _ = self.arrays.clear(array.id);
                    for (idx, elem) in split_on_string(self.columns.get_field_sep(), &string).enumerate()
                    {
                        count += 1.0;
                        let string = AwkStr::new_rc(elem.to_vec());
                        let _ = self.arrays.assign(array.id,
                                                   AwkStr::new_rc(format!("{}", idx + 1).into_bytes()),
                                                   RuntimeScalar::StrNum(string));
                    }
                    self.push_num(count)
                }
                Code::BuiltinSplit3 => {
                    let reg_str = self.pop_string();
                    let array = self.pop_array();
                    let _ = self.arrays.clear(array.id);
                    let string = self.pop_string();
                    let reg = self.regex_cache.get(&reg_str);
                    let mut count: f64 = 0.0;
                    for (idx, elem) in split_on_regex(&reg, &string).enumerate()
                    {
                        count += 1.0;
                        let string = AwkStr::new_rc(elem.to_vec());
                        let _ = self.arrays.assign(array.id,
                                                   AwkStr::new_rc(format!("{}", idx + 1).into_bytes()),
                                                   RuntimeScalar::StrNum(string));
                    }
                    self.push_num(count);
                }
                Code::BuiltinSqrt => {
                    let num = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(num.sqrt()));
                }
                Code::BuiltinTolower => {
                    let mut str = self.pop_string().downgrade_or_clone();
                    // TODO lowercase non-ascii
                    let bytes = str.as_bytes_mut();
                    bytes.make_ascii_lowercase();
                    self.push_unknown(RuntimeScalar::Str(RcAwkStr::new(str)));
                }
                Code::BuiltinToupper => {
                    let mut str = self.pop_string().downgrade_or_clone();
                    // TODO lowercase non-ascii
                    let bytes = str.as_bytes_mut();
                    bytes.make_ascii_uppercase();
                    self.push_unknown(RuntimeScalar::Str(RcAwkStr::new(str)));
                }
                Code::Sub3 { global: _ } => {
                    let input_str = self.pop_string();
                    let replacement = self.pop_string();
                    let regex = self.pop_string();
                    let regex = self.regex_cache.get(&regex);

                    let matched = regex.match_idx(&*input_str);
                    if let Some(mtc) = matched {
                        let input_bytes = input_str.bytes();
                        let mut new_string = AwkStr::new((&input_bytes[0..mtc.start]).to_vec());
                        new_string.push_str(replacement.bytes());
                        new_string.push_str(&input_bytes[mtc.start + mtc.len..]);
                        self.push_num(1.0);
                        self.push_str(StringScalar::Str(new_string.rc()));
                    } else {
                        self.push_num(0.0);
                        self.push_str(input_str);
                    }
                }
                Code::NumToVar => {
                    let num = self.pop_num();
                    self.push_unknown(RuntimeScalar::Num(num));
                }
                Code::NumToStr => {
                    let num = self.pop_num();
                    let string = self.val_to_string(RuntimeScalar::Num(num));
                    self.push_str(StringScalar::Str(string)); // TODO: strnum?
                }
                Code::StrToVar => {
                    let str = self.pop_string();
                    self.push_unknown(str.into());
                }
                Code::StrToNum => {
                    let str = self.pop_string();
                    let num = self.str_to_num(&*str);
                    self.push_num(num);
                }
                Code::VarToNum => {
                    let var = self.pop_unknown();
                    let num = self.val_to_num(var);
                    self.push_num(num);
                }
                Code::VarToStr => {
                    let var = self.pop_unknown();
                    let str = self.val_to_string_scalar(var);
                    self.push_str(str);
                }
                Code::PopStr => {
                    self.pop_string();
                }
                Code::PopNum => {
                    self.pop_num();
                }
            }
            ip += 1;
        }
    }

    #[cfg(test)]
    fn print_stacks(&self) {
        println!("{:?} {:?} {:?} {:?}", self.unknown_stack, self.str_stack, self.num_stack, self.arr_stack)
    }
}

#[inline(always)]
fn offset_ip(ip: &mut usize, offset: i16) {
    *ip = ((*ip as isize) + offset as isize) as usize
}