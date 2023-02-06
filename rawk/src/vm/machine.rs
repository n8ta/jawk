use std::io::{Write};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::arrays::{Arrays, split_on_regex, split_on_string};
use crate::awk_str::{AwkStr, RcAwkStr};
use crate::columns::Columns;
use crate::{binop, mathop};
use crate::typing::GlobalArrayId;
use crate::vm::{Code, VmFunc, VmProgram};
use crate::vm::converter::Converter;
use crate::util::{clamp_to_max_len, clamp_to_slice_index, index_of, unwrap};
use crate::vm::regex_cache::RegexCache;
use crate::vm::runtime_scalar::{RuntimeScalar, StringScalar};
use crate::vm::vm_special_vars::{NUM_GSCALAR_SPECIALS, GlobalScalarSpecials};


pub struct FunctionScope {
    pub unknown_stack_base_offset: usize,
    pub str_stack_base_offset: usize,
    pub num_stack_base_offset: usize,
    pub array_base_offset: usize,
}


pub struct VirtualMachine {
    pub vm_program: &'static VmProgram,

    pub global_scalars: Vec<RuntimeScalar>,

    // Value stacks
    pub unknown_stack: Vec<RuntimeScalar>,
    pub num_stack: Vec<f64>,
    pub str_stack: Vec<StringScalar>,
    pub arr_stack: Vec<GlobalArrayId>,

    pub scopes: Vec<FunctionScope>,
    pub arrays: Arrays,
    pub columns: Columns,
    pub converter: Converter,
    pub regex_cache: RegexCache,

    pub stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,

    pub srand_seed: f64,
}


impl VirtualMachine {
    pub fn new(vm_program: VmProgram, files: Vec<String>, stdout: Box<dyn Write>, stderr: Box<dyn Write>) -> Self {
        unsafe { libc::srand(09171998) }
        let vm_program = Box::leak(Box::new(vm_program));
        let s = Self {
            vm_program,
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
    pub fn run(mut self) -> (Box<dyn Write>, Box<dyn Write>) {
        self.arrays.allocate(self.vm_program.analysis.global_arrays.len()); // TODO u16max
        for _ in 0..self.vm_program.analysis.global_scalars.len() {
            self.global_scalars.push(RuntimeScalar::Str(RcAwkStr::new(AwkStr::new(vec![]))));
        }
        self.run_function(self.vm_program.main());
        (self.stdout, self.stderr)
    }

    pub fn push_unknown(&mut self, scalar: RuntimeScalar) {
        self.unknown_stack.push(scalar)
    }
    pub fn push_num(&mut self, num: f64) {
        self.num_stack.push(num)
    }
    pub fn push_str(&mut self, str: StringScalar) {
        self.str_stack.push(str)
    }
    pub fn push_arr(&mut self, array_id: GlobalArrayId) {
        self.arr_stack.push(array_id)
    }
    pub fn push_bool(&mut self, b: bool) {
        self.push_num(if b { 1.0 } else { 0.0 })
    }

    pub fn pop_array(&mut self) -> GlobalArrayId {
        unwrap(self.arr_stack.pop())
    }
    pub fn pop_unknown(&mut self) -> RuntimeScalar {
        unwrap(self.unknown_stack.pop())
    }
    pub fn pop_num(&mut self) -> f64 {
        unwrap(self.num_stack.pop())
    }
    pub fn pop_string(&mut self) -> StringScalar {
        unwrap(self.str_stack.pop())
    }

    pub fn peek_unknown(&self) -> &RuntimeScalar {
        unwrap(self.unknown_stack.last())
    }
    pub fn peek_num(&self) -> &f64 {
        unwrap(self.num_stack.last())
    }
    pub fn peek_str(&self) -> &StringScalar {
        unwrap(self.str_stack.last())
    }

    pub fn set_scalar_arg(&mut self, idx: usize, value: RuntimeScalar) {
        let idx = unwrap(self.scopes.last()).unknown_stack_base_offset + idx;
        self.unknown_stack[idx] = value;
    }

    pub fn get_scalar_arg(&mut self, idx: usize) -> RuntimeScalar {
        let idx = unwrap(self.scopes.last()).unknown_stack_base_offset + idx;
        self.unknown_stack[idx].clone()
    }

    pub fn get_array_arg(&mut self, idx: usize) -> GlobalArrayId {
        let idx = unwrap(self.scopes.last()).array_base_offset + idx;
        self.arr_stack[idx].clone()
    }

    pub fn val_to_num(&mut self, value: RuntimeScalar) -> f64 {
        match value {
            RuntimeScalar::Str(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
            RuntimeScalar::StrNum(s) => self.converter.str_to_num(&*s).unwrap_or(0.0),
            RuntimeScalar::Num(n) => n,
        }
    }

    pub fn str_to_num(&mut self, s: &RcAwkStr) -> f64 {
        self.converter.str_to_num(&*s).unwrap_or(0.0)
    }

    pub fn val_to_string(&mut self, value: RuntimeScalar) -> RcAwkStr {
        match value {
            RuntimeScalar::Str(s) => s,
            RuntimeScalar::StrNum(s) => s,
            RuntimeScalar::Num(n) => AwkStr::new_rc(self.converter.num_to_str_internal(n).to_vec()),
        }
    }

    pub fn val_to_string_scalar(&mut self, value: RuntimeScalar) -> StringScalar {
        match value {
            RuntimeScalar::Str(s) => StringScalar::Str(s),
            RuntimeScalar::StrNum(s) => StringScalar::StrNum(s),
            RuntimeScalar::Num(n) => StringScalar::Str(AwkStr::new_rc(self.converter.num_to_str_internal(n).to_vec())),
        }
    }


    pub fn val_is_numeric(&mut self, value: &RuntimeScalar) -> bool {
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

    pub fn concat_array_indices(&mut self, count: usize) -> AwkStr {
        let subsep = self.global_scalars[GlobalScalarSpecials::SUBSEP as usize].clone();
        let subsep = self.val_to_string(subsep);
        let mut string = self.pop_string().downgrade_or_clone();
        for _ in 0..count - 1 {
            let addition = self.pop_string();
            string.push_str(&subsep);
            string.push_str(&*addition);
        }
        string
    }

    pub fn run_function(&mut self, function: &VmFunc) {
        let mut ip = 0;

        loop {
            #[cfg(test)]
            {
                // Coloring makes it easier to match up scalar stack and array stack visually when debugging
                print!("{} ip {:2} {} ", function.name(), ip, function.chunk()[ip].pretty_print_owned());
                self.print_stacks();
            }
            ip = (function[ip].code)(self, ip, function[ip].imm);
            if ip == usize::MAX {
                break
            }
        }
    }

    #[cfg(test)]
    pub fn print_stacks(&self) {
        println!("{:?} {:?} {:?} {:?}", self.unknown_stack, self.str_stack, self.num_stack, self.arr_stack)
    }
}