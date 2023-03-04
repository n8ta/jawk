// Awk special variables are stored the at the front of the global scalar
// and global array storage in the vm

use std::fmt::{Display, Formatter};
use crate::parser::ScalarType;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum SclSpecial {
    // Col specials
    FS = 0,
    RS = 1,
    FILENAME = 2,
    FNR = 3,
    NF = 4,
    NR = 5,

    // Str specials
    CONVFMT = 6,
    OFMT = 7,
    OFS = 8,
    ORS = 9,

    // Regex specials
    RLENGTH = 10,
    RSTART = 11,

    // Other specials
    SUBSEP = 12,
    ARGC = 13,
}

impl Display for SclSpecial {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", SCL_SPECIAL_MAP[*self as usize].0)
    }
}

impl TryFrom<&str> for SclSpecial {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if let Some((_, (_, special))) = SCL_SPECIAL_MAP
            .iter()
            .enumerate()
            .find(|(_idx, (name, special))| name == &value) {
            Ok(*special)
        } else {
            Err(())
        }
    }
}



type SclSpecialMapT = &'static [(&'static str, SclSpecial)];
type ArrSpecialMapT = &'static [(&'static str, ArrSpecial)];

const ARR_SPECIAL_MAP: ArrSpecialMapT = &[
    ("ARGV", ArrSpecial::ARGV),
    ("ENVIRON", ArrSpecial::ENVIRON),
];
const SCL_SPECIAL_MAP: SclSpecialMapT = &[
    ("FS", SclSpecial::FS),
    ("RS", SclSpecial::RS),
    ("FILENAME", SclSpecial::FILENAME),
    ("FNR", SclSpecial::FNR),
    ("NF", SclSpecial::NF),
    ("NR", SclSpecial::NR),
    ("CONVFMT", SclSpecial::CONVFMT),
    ("OFMT", SclSpecial::OFMT),
    ("OFS", SclSpecial::OFS),
    ("ORS", SclSpecial::ORS),
    ("RLENGTH", SclSpecial::RLENGTH),
    ("RSTART", SclSpecial::RSTART),
    ("SUBSEP", SclSpecial::SUBSEP),
    ("ARGC", SclSpecial::ARGC),
];


impl SclSpecial {
    pub const fn variants() -> SclSpecialMapT {
        &SCL_SPECIAL_MAP
    }
}

pub enum ArrSpecial {
    ARGV = 0,
    ENVIRON = 1,
}

impl ArrSpecial {
    pub fn variants() -> ArrSpecialMapT {
        &ARR_SPECIAL_MAP
    }
}