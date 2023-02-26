/*
Special vars are separated based on what part of the vm owns them.

Columns related
    r+w   FS
    r+w   RS
    r     FILENAME
    r     FNR
    r     NF
    r     NR

String Converter
    r+w   CONVFMT
    r+w   OFMT
    r+w   OFS
    r+w   ORS

Regex Cache
    r     RLENGTH
    r     RSTART

Arrays
    r+w   SUBSEP

TODO:
    ARGC
    ARGV
    ENVIRON

*/

use crate::awk_str::RcAwkStr;
use crate::vm::RuntimeScalar;

// All support getting
#[repr(u16)]
pub enum ColumnSpecials {
    // yes assignment
    RS = 0,
    FS = 1,
    // no assignment
    NR = 2,
    NF = 3,
    FNR = 4,
    FILENAME = 5,
}

// Supports getting and assigning
#[repr(u16)]
pub enum StringConverterSpecials {
    OFMT = 0,
    CONVFMT = 1,
}


pub const NUM_GSCALAR_SPECIALS: usize = 3;

// Supports getting and assigning
#[repr(u16)]
pub enum GlobalScalarSpecials {
    RLENGTH = 0,
    RSTART = 1,
    SUBSEP = 2,
}

impl GlobalScalarSpecials {
    pub fn initialize() -> Vec<RuntimeScalar> {
        let empty_strnum = RuntimeScalar::StrNum(RcAwkStr::new_bytes(vec![]));
        vec![empty_strnum.clone(),
             empty_strnum,
             RuntimeScalar::StrNum(RcAwkStr::new_bytes("".as_bytes().to_vec()))]
    }
}
