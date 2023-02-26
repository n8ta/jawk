// Awk special variables are stored the at the front of the global scalar
// and global array storage in the vm

use crate::parser::ScalarType;

#[derive(Debug, Copy, Clone, PartialEq)]
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

pub const ARR_SPECIAL_NAMES: &[&'static str] = &[
    "ARGC",
    "ENVIRON",
];
pub const SCL_SPECIAL_NAMES: &[&'static str] = &[
    "FS",
    "RS",
    "FILENAME",
    "FNR",
    "NF",
    "NR",
    "CONVFMT",
    "OFMT",
    "OFS",
    "ORS",
    "RLENGTH",
    "RSTART",
    "SUBSEP",
    "ARGC",
];


impl SclSpecial {
    pub const fn variants() -> &'static [(SclSpecial, &'static str, ScalarType)] {
        return &[
            (SclSpecial::FS, "FS", ScalarType::Str),
            (SclSpecial::RS, "RS", ScalarType::Str),
            (SclSpecial::FILENAME, "FILENAME", ScalarType::Str),
            (SclSpecial::FNR, "FNR", ScalarType::Num),
            (SclSpecial::NF, "NF", ScalarType::Num),
            (SclSpecial::NR, "NR", ScalarType::Num),
            (SclSpecial::CONVFMT, "CONVFMT", ScalarType::Str),
            (SclSpecial::OFMT, "OFMT", ScalarType::Str),
            (SclSpecial::OFS, "OFS", ScalarType::Str),
            (SclSpecial::ORS, "ORS", ScalarType::Str),
            (SclSpecial::RLENGTH, "RLENGTH", ScalarType::Num),
            (SclSpecial::RSTART, "RSTART", ScalarType::Num),
            (SclSpecial::SUBSEP, "SUBSEP", ScalarType::Str),
            (SclSpecial::ARGC, "ARGC", ScalarType::Num),
        ];
    }
}

pub enum ArrSpecial {
    ARGV = 0,
    ENVIRON = 1,
}

impl ArrSpecial {
    pub fn variants() -> &'static [(ArrSpecial, &'static str)] {
        return &[
            (ArrSpecial::ARGV, "ARGV"),
            (ArrSpecial::ENVIRON, "ENVIRON"),
        ];
    }
}