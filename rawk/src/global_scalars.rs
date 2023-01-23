use crate::symbolizer::Symbol;
use std::collections::HashMap;


pub trait SymbolMappingValue {
    fn create(idx: usize) -> Self;
}

#[derive(Debug, PartialEq, Clone)]
pub struct SymbolMapping<T: SymbolMappingValue> {
    // Offset all ids by `start_at` useful for global scalars
    // we want to reserve some space at the top for special vars.
    start_at: usize,
    mapping: HashMap<Symbol, T>,
}

impl<T: SymbolMappingValue> SymbolMapping<T> {
    pub fn new(start_at: usize) -> Self {
        Self {
            start_at,
            mapping: HashMap::new(),
        }
    }
    pub fn insert(&mut self, symbol: &Symbol) {
        if self.mapping.contains_key(&symbol) {
            return;
        } else {
            self.mapping.insert(symbol.clone(), T::create(self.mapping.len()+self.start_at))
        };
    }
    pub fn get(&self, symbol: &Symbol) -> Option<&T> {
        self.mapping.get(symbol)
    }

    pub fn mapping(&self) -> &HashMap<Symbol, T> {
        &self.mapping
    }

    pub fn contains_key(&self, symbol: &Symbol) -> bool {
        self.mapping.contains_key(symbol)
    }

    pub fn len(&self) -> usize {
        self.mapping.len()
    }

    pub fn all_symbols(&self) -> Vec<Symbol> {
        self.mapping.keys().cloned().collect()
    }
}
