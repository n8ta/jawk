use crate::symbolizer::Symbol;
use std::collections::HashMap;


pub trait SymbolMappingValue {
    fn create(symbol: &Symbol, idx: usize) -> Self;
}

#[derive(Debug, PartialEq, Clone)]
pub struct SymbolMapping<T: SymbolMappingValue> {
    mapping: HashMap<Symbol, T>,
}

impl<T: SymbolMappingValue> SymbolMapping<T> {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }
    pub fn insert(&mut self, symbol: &Symbol) {
        if self.mapping.contains_key(&symbol) {
            return;
        } else {
            self.mapping.insert(symbol.clone(), T::create(symbol,self.mapping.len()))
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
