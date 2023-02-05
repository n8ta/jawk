use std::fmt::{Debug, Formatter};
use crate::stack_counter::StackCounter;
use crate::vm::{VmFunc, VmProgram};

// Validates that each function has a net +1 (return value) effect on the scalar stack
// and a net 0 effect on the array stack. Also validates stack heights are consistent
// regardless of how you reach a given ip (instruction pointer).
// Panics if program is invalid.
pub fn validate_program(prog: &VmProgram) {
    for func in &prog.functions {
        let mut validator = FunctionValidator::new(&func, &prog);
        validator.validate()
    }
}

// Records the scalar stack and array stack height at a given
// ip. This should not change based on how you reach an ip.
#[derive(Copy, Clone)]
struct StackHeights {
    ip: usize,
    stacks: StackCounter,
}

impl Debug for StackHeights {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\nip:{:2} {}", self.ip, self.stacks)
    }
}

impl PartialEq for StackHeights {
    fn eq(&self, other: &Self) -> bool {
        self.stacks == other.stacks
    }
}

struct FunctionValidator<'a> {
    // Stack height when a given ip is reached but before it executes
    stack_heights: Vec<Option<StackHeights>>,
    func: &'a VmFunc,
    prog: &'a VmProgram,
}

impl<'a> FunctionValidator<'a> {
    pub fn new(func: &'a VmFunc, prog: &'a VmProgram) -> Self {
        Self {
            stack_heights: (0..func.chunk().len()).map(|_| None).collect(),
            func,
            prog,
        }
    }
    pub fn validate(&mut self) {
        let init = vec![StackHeights { ip: 0, stacks: StackCounter::new() }];
        self.validate_rec(0, &init);
    }

    fn validate_rec(&mut self, ip: usize, history: &Vec<StackHeights>) {

        println!("ip: {}", ip);
        // If we've been at this ip before make sure height match
        let stack_heights = history.last().unwrap();
        if let Some(existing) = self.stack_heights[ip] {
            assert_eq!(existing, *stack_heights, "Stack height do not match at ip {} in func {}. \nExpected: {:?} Found {:?}\nHistory: {:?}", ip, self.func.name(), existing, stack_heights, history);
            return;
        } else {
            self.stack_heights[ip] = Some(*stack_heights);
        }
        let side_effect = self.func.chunk()[ip].meta(&self.prog.func_map);
        if side_effect.is_ret() {
            return;
        }

        // Add this element to the history
        let mut history = history.clone();
        let mut next = stack_heights.clone();
        next.stacks.sub(&side_effect.args());
        next.stacks.add(&side_effect.returns());
        next.ip = ip;
        history.push(next);

        for descendant in side_effect.descendants() {
            let new_ip = (ip as isize) + descendant;
            assert!((0..self.func.chunk().len() as isize).contains(&new_ip), "jump outside of chunk at ip {}", ip);
            self.validate_rec(new_ip as usize, &history);
        }
    }
}
