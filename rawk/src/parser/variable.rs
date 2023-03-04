use std::fmt::{Display, Formatter};
use crate::parser::SclSpecial;
use crate::symbolizer::Symbol;

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Variable {
    User(Symbol),
    Special(SclSpecial),
}

impl From<Symbol> for Variable {
    fn from(sym: Symbol) -> Self {
        if let Ok(special) = SclSpecial::try_from(sym.to_str()) {
            Variable::Special(special)
        } else {
            Variable::User(sym)
        }
    }
}
impl From<SclSpecial> for Variable {
    fn from(special: SclSpecial) -> Self {
        Variable::Special(special)
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Variable::User(sym) => write!(f, "{}", sym),
            Variable::Special(special) => write!(f, "{}", special),
        }
    }
}