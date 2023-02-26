use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use crate::global_scalars::SymbolMappingValue;
use crate::specials::{ARR_SPECIAL_NAMES, ArrSpecial, SCL_SPECIAL_NAMES, SclSpecial};
use crate::symbolizer::Symbol;

// These wrappers help prevent mixing of ids between arrays and scalars

#[derive(PartialEq, Clone, Copy)]
pub struct GlobalArrayId {
    pub id: usize,
}
#[derive(PartialEq, Clone, Copy)]
pub struct GlobalScalarId {
    pub id: usize,
}

impl SymbolMappingValue for GlobalArrayId {
    fn create(sym: &Symbol, id: usize) -> Self {
        Self { id }
    }
}

impl SymbolMappingValue for GlobalScalarId {
    fn create(sym: &Symbol, id: usize) -> Self {
        Self { id }
    }
}

impl Debug for GlobalScalarId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl Debug for GlobalArrayId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
