use crate::parser::{Arg, ArgT};
use crate::printable_error::PrintableError;
use crate::symbolizer::Symbol;
use crate::Symbolizer;
use std::fmt::{Display, Formatter};

pub const NUM_BUILTIN_VARIANTS: usize = 21;

#[derive(Debug, Clone, Copy)]
pub enum BuiltinFunc {
    Atan2,
    Close,
    Cos,
    Exp,
    Gsub,
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
    Sub,
    Substr,
    System,
    Tolower,
    Toupper, // Must stay as last (see builtin_factory.rs)
}

impl BuiltinFunc {
    pub fn args(&self, s: &mut Symbolizer) -> Vec<Arg> {
        match self {
            BuiltinFunc::Sin => vec![Arg::new(s.get("sin-arg-0"), ArgT::Scalar)],
            BuiltinFunc::Cos => vec![Arg::new(s.get("cos-arg-0"), ArgT::Scalar)],
            BuiltinFunc::Log => vec![Arg::new(s.get("log-arg-0"), ArgT::Scalar)],
            BuiltinFunc::Sqrt => vec![Arg::new(s.get("sqrt-arg-0"), ArgT::Scalar)],
            BuiltinFunc::Exp => vec![Arg::new(s.get("exp-arg-0"), ArgT::Scalar)],
            BuiltinFunc::Int => vec![Arg::new(s.get("int-arg-0"), ArgT::Scalar)],

            BuiltinFunc::Rand => todo!(),
            BuiltinFunc::Srand => todo!(),

            BuiltinFunc::Atan2 => todo!(),
            BuiltinFunc::Close => todo!(),
            BuiltinFunc::Gsub => todo!(),
            BuiltinFunc::Index => todo!(),
            BuiltinFunc::Length => todo!(),
            BuiltinFunc::Matches => todo!(),
            BuiltinFunc::Split => todo!(),
            BuiltinFunc::Sprintf => todo!(),
            BuiltinFunc::Sub => todo!(),
            BuiltinFunc::Substr => todo!(),
            BuiltinFunc::System => todo!(),
            BuiltinFunc::Tolower => vec![Arg::new(s.get("lower-arg-0"), ArgT::Scalar)],
            BuiltinFunc::Toupper => vec![Arg::new(s.get("upper-arg-0"), ArgT::Scalar)],
        }
    }
    pub fn names_as_symbols(symbolizer: &mut Symbolizer) -> [Symbol; NUM_BUILTIN_VARIANTS] {
        [
            symbolizer.get("Atan2"),
            symbolizer.get("Close"),
            symbolizer.get("Cos"),
            symbolizer.get("Exp"),
            symbolizer.get("Gsub"),
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
            symbolizer.get("Sub"),
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
            BuiltinFunc::Gsub => "Gsub",
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
            BuiltinFunc::Sub => "Sub",
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
            "gsub" => BuiltinFunc::Gsub,
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
            "sub" => BuiltinFunc::Sub,
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
