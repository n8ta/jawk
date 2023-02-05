use crate::parser::ScalarType;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum StackT {
    Num,
    Str,
    Var,
    Array,
}

const VARIANTS: &'static [StackT; 4] = &[StackT::Var,StackT::Str,StackT::Num, StackT::Array];

impl StackT {
    pub fn iter() -> &'static [StackT; 4] {
        &VARIANTS
    }
}

impl TryInto<ScalarType> for StackT {
    type Error = ();

    fn try_into(self) -> Result<ScalarType, Self::Error> {
        match self {
            StackT::Num => Ok(ScalarType::Num),
            StackT::Str => Ok(ScalarType::Str),
            StackT::Var => Ok(ScalarType::Var),
            StackT::Array => Err(())
        }
    }
}

impl Into<StackT> for ScalarType {
    fn into(self) -> StackT {
        match self {
            ScalarType::Str => StackT::Str,
            ScalarType::Num => StackT::Num,
            ScalarType::Var => StackT::Var,
        }
    }
}