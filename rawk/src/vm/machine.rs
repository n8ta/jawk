use std::io::{Write};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::awk_str::{AwkStr, RcAwkStr};
use crate::runtime::arrays::{Arrays, split_on_regex, split_on_string};
use crate::runtime::columns::Columns;
use crate::runtime::rc_manager::RcManager;
use crate::runtime::regex_cache::RegexCache;
use crate::runtime::converter::Converter;
use crate::{binop, mathop};
use crate::parser::{SclSpecial};
use crate::typing::{GlobalArrayId, GlobalScalarId};
use crate::vm::{Code, VmFunc, VmProgram};
use crate::util::{clamp_to_max_len, clamp_to_slice_index, index_of, unwrap};
use crate::vm::runtime_scalar::{RuntimeScalar, StringScalar};


pub struct FunctionScope {
    pub unknown_stack_base_offset: usize,
    pub str_stack_base_offset: usize,
    pub num_stack_base_offset: usize,
    pub array_base_offset: usize,
}


pub struct VirtualMachine {
    pub vm_program: &'static VmProgram, // Just leak it so we don't need to litter the program with runtimes.

    pub global_scalars: Vec<RuntimeScalar>,
    pub special_scalars: Vec<SclSpecial>,

    // Distributes and recycles awk strings, saves lots of malloc'ing by reusing Rc's.
    pub shitty_malloc: RcManager,

    // Value stacks
    pub unknown_stack: Vec<RuntimeScalar>,
    pub num_stack: Vec<f64>,
    pub str_stack: Vec<StringScalar>,
    pub arr_stack: Vec<GlobalArrayId>,

    // Scopes
    pub scopes: Vec<FunctionScope>,

    // Runtime modules managing various piece of state
    pub arrays: Arrays,
    pub columns: Columns,
    pub converter: Converter,
    pub regex_cache: RegexCache,
    pub srand_seed: f64,

    // IO
    pub stdout: Box<dyn Write>,
    stderr: Box<dyn Write>,
}


impl VirtualMachine {
    pub fn new(vm_program: VmProgram, files: Vec<String>, stdout: Box<dyn Write>, stderr: Box<dyn Write>) -> Self {
        unsafe { libc::srand(09171998) }
        let vm_program = Box::leak(Box::new(vm_program));

        let num_gscls = vm_program.analysis.global_scalars.len();
        let mut global_scalars = Vec::with_capacity(num_gscls);
        for _ in 0..num_gscls {
            global_scalars.push(RuntimeScalar::Str(RcAwkStr::new_bytes(vec![])));
        }

        let mut special_scalars = vec![];

        let s = Self {
            vm_program,
            global_scalars,
            special_scalars,
            shitty_malloc: RcManager::new(),
            unknown_stack: vec![],
            num_stack: vec![],
            str_stack: vec![],
            arr_stack: vec![],
            scopes: vec![],
            arrays: Arrays::new(vm_program.analysis.global_arrays.len()),
            columns: Columns::new(files),
            converter: Converter::new(),
            regex_cache: RegexCache::new(),
            srand_seed: 09171998.0,
            stdout,
            stderr,
        };
        s
    }
    pub fn run(mut self) -> (Box<dyn Write>, Box<dyn Write>) {
        self.run_function(self.vm_program.main());
        (self.stdout, self.stderr)
    }

    pub fn gscl(&mut self, idx: GlobalScalarId) -> &RuntimeScalar {
        unwrap(self.global_scalars.get(idx.id))
    }

    pub fn assign_gscl(&mut self, idx: GlobalScalarId, value: RuntimeScalar) {
        let existing = unwrap(self.global_scalars.get_mut(idx.id));
        let prior_value = std::mem::replace(existing, value);
        self.shitty_malloc.drop_scalar(prior_value)
    }


    pub fn special(&mut self, special: SclSpecial) -> RuntimeScalar {
        todo!("read special")
    }

    pub fn assign_special(&mut self, special: SclSpecial, value: RuntimeScalar) {
        todo!("assign special");
        // let existing = unwrap(self.global_scalars.get_mut(idx as usize));
        // let prior_value = std::mem::replace(existing,  scalar);
        // self.shitty_malloc.drop_scalar(prior_value)
    }

    pub fn push_unknown(&mut self, scalar: RuntimeScalar) { self.unknown_stack.push(scalar) }
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
            RuntimeScalar::Num(n) => self.shitty_malloc.copy_from_slice(self.converter.num_to_str_internal(n)).rc(),
        }
    }

    pub fn val_to_string_scalar(&mut self, value: RuntimeScalar) -> StringScalar {
        match value {
            RuntimeScalar::Str(s) => StringScalar::Str(s),
            RuntimeScalar::StrNum(s) => StringScalar::StrNum(s),
            RuntimeScalar::Num(n) => StringScalar::Str(self.shitty_malloc.copy_from_slice(self.converter.num_to_str_internal(n)).rc())
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
        todo!("concat hook up subsep");
        let subsep = RuntimeScalar::Str(RcAwkStr::new_bytes("-".as_bytes().to_vec()));
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