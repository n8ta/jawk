use std::fmt::{Debug, Formatter};
use crate::global_scalars::SymbolMappingValue;

#[derive(PartialEq, Clone, Copy)]
pub struct GlobalArrayId {
    pub id: u16,
}
impl SymbolMappingValue  for GlobalArrayId {
    // TODO: handle u16max
    fn create(id: usize) -> Self { Self { id: id as u16 } }
}

#[derive(PartialEq, Clone, Copy)]
pub struct GlobalScalarId {
    pub id: u16,
}
impl SymbolMappingValue  for GlobalScalarId {
    // TODO: handle u16max
    fn create(id: usize) -> Self { Self { id: id as u16 } }
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