#[macro_export]
macro_rules! mathop {
        ($name:ident, $operator:expr) => {
            pub fn $name(vm: &mut VirtualMachine, ip: usize, _imm: Immed) -> usize {
                let rhs = vm.pop_num();
                let lhs = vm.pop_num();
                vm.push_num($operator(lhs,rhs));
                ip + 1
            }
        };
    }

#[macro_export]
macro_rules! binop {
    ($name:ident, $operator:expr) => {
        pub fn $name(vm: &mut VirtualMachine, ip: usize, _imm: Immed) -> usize {

            let right = vm.pop_unknown();
            let left = vm.pop_unknown();
            let res = if vm.val_is_numeric(&left) && vm.val_is_numeric(&right) {
                let left = vm.val_to_num(left);
                let right = vm.val_to_num(right);
                $operator(left,right)
            } else {
                // String comparisons
                let left = vm.val_to_string(left);
                let right = vm.val_to_string(right);
                $operator(left,right)
            };
            vm.push_bool(res);
            ip + 1
        }
    }
}

// Optimized version of binop that only handles num num comparisons
#[macro_export]
macro_rules! binop_num_only {
    ($name:ident, $operator:expr) => {
        pub fn $name(vm: &mut VirtualMachine, ip: usize, _imm: Immed) -> usize {
            let right = vm.pop_num();
            let left = vm.pop_num();
            let res = $operator(left, right);
            vm.push_bool(res);
            ip + 1
        }
    }
}


#[inline(always)]
pub fn add(a: f64, b: f64) -> f64 { a + b }

#[inline(always)]
pub fn minus(a: f64, b: f64) -> f64 { a - b }

#[inline(always)]
pub fn div(a: f64, b: f64) -> f64 { a / b }

#[inline(always)]
pub fn mult(a: f64, b: f64) -> f64 { a * b }

#[inline(always)]
pub fn exp(a: f64, b: f64) -> f64 { a.powf(b) }

#[inline(always)]
pub fn modulo(a: f64, b: f64) -> f64 { a % b }

#[inline(always)]
pub fn lt<T: PartialEq + PartialOrd>(a: T, b: T) -> bool { a < b }

#[inline(always)]
pub fn gt<T: PartialEq + PartialOrd>(a: T, b: T) -> bool { a > b }

#[inline(always)]
pub fn lteq<T: PartialEq + PartialOrd>(a: T, b: T) -> bool { a <= b }

#[inline(always)]
pub fn gteq<T: PartialEq + PartialOrd>(a: T, b: T) -> bool { a >= b }

#[inline(always)]
pub fn eq<T: PartialEq + PartialOrd>(a: T, b: T) -> bool {
    a == b
}

#[inline(always)]
pub fn neq<T: PartialEq + PartialOrd>(a: T, b: T) -> bool { a != b }