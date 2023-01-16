use crate::global_scalars::SymbolMappingValue;

#[derive(Debug,PartialEq, Clone, Copy)]
pub struct GlobalArrayId {
    pub id: u16,
}
impl SymbolMappingValue  for GlobalArrayId {
    // TODO: handle u16max
    fn create(id: usize) -> Self { Self { id: id as u16 } }
}

#[derive(Debug,PartialEq, Clone, Copy)]
pub struct GlobalScalarId {
    pub id: u16,
}
impl SymbolMappingValue  for GlobalScalarId {
    // TODO: handle u16max
    fn create(id: usize) -> Self { Self { id: id as u16 } }
}