#[macro_export]
macro_rules! mathop {
        ($s:expr, $operator:expr) => {
            let rhs = $s.pop_scalar();
            let lhs = $s.pop_scalar();
            let rhs = $s.val_to_num(rhs);
            let lhs = $s.val_to_num(lhs);
            $s.push(RuntimeScalar::Num($operator(lhs,rhs)));
        };
    }

#[macro_export]
macro_rules! binop {
    ($s:expr, $operator:expr) => {
        let right = $s.pop_scalar();
        let left = $s.pop_scalar();
        let res = if $s.val_is_numeric(&left) && $s.val_is_numeric(&right) {
            let left = $s.val_to_num(left);
            let right = $s.val_to_num(right);
            $operator(left,right)
        } else {
            // String comparisons
            let left = $s.val_to_string(left);
            let right = $s.val_to_string(right);
            $operator(left,right)
        };
        $s.push_bool(res);
    }
}


#[inline(always)]
pub fn add(a: f64, b: f64) -> f64 { a + b }

#[inline(always)]
pub fn sub(a: f64, b: f64) -> f64 { a - b }

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
pub fn eq<T: PartialEq + PartialOrd>(a: T, b: T) -> bool { a == b }

#[inline(always)]
pub fn neq<T: PartialEq + PartialOrd>(a: T, b: T) -> bool { a != b }