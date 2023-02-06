use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use crate::global_scalars::SymbolMappingValue;

#[derive(PartialEq, Clone, Copy)]
pub struct GlobalArrayId {
    pub     id: usize,
}
impl SymbolMappingValue  for GlobalArrayId {
    fn create(id: usize) -> Self { Self { id } }
}

#[derive(PartialEq, Clone, Copy)]
pub struct GlobalScalarId {
    pub id: usize,
}
impl SymbolMappingValue  for GlobalScalarId {
    fn create(id: usize) -> Self { Self { id } }
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