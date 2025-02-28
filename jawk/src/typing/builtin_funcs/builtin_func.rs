use crate::parser::{Arg, ArgT};
use crate::printable_error::PrintableError;
use crate::symbolizer::Symbol;
use crate::Symbolizer;
use std::fmt::{Display, Formatter};

pub const NUM_BUILTIN_VARIANTS: usize = 19;

#[derive(Debug, Clone, Copy)]
pub enum BuiltinFunc {
    Atan2,
    Close,
    Cos,
    Exp,
    // Gsub Handled separately from rest of the builtin type system since their out variables are complex
    // Sub ^ same
    Substr,
    Index,
    Int,
    Length,
    Log,
    Matches,
    Rand,
    Sin,
    Split,
    Sprintf,
    Sqrt,
    Srand,
    System,
    Tolower,
    Toupper, // Must stay as last (see builtin_factory.rs)
}

impl BuiltinFunc {
    pub fn args(&self, s: &mut Symbolizer) -> Vec<Arg> {
        match self {
            BuiltinFunc::Sin => vec![Arg::new_scl(s.get("sin-arg-0"))],
            BuiltinFunc::Cos => vec![Arg::new_scl(s.get("cos-arg-0"))],
            BuiltinFunc::Log => vec![Arg::new_scl(s.get("log-arg-0"))],
            BuiltinFunc::Sqrt => vec![Arg::new_scl(s.get("sqrt-arg-0"))],
            BuiltinFunc::Exp => vec![Arg::new_scl(s.get("exp-arg-0"))],
            BuiltinFunc::Int => vec![Arg::new_scl(s.get("int-arg-0"))],
            BuiltinFunc::Rand => vec![],
            BuiltinFunc::Srand => vec![Arg::new_scl(s.get("rand-arg-0"))],
            BuiltinFunc::Atan2 => vec![Arg::new_scl(s.get("atan2-arg-0")), Arg::new_scl(s.get("atan2-arg-1"))],
            BuiltinFunc::Length => vec![Arg::new_optional(s.get("length-arg-0"), ArgT::Scalar)],
            BuiltinFunc::Tolower => vec![Arg::new_scl(s.get("lower-arg-0"))],
            BuiltinFunc::Toupper => vec![Arg::new_scl(s.get("upper-arg-0"))],
            BuiltinFunc::Split => vec![Arg::new_scl(s.get("split-arg-0")), Arg::new_arr(s.get("split-arg-1")), Arg::new_optional(s.get("split-arg-2"), ArgT::Scalar)],
            BuiltinFunc::Substr => vec![Arg::new_scl(s.get("substr-arg-0")), Arg::new_scl(s.get("substr-arg-1")), Arg::new_optional(s.get("substr-arg-2"), ArgT::Scalar)],
            BuiltinFunc::Index => vec![Arg::new_scl(s.get("index-arg-0")), Arg::new_scl(s.get("index-arg-1"))],
            BuiltinFunc::Matches => todo!(),
            BuiltinFunc::Sprintf => todo!(),
            BuiltinFunc::Close => todo!(),
            BuiltinFunc::System => todo!(),
        }
    }
    pub fn names_as_symbols(symbolizer: &mut Symbolizer) -> [Symbol; NUM_BUILTIN_VARIANTS] {
        [
            symbolizer.get("Atan2"),
            symbolizer.get("Close"),
            symbolizer.get("Cos"),
            symbolizer.get("Exp"),
            symbolizer.get("Index"),
            symbolizer.get("Int"),
            symbolizer.get("Length"),
            symbolizer.get("Log"),
            symbolizer.get("Matches"),
            symbolizer.get("Rand"),
            symbolizer.get("Sin"),
            symbolizer.get("Split"),
            symbolizer.get("Sprintf"),
            symbolizer.get("Sqrt"),
            symbolizer.get("Srand"),
            symbolizer.get("Substr"),
            symbolizer.get("System"),
            symbolizer.get("Tolower"),
            symbolizer.get("Toupper"),
        ]
    }
    pub fn to_str(&self) -> &'static str {
        match self {
            BuiltinFunc::Atan2 => "Atan2",
            BuiltinFunc::Close => "Close",
            BuiltinFunc::Cos => "Cos",
            BuiltinFunc::Exp => "Exp",
            BuiltinFunc::Index => "Index",
            BuiltinFunc::Int => "Int",
            BuiltinFunc::Length => "Length",
            BuiltinFunc::Log => "Log",
            BuiltinFunc::Matches => "Matches",
            BuiltinFunc::Rand => "Rand",
            BuiltinFunc::Sin => "Sin",
            BuiltinFunc::Split => "Split",
            BuiltinFunc::Sprintf => "Sprintf",
            BuiltinFunc::Sqrt => "Sqrt",
            BuiltinFunc::Srand => "Srand",
            BuiltinFunc::Substr => "Substr",
            BuiltinFunc::System => "System",
            BuiltinFunc::Tolower => "Tolower",
            BuiltinFunc::Toupper => "Toupper",
        }
    }
    pub fn get(value: &str) -> Option<BuiltinFunc> {
        let res = match value {
            "atan2" => BuiltinFunc::Atan2,
            "close" => BuiltinFunc::Close,
            "cos" => BuiltinFunc::Cos,
            "exp" => BuiltinFunc::Exp,
            "index" => BuiltinFunc::Index,
            "int" => BuiltinFunc::Int,
            "length" => BuiltinFunc::Length,
            "log" => BuiltinFunc::Log,
            "match" => BuiltinFunc::Matches,
            "rand" => BuiltinFunc::Rand,
            "sin" => BuiltinFunc::Sin,
            "split" => BuiltinFunc::Split,
            "sprintf" => BuiltinFunc::Sprintf,
            "sqrt" => BuiltinFunc::Sqrt,
            "srand" => BuiltinFunc::Srand,
            "substr" => BuiltinFunc::Substr,
            "system" => BuiltinFunc::System,
            "tolower" => BuiltinFunc::Tolower,
            "toupper" => BuiltinFunc::Toupper,
            _ => return None,
        };
        Some(res)
    }
}

impl Display for BuiltinFunc {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl TryFrom<&str> for BuiltinFunc {
    type Error = PrintableError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match BuiltinFunc::get(value) {
            Some(r) => Ok(r),
            _ => Err(PrintableError::new(format!(
                "{} is not a builtin function",
                value
            ))),
        }
    }
}
