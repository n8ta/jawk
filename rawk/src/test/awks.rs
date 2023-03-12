use std::iter::Filter;
use std::slice::Iter;

#[derive(Copy, Clone)]
#[repr(usize)]
pub enum Awk {
    Goawk = 1,
    Gawk = 2,
    Mawk = 4,
    Onetrueawk = 8,
    Rawk = 16,
}

pub type AwkTuple = (&'static str, Awk);
const AWKS: &'static[AwkTuple]  = &[("goawk", Awk::Goawk), ("gawk", Awk::Gawk), ("mawk", Awk::Mawk), ("onetrueawk", Awk::Onetrueawk)];

impl Awk {
    pub fn without<'a>(flags: usize) -> Vec<AwkTuple> {
        AWKS.iter().filter(|(name, flag)| flags & (*flag as usize) != 0 ).cloned().collect()
    }
}

