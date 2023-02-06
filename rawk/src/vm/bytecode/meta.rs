use std::fmt::{Debug, Formatter};
use crate::stackt::StackT;
use crate::util::pad;
use crate::stack_counter::{StackCounter as SC};

impl Debug for Meta {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let args = pad(format!("[{:?}]", self.args), 20);
        let rets = self.returns.make_array();
        let ret = pad(format!("[{:?}]", rets), 40);
        write!(f, "args: {} push: {}", args, ret)
    }
}

pub struct Meta {
    // Stacks that arguments come from
    args: Vec<StackT>,
    // Stacks that are pushed to after the instruction
    returns: SC,
    is_ret: bool,
    descendant_offsets: Vec<isize>,
}

impl Meta {
    pub fn args(&self) -> &[StackT] {
        &self.args
    }
    pub fn new(args: Vec<StackT>, returns: SC) -> Self {
        Self { args, returns: returns, is_ret: false, descendant_offsets: vec![1] }
    }
    pub fn set_is_ret(mut self) -> Self {
        self.is_ret = true;
        self
    }
    pub fn jump(mut self, offsets: Vec<isize>) -> Self {
        self.descendant_offsets = offsets;
        self
    }
    pub fn returns(&self) -> &SC {
        &self.returns
    }
    pub fn is_ret(&self) -> bool {
        self.is_ret
    }
    pub fn descendants(&self) -> &[isize] {
        &self.descendant_offsets
    }
}