use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::{binop, binop_num_only, mathop};
use crate::arrays::{split_on_regex, split_on_string};
use crate::awk_str::{AwkStr, RcAwkStr};
use crate::util::{clamp_to_max_len, clamp_to_slice_index, index_of};
use crate::vm::bytecode::code_and_immed::Immed;
use crate::vm::{RuntimeScalar, StringScalar, VirtualMachine};
use crate::vm::machine::FunctionScope;

pub fn num_to_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = vm.pop_num();
    vm.push_unknown(RuntimeScalar::Num(num));
    ip + 1
}

pub fn num_to_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = vm.pop_num();
    let string = vm.val_to_string(RuntimeScalar::Num(num));
    vm.push_str(StringScalar::Str(string)); // TODO: strnum?
    ip + 1
}

pub fn str_to_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let str = vm.pop_string();
    vm.push_unknown(str.into());
    ip + 1
}

pub fn str_to_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let str = vm.pop_string();
    let num = vm.str_to_num(&*str);
    vm.push_num(num);
    ip + 1
}

pub fn var_to_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let var = vm.pop_unknown();
    let num = vm.val_to_num(var);
    vm.push_num(num);
    ip + 1
}

pub fn var_to_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let var = vm.pop_unknown();
    let str = vm.val_to_string_scalar(var);
    vm.push_str(str);
    ip + 1
}

pub fn pop(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    vm.pop_unknown();
    ip + 1
}

pub fn pop_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    vm.pop_string();
    ip + 1
}

pub fn pop_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    vm.pop_num();
    ip + 1
}

pub fn column(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let index = vm.pop_num();
    let idx = index.round() as usize;
    let field = vm.columns.get(idx);
    vm.push_str(StringScalar::StrNum(field.rc()));
    ip + 1
}

pub fn next_line(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let more_lines = vm.columns.next_line().unwrap();
    vm.push_bool(more_lines);
    ip + 1
}

pub fn assign_gscl_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let scalar = vm.pop_unknown();
    vm.global_scalars[unsafe { imm.global_scl_id }.id] = scalar;
    ip + 1
}

pub fn assign_gscl_ret_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let scalar = vm.pop_unknown();
    vm.global_scalars[unsafe { imm.global_scl_id }.id] = scalar.clone();
    vm.push_unknown(scalar);
    ip + 1
}

pub fn assign_gscl_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = vm.pop_num();
    vm.global_scalars[unsafe { imm.global_scl_id }.id] = RuntimeScalar::Num(num);
    ip + 1
}

pub fn assign_gscl_ret_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = vm.pop_num();
    vm.global_scalars[unsafe { imm.global_scl_id }.id] = RuntimeScalar::Num(num);
    vm.push_num(num);
    ip + 1
}

pub fn assign_gscl_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let str: RuntimeScalar = vm.pop_string().into();
    vm.global_scalars[unsafe { imm.global_scl_id }.id] = str;
    ip + 1
}

pub fn assign_gscl_ret_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let str: RuntimeScalar = vm.pop_string().into();
    vm.global_scalars[unsafe { imm.global_scl_id }.id] = str.clone();
    vm.push_unknown(str);
    ip + 1
}

pub fn global_arr(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    vm.push_arr(unsafe { imm.global_arr_id });
    ip + 1
}

pub fn gscl_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    vm.push_unknown(vm.global_scalars[unsafe { imm.global_scl_id }.id].clone());
    ip + 1
}

pub fn gscl_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let scl = vm.global_scalars[unsafe { imm.global_scl_id }.id].clone();
    let num = match scl {
        RuntimeScalar::Str(_) => unsafe { std::hint::unreachable_unchecked() },
        RuntimeScalar::StrNum(_) => unsafe { std::hint::unreachable_unchecked() },
        RuntimeScalar::Num(num) => num,
    };
    vm.push_num(num);
    ip + 1
}

pub fn gscl_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let scl = vm.global_scalars[unsafe { imm.global_scl_id }.id].clone();
    let str = vm.val_to_string_scalar(scl);
    vm.push_str(str);
    ip + 1
}

pub fn assign_arg_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let new_value = vm.pop_unknown();
    vm.set_scalar_arg(arg_idx, new_value);
    ip + 1
}

pub fn assign_arg_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let new_value = vm.pop_string();
    vm.set_scalar_arg(arg_idx, new_value.into());
    ip + 1
}

pub fn assign_arg_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let new_value = vm.pop_num();
    vm.set_scalar_arg(arg_idx, RuntimeScalar::Num(new_value));
    ip + 1
}

pub fn assign_arg_ret_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let new_value = vm.pop_unknown();
    vm.set_scalar_arg(arg_idx, new_value.clone());
    vm.push_unknown(new_value);
    ip + 1
}

pub fn assign_arg_ret_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let new_value = vm.pop_string();
    let new_value_clone: StringScalar = new_value.clone();
    let new_value_rt: RuntimeScalar = new_value_clone.into();
    vm.set_scalar_arg(arg_idx, new_value_rt);
    vm.push_str(new_value);
    ip + 1
}

pub fn assign_arg_ret_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let new_value = vm.pop_num();
    vm.set_scalar_arg(arg_idx, RuntimeScalar::Num(new_value));
    vm.push_num(new_value);
    ip + 1
}

pub fn arg_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let arg = vm.get_scalar_arg(arg_idx);
    vm.push_unknown(arg);
    ip + 1
}

pub fn arg_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let arg = vm.get_scalar_arg(arg_idx);
    let arg = vm.val_to_string_scalar(arg);
    vm.push_str(arg);
    ip + 1
}

pub fn arg_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let arg = vm.get_scalar_arg(arg_idx);
    let arg = vm.val_to_num(arg);
    vm.push_num(arg);
    ip + 1
}

pub fn arg_arr(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg_idx = unsafe { imm.arg_idx };
    let arr = vm.get_array_arg(arg_idx);
    vm.push_arr(arr);
    ip + 1
}

mathop!(exp, crate::vm::bytecode::subroutine_helpers::exp);
mathop!(mult, crate::vm::bytecode::subroutine_helpers::mult);
mathop!(div, crate::vm::bytecode::subroutine_helpers::div);
mathop!(modulo, crate::vm::bytecode::subroutine_helpers::modulo);
mathop!(add, crate::vm::bytecode::subroutine_helpers::add);
mathop!(minus, crate::vm::bytecode::subroutine_helpers::minus);

binop!(lt, crate::vm::bytecode::subroutine_helpers::lt);
binop!(gt, crate::vm::bytecode::subroutine_helpers::gt);
binop!(lteq, crate::vm::bytecode::subroutine_helpers::lteq);
binop!(gteq, crate::vm::bytecode::subroutine_helpers::gteq);
binop!(eqeq, crate::vm::bytecode::subroutine_helpers::eq);
binop!(neq, crate::vm::bytecode::subroutine_helpers::neq);

binop_num_only!(lt_num, crate::vm::bytecode::subroutine_helpers::lt);
binop_num_only!(gt_num, crate::vm::bytecode::subroutine_helpers::gt);
binop_num_only!(lteq_num, crate::vm::bytecode::subroutine_helpers::lteq);
binop_num_only!(gteq_num, crate::vm::bytecode::subroutine_helpers::gteq);
binop_num_only!(eqeq_num, crate::vm::bytecode::subroutine_helpers::eq);
binop_num_only!(neq_num, crate::vm::bytecode::subroutine_helpers::neq);


pub fn matches(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let regex_str = vm.pop_string(); // the regex
    let str = vm.pop_string(); // the string
    let regex = vm.regex_cache.get(&*regex_str);
    let is_match = regex.matches(&str);
    vm.push_bool(is_match);
    ip + 1
}

pub fn nmatches(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let regex_str = vm.pop_string(); // the regex
    let str = vm.pop_string(); // the string
    let regex = vm.regex_cache.get(&*regex_str);
    let is_match = regex.matches(&str);
    vm.push_bool(!is_match);
    ip + 1
}
pub fn assign_array_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let value = vm.pop_unknown();
    let _ = vm.arrays.assign(array.id, RcAwkStr::new(indices), value);
    ip + 1
}
pub fn assign_array_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let value = vm.pop_string();
    let _ = vm.arrays.assign(array.id, RcAwkStr::new(indices), value.clone().into());
    ip + 1
}
pub fn assign_array_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let value = vm.pop_num();
    let _ = vm.arrays.assign(array.id, RcAwkStr::new(indices), RuntimeScalar::Num(value));
    ip + 1
}
pub fn assign_array_ret_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let value = vm.pop_unknown();
    let _ = vm.arrays.assign(array.id, RcAwkStr::new(indices), value.clone());
    vm.push_unknown(value);
    ip + 1
}
pub fn assign_array_ret_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let value = vm.pop_string();
    let _ = vm.arrays.assign(array.id, RcAwkStr::new(indices), value.clone().into());
    vm.push_str(value);
    ip + 1
}
pub fn assign_array_ret_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let value = vm.pop_num();
    let _ = vm.arrays.assign(array.id, RcAwkStr::new(indices), RuntimeScalar::Num(value));
    vm.push_num(value);
    ip + 1
}

pub fn array_member(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let contains = vm.arrays.in_array(array.id, RcAwkStr::new(indices));
    vm.push_bool(contains);
    ip + 1
}
pub fn array_index(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_indices = unsafe { imm.array_indices };
    let indices = vm.concat_array_indices(num_indices);
    let array = vm.pop_array();
    let result = vm.arrays.access(array.id, RcAwkStr::new(indices)); // TODO: Skip this Rc::new() ?
    let value = if let Some(result) = result {
        result.clone()
    } else {
        RuntimeScalar::StrNum(RcAwkStr::new_bytes("".as_bytes().to_vec()))
    };
    vm.push_unknown(value);
    ip + 1
}


pub fn concat(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let count = unsafe { imm.concat_count };
    debug_assert!(count >= 2);
    let mut string = vm.pop_string().downgrade_or_clone();
    for _ in 0..count - 1 {
        let additional = vm.pop_string();
        string.push_str(&*additional);
    }
    vm.push_str(StringScalar::Str(string.rc()));
    ip + 1
}

pub fn builtin_atan2(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg2 = vm.pop_num();
    let arg1 = vm.pop_num();
    vm.push_num(arg1.atan2(arg2));
    ip + 1
}
pub fn builtin_cos(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg1 = vm.pop_num();
    vm.push_num(arg1.cos());
    ip + 1
}
pub fn builtin_exp(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let arg1 = vm.pop_num();
    vm.push_num(arg1.exp());
    ip + 1
}
pub fn builtin_substr2(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let start_idx = vm.pop_num();
    let string = vm.pop_string();
    let start_idx = clamp_to_slice_index(start_idx - 1.0, string.bytes().len());
    let output = AwkStr::new_rc(string.bytes()[start_idx..].to_vec());
    vm.push_str(StringScalar::Str(output));
    ip + 1
}
pub fn builtin_substr3(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let max_chars = vm.pop_num();
    let start_idx = vm.pop_num();
    let string = vm.pop_string();
    let str_len = string.bytes().len();
    let start_idx = clamp_to_slice_index(start_idx - 1.0, str_len);
    let max_chars = clamp_to_max_len(max_chars, start_idx, str_len);
    let awk_str = AwkStr::new_rc(string.bytes()[start_idx..start_idx + max_chars].to_vec());
    vm.push_str(StringScalar::Str(awk_str));
    ip + 1
}
pub fn builtin_index(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let needle = vm.pop_string();
    let haystack = vm.pop_string();
    let number = if let Some(idx) = index_of(needle.bytes(), haystack.bytes()) {
        (idx + 1) as f64
    } else {
        0.0
    };
    vm.push_num(number);
    ip + 1
}
pub fn builtin_int(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let flt = vm.pop_num();
    vm.push_num(flt.trunc());
    ip + 1
}
pub fn builtin_length0(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num_fields = vm.columns.get(0);
    vm.push_num(num_fields.len() as f64);
    ip + 1
}
pub fn builtin_length1(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let s = vm.pop_string();
    vm.push_num(s.len() as f64);
    ip + 1
}
pub fn builtin_log(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = vm.pop_num();
    vm.push_num(num.ln());
    ip + 1
}
pub fn builtin_rand(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let rand = unsafe { libc::rand() } as f64;
    let num = rand / libc::RAND_MAX as f64;
    vm.push_num(num);
    ip + 1
}
pub fn builtin_sin(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = vm.pop_num();
    vm.push_num(num.sin());
    ip + 1
}
pub fn builtin_split2(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let array = vm.pop_array();
    let string = vm.pop_string();
    let mut count: f64 = 0.0;
    let _ = vm.arrays.clear(array.id);
    for (idx, elem) in split_on_string(vm.columns.get_field_sep(), &string).enumerate()
    {
        count += 1.0;
        let string = AwkStr::new_rc(elem.to_vec());
        let _ = vm.arrays.assign(array.id,
                                 AwkStr::new_rc(format!("{}", idx + 1).into_bytes()),
                                 RuntimeScalar::StrNum(string));
    }
    vm.push_num(count);
    ip + 1
}
pub fn builtin_split3(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let reg_str = vm.pop_string();
    let array = vm.pop_array();
    let _ = vm.arrays.clear(array.id);
    let string = vm.pop_string();
    let reg = vm.regex_cache.get(&reg_str);
    let mut count: f64 = 0.0;
    for (idx, elem) in split_on_regex(&reg, &string).enumerate()
    {
        count += 1.0;
        let string = AwkStr::new_rc(elem.to_vec());
        let _ = vm.arrays.assign(array.id,
                                 AwkStr::new_rc(format!("{}", idx + 1).into_bytes()),
                                 RuntimeScalar::StrNum(string));
    }
    vm.push_num(count);
    ip + 1
}
pub fn builtin_sqrt(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = vm.pop_num();
    vm.push_num(num.sqrt());
    ip + 1
}
pub fn builtin_srand0(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let prior = vm.srand_seed;
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap(); // TODO: Handle no time?
    let as_float = since_the_epoch.as_secs_f64();
    let as_int: std::os::raw::c_uint = since_the_epoch.as_secs_f64() as std::os::raw::c_uint;
    unsafe { libc::srand(as_int) }
    vm.srand_seed = as_float;
    vm.push_num(prior);
    ip + 1
}
pub fn builtin_srand1(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let seed = vm.pop_num();
    let prior = vm.srand_seed;
    let seed_int = (seed % (std::os::raw::c_uint::MAX as f64)) as std::os::raw::c_uint;
    unsafe { libc::srand(seed_int) }
    vm.srand_seed = seed;
    vm.push_num(prior);
    ip + 1
}
pub fn builtin_tolower(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let mut str = vm.pop_string().downgrade_or_clone();
    // TODO lowercase non-ascii
    let bytes = str.as_bytes_mut();
    bytes.make_ascii_lowercase();
    vm.push_str(StringScalar::Str(RcAwkStr::new(str)));
    ip + 1
}
pub fn builtin_toupper(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let mut str = vm.pop_string().downgrade_or_clone();
    // TODO lowercase non-ascii
    let bytes = str.as_bytes_mut();
    bytes.make_ascii_uppercase();
    vm.push_str(StringScalar::Str(RcAwkStr::new(str)));
    ip + 1
}

pub fn sub3(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    // TODO: GLOBAL?
    // TODO: Sub2

    let is_global = unsafe { imm.sub3_isglobal };
    let input_str = vm.pop_string();
    let replacement = vm.pop_string();
    let regex = vm.pop_string();
    let regex = vm.regex_cache.get(&regex);

    let matched = regex.match_idx(&*input_str);
    if let Some(mtc) = matched {
        let input_bytes = input_str.bytes();
        let mut new_string = AwkStr::new((&input_bytes[0..mtc.start]).to_vec());
        new_string.push_str(replacement.bytes());
        new_string.push_str(&input_bytes[mtc.start + mtc.len..]);
        vm.push_num(1.0);
        vm.push_str(StringScalar::Str(new_string.rc()));
    } else {
        vm.push_num(0.0);
        vm.push_str(input_str);
    }
    ip + 1
}

#[inline(always)]
fn offset_ip(ip: usize, offset: isize) -> usize {
    ((ip as isize) + offset) as usize
}

pub fn rel_jump_if_false_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let offset = unsafe { imm.offset };
    if vm.pop_unknown().truthy() {
        ip + 1
    } else {
        offset_ip(ip, offset)
    }

}
pub fn rel_jump_if_false_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let offset = unsafe { imm.offset };
    if vm.pop_string().truthy() {
        ip + 1
    } else {
        offset_ip(ip, offset)
    }

}
pub fn rel_jump_if_false_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let offset = unsafe { imm.offset };
    let popped = vm.pop_num();
    if popped == 0.0 {
        offset_ip(ip, offset)
    } else {
        ip + 1
    }
}
pub fn rel_jump_if_true_var(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let offset = unsafe { imm.offset };
    if vm.pop_unknown().truthy() {
        offset_ip(ip, offset)
    } else {
        ip + 1
    }

}
pub fn rel_jump_if_true_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let offset = unsafe { imm.offset };
    if vm.pop_string().truthy() {
        offset_ip(ip, offset)
    } else {
        ip + 1
    }

}
pub fn rel_jump_if_true_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let offset = unsafe { imm.offset };
    if vm.pop_num() != 0.0 {
        offset_ip(ip, offset)
    } else {
        ip + 1
    }
}

pub fn rel_jump(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let offset = unsafe { imm.offset };
    offset_ip(ip, offset)
}
pub fn print(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let value = vm.pop_string();
    vm.stdout.write_all(&value).unwrap();
    if !value.bytes().ends_with(&[10]) {
        vm.stdout.write_all(&[10]).unwrap();
    }
    ip + 1
}
pub fn printf(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    // TODO: Actually call printf
    let num_args = unsafe { imm.printf_args};
    let fstring = vm.pop_string();
    vm.stdout.write_all(&fstring).unwrap();

    for _ in 0..num_args {
        let s = vm.pop_string();
        vm.stdout.write_all(&s).unwrap();
    }
    ip + 1
}

pub fn noop(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    // TODO: remove no-op entirely
    ip + 1
}

pub fn ret(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    usize::MAX
}

pub fn const_str(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let str = unsafe { imm.string };
    let str = unsafe { RcAwkStr::from_raw(str) };
    vm.push_str(StringScalar::Str(str));
    ip + 1
}

pub fn const_num(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let num = unsafe { imm.num };
    vm.push_num(num);
    ip + 1
}

pub fn call(vm: &mut VirtualMachine, ip: usize, imm: Immed) -> usize {
    let target = unsafe { imm.call_target };
    let target = &vm.vm_program.functions[target as usize];
    let new_scope = FunctionScope {
        unknown_stack_base_offset: vm.unknown_stack.len() - target.num_scalar_args(),
        str_stack_base_offset: vm.str_stack.len(),
        num_stack_base_offset: vm.num_stack.len(),
        array_base_offset: vm.arr_stack.len() - target.num_array_args(),
    };
    vm.scopes.push(new_scope);

    vm.run_function(target);

    let return_value = vm.pop_unknown();
    let scope = vm.scopes.pop().unwrap();

    vm.unknown_stack.truncate(scope.unknown_stack_base_offset); // remove args from the stack
    vm.str_stack.truncate(scope.str_stack_base_offset);
    vm.num_stack.truncate(scope.num_stack_base_offset);
    vm.arr_stack.truncate(scope.array_base_offset); // remove array args from the stack

    vm.push_unknown(return_value);
    ip + 1
}





