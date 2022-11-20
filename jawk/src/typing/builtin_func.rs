use crate::PrintableError;

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
    Toupper,
}

impl BuiltinFunc {
    fn get(value: &str) -> Option<BuiltinFunc> {
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

    pub fn is_builtin(str: &str) -> bool {
        BuiltinFunc::get(str).is_some()
    }
}

impl TryFrom<&str> for BuiltinFunc {
    type Error = PrintableError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match BuiltinFunc::get(value) {
            Some(r) => Ok(r),
            _ => Err(PrintableError::new(format!("{} is not a builtin function", value))),
        }
    }
}