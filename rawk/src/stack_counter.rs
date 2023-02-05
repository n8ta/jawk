use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut};
use crate::parser::ScalarType;
use crate::stackt::StackT;

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct StackCounter {
    stacks: [usize; 4],
}


impl StackCounter {
    pub fn new() -> Self {
        Self { stacks: [0, 0, 0, 0] }
    }

    pub fn set(mut self, stack: StackT, delta: usize) -> Self {
        self[stack] += delta;
        self
    }
    pub fn num(cnt: usize) -> Self {
        Self::new().set(StackT::Num, cnt)
    }
    pub fn str(cnt: usize) -> Self {
        Self::new().set(StackT::Str, cnt)
    }
    pub fn var(cnt: usize) -> Self {
        Self::new().set(StackT::Var, cnt)
    }
    pub fn arr(cnt: usize) -> Self {
        Self::new().set(StackT::Array, cnt)
    }

    pub fn count(&self, typ: StackT) -> usize {
        self[typ]
    }
    pub fn total(&self) -> usize {
        self.stacks.iter().sum()
    }

    pub fn single_scalar_return_value(&self) -> ScalarType {
        debug_assert!(self.total() == 1);
        debug_assert!(self[StackT::Array] == 0);
        for typ in &[ScalarType::Var, ScalarType::Num, ScalarType::Str] {
            let stack: StackT = (*typ).into();
            if self[stack] == 1 {
                return *typ;
            }
        }
        panic!("non-scalar return type");
    }

    pub fn make_array(&self) -> Vec<StackT> {
        let mut res = vec![];
        for variant in StackT::iter() {
            for _ in 0..self[*variant] {
                res.push(*variant);
            }
        }
        res
    }

    pub fn add(&mut self, other: &Self) {
        for var in StackT::iter() {
            self[*var] += other[*var];
        }
    }
    pub fn sub(&mut self, rm: &[StackT]) {
        for stack in rm {
            self[*stack] -= 1;
        }
    }
}
impl Index<StackT> for StackCounter {
    type Output = usize;

    fn index(&self, index: StackT) -> &Self::Output {
        &self.stacks[index as usize]
    }
}
impl IndexMut<StackT> for StackCounter {
    fn index_mut(&mut self, index: StackT) -> &mut Self::Output {
        &mut self.stacks[index as usize]
    }
}

impl Display for StackCounter  {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "var: {:1} str: {:1} num: {:1} arr: {:1}", self[StackT::Var], self[StackT::Str], self[StackT::Num], self[StackT::Array])
    }
}