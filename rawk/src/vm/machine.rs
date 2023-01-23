use std::io::{Write};
use std::ops::{Add, Sub};
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
use crate::vm::runtime_scalar::RuntimeScalar;
use crate::vm::vm_special_vars::{NUM_GSCALAR_SPECIALS, GlobalScalarSpecials};


struct FunctionScope {
    // Scalar args start here on scalar_stack
    scalar_base_offset: usize,
    // Array args start here on array_stack
    array_base_offset: usize,
}

pub struct VirtualMachine<'a, OutT: Write, ErrT: Write> {
    global_scalars: Vec<RuntimeScalar>,

    //array stack
    arrs: Vec<GlobalArrayId>,

    //scalar stack
    ss: Vec<RuntimeScalar>,

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
            arrs: vec![],
            ss: vec![],
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
    fn push(&mut self, scalar: RuntimeScalar) {
        self.ss.push(scalar)
    }
    fn push_arr(&mut self, array_id: GlobalArrayId) {
        self.arrs.push(array_id)
    }
    fn push_bool(&mut self, b: bool) {
        if b { self.push(RuntimeScalar::Num(1.0)) } else { self.push(RuntimeScalar::Num(0.0)) }
    }

    fn pop_array(&mut self) -> GlobalArrayId {
        if let Some(popped) = self.arrs.pop() {
            popped
        } else {
            panic!("Compiler bug missing stack value")
        }
    }

    fn pop_scalar(&mut self) -> RuntimeScalar {
        if let Some(popped) = self.ss.pop() {
            popped
        } else {
            panic!("Compiler bug missing stack value")
        }
    }

    fn pop_to_number(&mut self) -> f64 {
        let scalar = self.pop_scalar();
        self.val_to_num(scalar)
    }

    fn pop_to_string(&mut self) -> RcAwkStr {
        let scalar = self.pop_scalar();
        self.val_to_string(scalar)
    }

    fn peek_scalar(&self) -> &RuntimeScalar {
        self.ss.last().unwrap()
    }

    fn set_scalar_arg(&mut self, idx: usize, value: RuntimeScalar, ) {
        let idx = self.scopes.last().unwrap().scalar_base_offset + idx;
        self.ss[idx] = value;
    }

    fn get_scalar_arg(&mut self, idx: usize) -> RuntimeScalar {
        let idx = self.scopes.last().unwrap().scalar_base_offset + idx;
        self.ss[idx].clone()
    }

    fn get_array_arg(&mut self, idx: usize) -> GlobalArrayId {
        let idx = self.scopes.last().unwrap().array_base_offset + idx;
        self.arrs[idx].clone()
    }

    fn val_to_num(&mut self, value: RuntimeScalar) -> f64 {
        match value {
            RuntimeScalar::Str(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
            RuntimeScalar::StrNum(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
            RuntimeScalar::Num(n) => n,
        }
    }

    fn val_to_string(&mut self, value: RuntimeScalar) -> RcAwkStr {
        match value {
            RuntimeScalar::Str(s) => s,
            RuntimeScalar::StrNum(s) => s,
            RuntimeScalar::Num(n) => AwkStr::new_rc(self.converter.num_to_str_internal(n).to_vec()),
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

        let string = self.pop_scalar();
        let mut string = self.val_to_string(string).downgrade_or_clone();
        for _ in 0..count - 1 {
            let addition = self.pop_scalar();
            let addition = self.val_to_string(addition);
            string.push_str(&subsep);
            string.push_str(&*addition);
        }
        string
    }

    fn run_function(&mut self, function: &VmFunc, program: &VmProgram) {
        let mut ip = 0;

        loop {
            let code = &function[ip];
            #[cfg(test)]
            {
                // Coloring makes it easier to match up scalar stack and array stack visually when debugging
                let red = "\x1b[0;31m";let yellow = "\x1b[0;33m";let end = "\x1b[0m";
                println!("ip {:2} {} {}ss:{:?}{}\n                                                         {}as:{:?}{}", ip, code.pretty_print_owned(), red, self.ss, end, yellow, self.arrs, end);
            }

            match code {
                Code::FloatZero => self.push(RuntimeScalar::Num(0.0)),
                Code::FloatOne => self.push(RuntimeScalar::Num(1.0)),
                Code::Pop => { self.ss.pop().unwrap(); }
                Code::Column => {
                    let index = self.pop_scalar();
                    let idx = self.val_to_num(index);
                    let idx = idx.round() as usize;
                    let field = self.columns.get(idx);
                    self.push(RuntimeScalar::StrNum(RcAwkStr::new(field)));
                }
                Code::NextLine => {
                    let more_lines = self.columns.next_line().unwrap(); // TODO: no unwrap
                    self.push_bool(more_lines);
                }
                Code::GSclAssign(idx) => {
                    let scalar = self.pop_scalar();
                    self.global_scalars[idx.id as usize] = scalar.clone();
                    self.push(scalar);
                }
                Code::GScl(idx) => {
                    self.push(self.global_scalars[idx.id as usize].clone())
                }
                Code::ArgSclAsgn { arg_idx } => {
                    let new_value = self.pop_scalar();
                    self.set_scalar_arg(*arg_idx as usize, new_value.clone());
                    self.push(new_value);
                }
                Code::ArgScl { arg_idx } => {
                    let arg = self.get_scalar_arg(*arg_idx as usize);
                    self.push(arg);
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
                    let rhs = self.pop_scalar(); // the regex
                    let lhs = self.pop_scalar(); // the string
                    let rhs = self.val_to_string(rhs);
                    let lhs = self.val_to_string(lhs);
                    let regex = self.regex_cache.get(&*rhs);
                    let is_match = regex.matches(&lhs);
                    self.push_bool(is_match);
                }
                Code::NMatches => {
                    let rhs = self.pop_scalar(); // the regex
                    let lhs = self.pop_scalar(); // the string
                    let rhs = self.val_to_string(rhs);
                    let lhs = self.val_to_string(lhs);
                    let regex = self.regex_cache.get(&*rhs);
                    let is_match = regex.matches(&lhs);
                    self.push_bool(!is_match);
                }
                Code::Concat { count } => {
                    debug_assert!(*count >= 2);
                    let string = self.pop_scalar();
                    let mut string = self.val_to_string(string).downgrade_or_clone();
                    for _ in 0..count - 1 {
                        let addition = self.pop_scalar();
                        let addition = self.val_to_string(addition);
                        string.push_str(&*addition);
                    }
                    self.push(RuntimeScalar::Str(RcAwkStr::new(string)));
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
                Code::ArrayAssign { indices: num_indices } => {
                    let indices = self.concat_array_indices(*num_indices);
                    let array = self.pop_array();
                    let value = self.pop_scalar();
                    let _ = self.arrays.assign(array.id, RcAwkStr::new(indices), value.clone());
                    self.push(value);
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
                    self.push(value);
                }
                Code::Call { target } => {
                    let target = &program.functions[*target as usize];
                    let new_scope = FunctionScope {
                        scalar_base_offset: self.ss.len() - target.num_scalar_args(),
                        array_base_offset: self.arrs.len() - target.num_array_args(),
                    };
                    self.scopes.push(new_scope);

                    self.run_function(target, program);

                    let return_value = self.pop_scalar();
                    let scope = self.scopes.pop().unwrap();

                    self.ss.truncate(scope.scalar_base_offset); // remove args from the stack
                    self.arrs.truncate(scope.array_base_offset);

                    self.push(return_value);
                }
                Code::Print => {
                    let value = self.pop_scalar();
                    let str = match value {
                        RuntimeScalar::Str(s) => s,
                        RuntimeScalar::StrNum(s) => s,
                        RuntimeScalar::Num(num) => {
                            let bytes = self.converter.num_to_str_output(num);
                            self.stdout.write_all(bytes).unwrap();
                            self.stdout.write_all("\n".as_bytes()).unwrap();
                            ip += 1;
                            continue;
                        }
                    };
                    if str.bytes().ends_with(&[10]) {
                        self.stdout.write_all(&str).unwrap();
                    } else {
                        self.stdout.write_all(&str).unwrap();
                        self.stdout.write_all(&[10]).unwrap();
                    }
                }
                Code::Printf { num_args } => {
                    // TODO: Actually call printf
                    let fstring = self.pop_to_string();
                    self.stdout.write_all(&fstring).unwrap();

                    for _ in 0..*num_args {
                        let s = self.pop_to_string();
                        self.stdout.write_all(&s).unwrap();
                    }
                }
                Code::NoOp => {} // ez-pz
                Code::Ret => {
                    return;
                }, // it seems weird but this is it
                Code::ConstLkp { idx } => self.push(function.get_const_from_idx(*idx)),
                Code::JumpIfFalseLbl(_) | Code::JumpLbl(_) | Code::JumpIfTrueLbl(_) | Code::Label(_) => {
                    let bytes = "compiler bug: jump to labels should be removed before VM".as_bytes();
                    self.stderr.write_all(bytes).unwrap();
                    panic!("compiler bug: jump to labels should be removed before VM")
                }
                Code::RelJumpIfFalse { offset } => {
                    if !self.peek_scalar().truthy() {
                        offset_ip(&mut ip, *offset);
                        continue;
                    }
                }
                Code::RelJumpIfTrue { offset } => {
                    if self.peek_scalar().truthy() {
                        offset_ip(&mut ip, *offset);
                        continue;
                    }
                }
                Code::RelJump { offset } => {
                    offset_ip(&mut ip, *offset);
                    continue;
                }
                Code::BuiltinAtan2 => {
                    let arg2 = self.pop_to_number();
                    let arg1 = self.pop_to_number();
                    self.push(RuntimeScalar::Num(arg1.atan2(arg2)));
                }
                Code::BuiltinCos => {
                    let arg1 = self.pop_to_number();
                    self.push(RuntimeScalar::Num(arg1.cos()));
                }
                Code::BuiltinExp => {
                    let arg1 = self.pop_to_number();
                    self.push(RuntimeScalar::Num(arg1.exp()));
                }
                Code::BuiltinSubstr2 => {
                    let start_idx = self.pop_to_number();
                    let string = self.pop_to_string();
                    let start_idx = clamp_to_slice_index(start_idx - 1.0, string.bytes().len());
                    let output = AwkStr::new_rc(string.bytes()[start_idx..].to_vec());
                    self.push(RuntimeScalar::Str(output));
                }
                Code::BuiltinSubstr3 => {
                    let max_chars = self.pop_to_number();
                    let start_idx = self.pop_to_number();
                    let string = self.pop_to_string();
                    let str_len = string.bytes().len();
                    let start_idx = clamp_to_slice_index(start_idx - 1.0, str_len);
                    let max_chars = clamp_to_max_len(max_chars, start_idx, str_len);
                    let awk_str = AwkStr::new_rc(string.bytes()[start_idx..start_idx + max_chars].to_vec());
                    self.push(RuntimeScalar::Str(awk_str));
                }
                Code::BuiltinIndex => {
                    let needle = self.pop_to_string();
                    let haystack = self.pop_to_string();
                    let number = if let Some(idx) = index_of(needle.bytes(), haystack.bytes()) {
                        (idx + 1) as f64
                    } else {
                        0.0
                    };
                    self.push(RuntimeScalar::Num(number));
                }
                Code::BuiltinInt => {
                    let flt = self.pop_to_number();
                    self.push(RuntimeScalar::Num(flt.trunc()));
                }
                Code::BuiltinLength0 => {
                    // TODO: utf8
                    // TODO: No copy here
                    let num_fields = self.columns.get(0);
                    self.push(RuntimeScalar::Num(num_fields.len() as f64));
                }
                Code::BuiltinLength1 => {
                    let s = self.pop_to_string();
                    self.push(RuntimeScalar::Num(s.len() as f64))
                }
                Code::BuiltinLog => {
                    let num = self.pop_to_number();
                    self.push(RuntimeScalar::Num(num.ln()));
                }
                Code::BuiltinRand => {
                    let rand = unsafe { libc::rand() } as f64;
                    let num = rand / libc::RAND_MAX as f64;
                    self.push(RuntimeScalar::Num(num));
                }
                Code::BuiltinSrand0 => {
                    let prior = self.srand_seed;
                    let start = SystemTime::now();
                    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap(); // TODO: Handle no time?
                    let as_float = since_the_epoch.as_secs_f64();
                    let as_int: std::os::raw::c_uint = since_the_epoch.as_secs_f64() as std::os::raw::c_uint;
                    unsafe { libc::srand(as_int) }
                    self.srand_seed = as_float;
                    self.push(RuntimeScalar::Num(prior));
                }
                Code::BuiltinSrand1 => {
                    let seed = self.pop_to_number();
                    let prior = self.srand_seed;
                    let seed_int = (seed % (std::os::raw::c_uint::MAX as f64)) as std::os::raw::c_uint;
                    unsafe { libc::srand(seed_int) }
                    self.srand_seed = seed;
                    self.push(RuntimeScalar::Num(prior));
                }
                Code::BuiltinSin => {
                    let num = self.pop_to_number();
                    self.push(RuntimeScalar::Num(num.sin()));
                }
                Code::BuiltinSplit2 => {
                    let array = self.pop_array();
                    let string = self.pop_to_string();
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
                    self.push(RuntimeScalar::Num(count));
                }
                Code::BuiltinSplit3 => {
                    let reg_str = self.pop_to_string();
                    let array = self.pop_array();
                    let _ = self.arrays.clear(array.id);
                    let string = self.pop_to_string();
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
                    self.push(RuntimeScalar::Num(count));
                }
                Code::BuiltinSqrt => {
                    let num = self.pop_to_number();
                    self.push(RuntimeScalar::Num(num.sqrt()));
                }
                Code::BuiltinTolower => {
                    let mut str = self.pop_to_string().downgrade_or_clone();
                    // TODO lowercase non-ascii
                    let bytes = str.as_bytes_mut();
                    bytes.make_ascii_lowercase();
                    self.push(RuntimeScalar::Str(RcAwkStr::new(str)));
                }
                Code::BuiltinToupper => {
                    let mut str = self.pop_to_string().downgrade_or_clone();
                    // TODO lowercase non-ascii
                    let bytes = str.as_bytes_mut();
                    bytes.make_ascii_uppercase();
                    self.push(RuntimeScalar::Str(RcAwkStr::new(str)));
                }
                Code::Sub { global: _ } => {
                    let input_str = self.pop_to_string();
                    let replacement = self.pop_to_string();
                    let regex = self.pop_to_string();
                    let regex = self.regex_cache.get(&regex);

                    let matched = regex.match_idx(&*input_str);
                    if let Some(mtc) = matched {
                        let input_bytes = input_str.bytes();
                        let mut new_string = AwkStr::new((&input_bytes[0..mtc.start]).to_vec());
                        new_string.push_str(replacement.bytes());
                        new_string.push_str(&input_bytes[mtc.start + mtc.len..]);
                        self.push(RuntimeScalar::Num(1.0));
                        self.push(RuntimeScalar::Str(new_string.rc()));
                    } else {
                        self.push(RuntimeScalar::Num(0.0));
                        self.push(RuntimeScalar::Str(input_str));
                    }
                }
            }
            ip += 1;
        }
    }
}

fn offset_ip(ip: &mut usize, offset: i16) {
    if offset > 0 {
        *ip = ip.add(offset as usize)
    } else {
        *ip = ip.sub((-offset) as usize)
    }
}